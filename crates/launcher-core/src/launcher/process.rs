use super::{LaunchSpec, NativeScratch};
use crate::{
    auth::SecretString,
    core::{LauncherError as Error, Result},
    installer::Cancellation,
    profiles::ProfileId,
    storage::ResourceStore,
};
use serde::Serialize;
use std::{
    collections::VecDeque,
    fmt,
    process::Stdio,
    sync::{Arc, Mutex},
    time::Duration,
};
use tokio::{
    io::{AsyncBufReadExt, AsyncRead, BufReader},
    process::{Child, Command},
};

const LINE_LIMIT: usize = 4096;
const OUTPUT_LINES: usize = 128;

#[derive(Debug, Clone, Copy, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameExit {
    pub code: Option<i32>,
    pub success: bool,
    pub stopped: bool,
}

#[derive(Serialize)]
struct OutputLine {
    stream: &'static str,
    text: String,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunLog {
    schema_version: u32,
    exit: GameExit,
    output: VecDeque<OutputLine>,
}

#[derive(Serialize)]
#[serde(rename_all = "camelCase")]
struct RunMarker {
    schema_version: u32,
    java_pid: Option<u32>,
}

fn run_marker(profile: ProfileId) -> String {
    format!("profiles/{}/runs/active.json", profile.0)
}

pub(super) fn verify_no_pending_run(store: &ResourceStore, profile: ProfileId) -> Result<()> {
    if store.run_marker_exists(&run_marker(profile))? {
        return Err(Error::GameRecoveryRequired);
    }
    Ok(())
}

pub struct GameProcess {
    // Drop the child (kill-on-drop) before releasing the runtime and the leases.
    child: Child,
    runtime: tokio::runtime::Runtime,
    token: SecretString,
    store: Arc<ResourceStore>,
    profile: ProfileId,
    _scratch: Option<NativeScratch>,
}

impl fmt::Debug for GameProcess {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str("GameProcess([REDACTED])")
    }
}

impl GameProcess {
    pub(super) fn spawn(
        spec: LaunchSpec,
        token: SecretString,
        store: Arc<ResourceStore>,
        profile: ProfileId,
        scratch: Option<NativeScratch>,
    ) -> Result<Self> {
        let runtime = tokio::runtime::Builder::new_current_thread()
            .enable_all()
            .build()
            .map_err(|_| Error::GameSpawn)?;
        verify_no_pending_run(&store, profile)?;
        let marker = run_marker(profile);
        // Reserve before spawning so an unexpected launcher exit cannot permit a second game.
        store.document(
            &marker,
            &RunMarker {
                schema_version: 1,
                java_pid: None,
            },
        )?;
        let child = {
            let _entered = runtime.enter();
            let mut command = Command::new(spec.executable);
            command
                .args(spec.arguments)
                .current_dir(spec.working_directory)
                .stdin(Stdio::null())
                .stdout(Stdio::piped())
                .stderr(Stdio::piped())
                .kill_on_drop(true);
            #[cfg(windows)]
            command.creation_flags(0x0800_0000); // CREATE_NO_WINDOW
            for key in crate::runtime::JAVA_ENV_OVERRIDES {
                command.env_remove(key);
            }
            match command.spawn() {
                Ok(child) => child,
                Err(_) => {
                    store.remove_run_marker(&marker)?;
                    return Err(Error::GameSpawn);
                }
            }
        };
        // PID is diagnostic only. If publication fails the prior reservation still blocks reuse.
        let _ = store.document(
            &marker,
            &RunMarker {
                schema_version: 1,
                java_pid: child.id(),
            },
        );
        Ok(Self {
            child,
            runtime,
            token,
            store,
            profile,
            _scratch: scratch,
        })
    }

    /// Single owner of wait/kill. A cancellation ends only this child and reaps it.
    /// A failure to save diagnostic output must not turn a successful exit into a spawn failure.
    pub fn wait(mut self, cancel: &Cancellation) -> Result<CompletedGame> {
        let stdout = self.child.stdout.take().ok_or(Error::GameMonitor)?;
        let stderr = self.child.stderr.take().ok_or(Error::GameMonitor)?;
        let output = Mutex::new(VecDeque::new());
        let exit = self.runtime.block_on(async {
            let drain = async {
                tokio::join!(
                    collect(stdout, "stdout", self.token.expose_secret(), &output),
                    collect(stderr, "stderr", self.token.expose_secret(), &output)
                )
            };
            let monitor = async {
                let mut stop = tokio::time::interval(Duration::from_millis(75));
                loop {
                    tokio::select! {
                        biased;
                        status = self.child.wait() => {
                            let status = status.map_err(|_| Error::GameMonitor)?;
                            break Ok::<_, Error>(GameExit { code: status.code(), success: status.success(), stopped: false });
                        }
                        _ = stop.tick() => {
                            if cancel.check().is_err() {
                                self.child.kill().await.map_err(|_| Error::GameMonitor)?;
                                let status = self.child.wait().await.map_err(|_| Error::GameMonitor)?;
                                break Ok(GameExit { code: status.code(), success: status.success(), stopped: true });
                            }
                        }
                    }
                }
            };
            tokio::pin!(drain, monitor);
            let exit = tokio::select! {
                exit = &mut monitor => {
                    let _ = tokio::time::timeout(Duration::from_millis(500), &mut drain).await;
                    exit?
                }
                _ = &mut drain => monitor.await?,
            };
            Ok::<_, Error>(exit)
        })?;
        let log = RunLog {
            schema_version: 1,
            exit,
            output: output.into_inner().map_err(|_| Error::GameMonitor)?,
        };
        let saved = self
            .store
            .document(
                &format!("profiles/{}/logs/launcher-last-run.json", self.profile.0),
                &log,
            )
            .is_ok();
        // A monitor failure leaves the reservation in place because exit was not confirmed.
        let recovery_required = self
            .store
            .remove_run_marker(&run_marker(self.profile))
            .is_err();
        Ok(CompletedGame {
            exit,
            log_saved: saved,
            recovery_required,
        })
    }
}

#[derive(Debug)]
pub struct CompletedGame {
    pub exit: GameExit,
    pub log_saved: bool,
    pub recovery_required: bool,
}

async fn collect(
    pipe: impl AsyncRead + Unpin,
    stream: &'static str,
    token: &str,
    output: &Mutex<VecDeque<OutputLine>>,
) {
    let mut pipe = BufReader::new(pipe);
    let mut line = Vec::with_capacity(LINE_LIMIT);
    let mut oversized = false;
    while let Ok(bytes) = pipe.fill_buf().await {
        if bytes.is_empty() {
            if !line.is_empty() || oversized {
                record(&line, oversized, stream, token, output);
            }
            break;
        }
        let end = bytes.iter().position(|&b| b == b'\n');
        let length = end.unwrap_or(bytes.len());
        if !oversized && line.len() + length <= LINE_LIMIT {
            line.extend_from_slice(&bytes[..length]);
        } else {
            oversized = true;
        }
        pipe.consume(length + usize::from(end.is_some()));
        if end.is_some() {
            record(&line, oversized, stream, token, output);
            line.clear();
            oversized = false;
        }
    }
}

fn record(
    line: &[u8],
    oversized: bool,
    stream: &'static str,
    token: &str,
    output: &Mutex<VecDeque<OutputLine>>,
) {
    let text = if oversized {
        "[длинная строка пропущена]".into()
    } else {
        let line = String::from_utf8_lossy(line);
        let line = line.trim_end_matches('\r');
        if !token.is_empty() && line.contains(token) {
            // Drop the whole line, including neighboring fragments of sensitive output.
            "[строка с токеном скрыта]".into()
        } else {
            line.chars()
                .filter(|c| !c.is_control() || *c == '\t')
                .take(LINE_LIMIT / 2)
                .collect()
        }
    };
    if let Ok(mut output) = output.lock() {
        if output.len() == OUTPUT_LINES {
            output.pop_front();
        }
        output.push_back(OutputLine { stream, text });
    }
}

#[cfg(test)]
mod tests;

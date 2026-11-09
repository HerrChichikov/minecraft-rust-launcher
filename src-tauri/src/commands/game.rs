use super::{auth::AuthState, snapshot::ApiError, workspace::execute};
use launcher_core::{
    core::{EventSink, LauncherError as Error, OperationEvent, Platform, Result},
    installer::{Cancellation, InstallEngine, metadata::RuleContext},
    launcher::{GameExit, GameRunId, PreparedLaunch},
    profiles::ProfileId,
    storage::RootId,
};
use serde::{Deserialize, Serialize};
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
enum Stage {
    Preparing,
    Launching,
    Running,
    Stopping,
    Cancelled,
    Finished,
    Failed,
}
impl Stage {
    fn active(self) -> bool {
        matches!(
            self,
            Self::Preparing | Self::Launching | Self::Running | Self::Stopping
        )
    }
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct GameStatus {
    schema_version: u32,
    run_id: GameRunId,
    sequence: u64,
    profile_id: ProfileId,
    profile_name: String,
    stage: Stage,
    exit: Option<GameExit>,
    error_code: Option<&'static str>,
    log_saved: Option<bool>,
}

#[derive(Debug)]
struct Job {
    status: GameStatus,
    cancel: Cancellation,
}

#[derive(Debug, Default)]
struct Games {
    job: Option<Job>,
    exiting: bool,
}

#[derive(Debug, Default)]
pub(crate) struct GameState(Mutex<Games>);
impl GameState {
    pub(crate) fn active(&self) -> bool {
        self.0.lock().map_or(true, |games| games.active())
    }

    pub(crate) fn can_exit(&self) -> bool {
        self.0.lock().is_ok_and(|mut games| {
            if games.active() {
                return false;
            }
            games.exiting = true;
            true
        })
    }
}
impl Games {
    fn active(&self) -> bool {
        self.job
            .as_ref()
            .is_some_and(|job| job.status.stage.active())
    }
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StartRequest {
    root_id: RootId,
    profile_id: ProfileId,
    profile_revision: u32,
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct StopRequest {
    run_id: GameRunId,
}

struct IgnoreInstallProgress;
impl EventSink for IgnoreInstallProgress {
    fn publish(&self, _: OperationEvent) {}
}

fn publish<R: Runtime>(app: &AppHandle<R>, status: &GameStatus) {
    let _ = app.emit_to("main", "launcher://game", status);
}

fn transition<R: Runtime>(
    app: &AppHandle<R>,
    run_id: GameRunId,
    update: impl FnOnce(&mut GameStatus),
) {
    let state = app.state::<GameState>();
    if let Ok(mut guard) = state.0.lock()
        && let Some(job) = guard.job.as_mut().filter(|job| job.status.run_id == run_id)
    {
        update(&mut job.status);
        job.status.sequence += 1;
        publish(app, &job.status);
    }
}

#[tauri::command]
pub fn get_game<R: Runtime>(
    app: AppHandle<R>,
) -> std::result::Result<Option<GameStatus>, ApiError> {
    app.state::<GameState>()
        .0
        .lock()
        .map(|games| games.job.as_ref().map(|job| job.status.clone()))
        .map_err(|_| Error::GameMonitor.into())
}

#[tauri::command]
pub async fn start_game<R: Runtime>(
    app: AppHandle<R>,
    request: StartRequest,
) -> std::result::Result<GameStatus, ApiError> {
    if app.state::<GameState>().active() {
        return Err(Error::GameBusy.into());
    }
    let (auth_revision, ely_by) = app.state::<AuthState>().launch_session()?;
    let (engine, profile, policy) = execute(app.clone(), move |workspace| {
        let snapshot = workspace.snapshot()?;
        if snapshot.root_id != request.root_id || snapshot.active_profile_id != request.profile_id {
            return Err(Error::RevisionConflict);
        }
        let profile = snapshot
            .profiles
            .into_iter()
            .find(|p| p.id == request.profile_id && p.revision == request.profile_revision)
            .ok_or(Error::RevisionConflict)?;
        let store = workspace.installation_store(request.root_id, profile.id)?;
        let engine = InstallEngine::new(store, profile.id, Cancellation::default())?;
        Ok((engine, profile, snapshot.policy))
    })
    .await?;
    let cancel = engine.cancellation();
    let run_id = GameRunId::generate();
    let status = {
        let state = app.state::<GameState>();
        let mut guard = state.0.lock().map_err(|_| Error::GameMonitor)?;
        if guard.active() || guard.exiting {
            return Err(Error::GameBusy.into());
        }
        let status = GameStatus {
            schema_version: 1,
            run_id,
            sequence: guard.job.as_ref().map_or(1, |job| job.status.sequence + 1),
            profile_id: profile.id,
            profile_name: profile.name.clone(),
            stage: Stage::Preparing,
            exit: None,
            error_code: None,
            log_saved: None,
        };
        guard.job = Some(Job {
            status: status.clone(),
            cancel: cancel.clone(),
        });
        publish(&app, &status);
        status
    };

    tauri::async_runtime::spawn_blocking(move || {
        let run = || -> Result<_> {
            let platform = Platform::current_host()?;
            let context =
                RuleContext::new(platform, sysinfo::System::os_version().unwrap_or_default());
            let prepared = PreparedLaunch::prepare(
                &engine,
                profile,
                policy,
                context,
                &IgnoreInstallProgress,
                ely_by,
            )?;
            drop(engine);
            cancel.check()?;
            transition(&app, run_id, |status| {
                if status.stage != Stage::Stopping {
                    status.stage = Stage::Launching;
                }
            });
            let game = app
                .state::<AuthState>()
                .with_launch_identity(auth_revision, |identity| {
                    prepared.spawn(identity, &cancel)
                })?;
            transition(&app, run_id, |status| {
                if status.stage != Stage::Stopping {
                    status.stage = Stage::Running;
                }
            });
            game.wait(&cancel)
        };
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(run))
            .unwrap_or(Err(Error::GameMonitor));
        // The worker has reaped the process and released leases before publishing completion.
        transition(&app, run_id, |status| match outcome {
            Ok(completed) => {
                let exit = completed.exit;
                status.stage = if exit.success || exit.stopped {
                    Stage::Finished
                } else {
                    Stage::Failed
                };
                status.exit = Some(exit);
                status.log_saved = Some(completed.log_saved);
                if completed.recovery_required {
                    status.error_code = Some("game_recovery_required");
                } else if status.stage == Stage::Failed {
                    status.error_code = Some("game_exited");
                }
            }
            Err(Error::Cancelled) => status.stage = Stage::Cancelled,
            Err(error) => {
                status.stage = Stage::Failed;
                status.error_code = Some(ApiError::from(error).code());
            }
        });
    });
    Ok(status)
}

#[tauri::command]
pub fn stop_game<R: Runtime>(
    app: AppHandle<R>,
    request: StopRequest,
) -> std::result::Result<GameStatus, ApiError> {
    let state = app.state::<GameState>();
    let mut guard = state.0.lock().map_err(|_| Error::GameMonitor)?;
    let job = guard
        .job
        .as_mut()
        .filter(|job| job.status.run_id == request.run_id)
        .ok_or(Error::InvalidInput {
            field: "game run ID",
        })?;
    if job.status.stage.active() && job.status.stage != Stage::Stopping {
        job.cancel.cancel();
        job.status.stage = Stage::Stopping;
        job.status.sequence += 1;
        publish(&app, &job.status);
    }
    Ok(job.status.clone())
}

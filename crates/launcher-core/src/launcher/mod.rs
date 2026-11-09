//! Verified installation, metadata-derived Java arguments and owned game processes.

mod arguments;
mod process;

pub use process::{CompletedGame, GameExit, GameProcess};

use crate::{
    auth::{LaunchIdentity, agent},
    core::{EventSink, LauncherError as Error, Result},
    installer::{Cancellation, InstallEngine, PreparedInstallation, metadata},
    profiles::{Profile, ProfileId, SettingsPolicy, validation},
    storage::ResourceStore,
};
use std::{ffi::OsString, fmt, path::PathBuf, sync::Arc};

const CONSOLE_CONFIG: &[u8] = include_bytes!("console-log4j.xml");

/// Backend-only argv. The game token must never enter debug output or IPC.
pub struct LaunchSpec {
    pub executable: PathBuf,
    pub arguments: Vec<OsString>,
    pub working_directory: PathBuf,
}
impl fmt::Debug for LaunchSpec {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("LaunchSpec([REDACTED])")
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, serde::Serialize, serde::Deserialize)]
#[serde(transparent)]
pub struct GameRunId(uuid::Uuid);
impl GameRunId {
    pub fn generate() -> Self {
        Self(uuid::Uuid::new_v4())
    }
}

#[derive(Debug)]
pub struct PreparedLaunch {
    store: Arc<ResourceStore>,
    scratch: NativeScratch,
    plan: metadata::VersionPlan,
    installed: PreparedInstallation,
    context: metadata::RuleContext,
    profile: Profile,
    agent: Option<agent::VerifiedAgent>,
    working_directory: PathBuf,
    natives_directory: PathBuf,
    logging_config: PathBuf,
}

#[derive(Debug)]
struct NativeScratch {
    store: Arc<ResourceStore>,
    path: String,
}

impl NativeScratch {
    fn create(store: &Arc<ResourceStore>, profile: ProfileId) -> Result<Self> {
        let path = format!("profiles/{}/runs/{}", profile.0, uuid::Uuid::new_v4());
        store.create_run_directory(&path)?;
        Ok(Self {
            store: Arc::clone(store),
            path,
        })
    }
}

impl Drop for NativeScratch {
    fn drop(&mut self) {
        // This random, launcher-owned directory has never held saves or settings.
        let _ = self.store.remove_run_directory(&self.path);
    }
}

impl PreparedLaunch {
    /// Recheck/repair all inputs with the installer, then pin the profile lease through exit.
    pub fn prepare(
        engine: &InstallEngine,
        profile: Profile,
        policy: SettingsPolicy,
        context: metadata::RuleContext,
        events: &dyn EventSink,
        ely_by: bool,
    ) -> Result<Self> {
        validation::settings(&profile.settings, policy, true)?;
        if ["-XX:+UseG1GC", "-XX:+UseZGC", "-XX:+UseParallelGC"]
            .iter()
            .filter(|gc| {
                profile
                    .settings
                    .extra_jvm_args
                    .iter()
                    .any(|arg| arg == **gc)
            })
            .count()
            > 1
        {
            return Err(Error::InvalidInput {
                field: "JVM garbage collector",
            });
        }
        let store = engine.resource_store();
        process::verify_no_pending_run(&store, profile.id)?;
        let plan = engine.plan(&profile, &context)?;
        let installed =
            engine.prepare(&plan, context.platform, events, &profile.settings.runtime)?;
        let cancel = engine.cancellation();
        let agent = if ely_by {
            Some(agent::ensure(Arc::clone(&store), &cancel)?)
        } else {
            None
        };
        let profile_path = format!("profiles/{}", profile.id.0);
        let game = format!("{profile_path}/game");
        // The modern native JARs are on the classpath. LWJGL extracts their contents here.
        store.directory(&game)?;
        let scratch = NativeScratch::create(&store, profile.id)?;
        let working_directory = store.path(&game)?;
        let natives_directory = store.path(&scratch.path)?;
        // Mojang's file appender bypasses our token filtering. Use a console-only config.
        let logging_config =
            store.install_builtin("launch-console.xml", CONSOLE_CONFIG, &cancel)?;
        Ok(Self {
            store,
            scratch,
            plan,
            installed,
            context,
            profile,
            agent,
            working_directory,
            natives_directory,
            logging_config,
        })
    }

    /// Call with a freshly validated backend identity. Neither argv nor tokens cross IPC.
    pub fn spawn(self, identity: LaunchIdentity, cancel: &Cancellation) -> Result<GameProcess> {
        if identity.requires_ely_by_agent != self.agent.is_some()
            || (!identity.requires_ely_by_agent
                && !cfg!(all(debug_assertions, feature = "dev-auth")))
        {
            return Err(Error::AuthUnavailable);
        }
        let mut argv = arguments::build(&self, &identity)?;
        if let Some(agent) = &self.agent {
            let agent_args = agent.jvm_arguments(cancel)?;
            argv.splice(0..0, agent_args);
        }
        cancel.check()?;
        GameProcess::spawn(
            LaunchSpec {
                executable: self.installed.java_executable.into(),
                arguments: argv,
                working_directory: self.working_directory,
            },
            identity.access_token,
            self.store,
            self.profile.id,
            Some(self.scratch),
        )
    }
}

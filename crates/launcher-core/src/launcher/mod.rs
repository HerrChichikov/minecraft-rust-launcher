//! Process boundary contracts. Phase 1 never constructs or executes a game command.

use std::{ffi::OsString, fmt, future::Future, path::PathBuf};

use uuid::Uuid;

use crate::core::Result;

/// Implementations must validate paths/options and launch directly without a shell.
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

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct GameRunId(pub Uuid);

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct GameExit {
    pub code: Option<i32>,
    pub success: bool,
}

/// Supervisor owns child handles and validates run ownership, never accepting arbitrary PIDs.
pub trait ProcessSupervisor: Send + Sync {
    fn spawn(&self, specification: LaunchSpec) -> Result<GameRunId>;
    fn wait(&self, run: GameRunId) -> impl Future<Output = Result<GameExit>> + Send;
    fn request_stop(&self, run: GameRunId) -> impl Future<Output = Result<()>> + Send;
}

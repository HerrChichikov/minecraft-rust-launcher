use thiserror::Error;

pub type Result<T> = std::result::Result<T, LauncherError>;

/// Domain errors are mapped to sanitized IPC errors by the desktop adapter.
#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("the requested operation is unavailable in this phase")]
    FeatureUnavailable,
    #[error("invalid or unsupported upstream metadata")]
    Metadata,
    #[error("download failed")]
    Network,
    #[error("download integrity check failed")]
    Integrity,
    #[error("Java runtime is unavailable or incompatible")]
    RuntimeIncompatible,
    #[error("invalid {field}")]
    InvalidInput { field: &'static str },
    #[error("the profile is already in use")]
    ProfileBusy,
    #[error("the storage is already in use")]
    StorageBusy,
    #[error("the document is corrupt")]
    CorruptDocument,
    #[error("the document schema is unsupported")]
    UnsupportedSchema,
    #[error("the settings have changed; reload before saving")]
    RevisionConflict,
    #[error("the selected directory is not a launcher root")]
    UnmanagedDirectory,
    #[error("an unsafe filesystem path was rejected")]
    UnsafePath,
    #[error("the write was published but directory synchronization failed")]
    Durability(#[source] std::io::Error),
    #[error("the operation was cancelled")]
    Cancelled,
    #[error("this platform is unsupported")]
    UnsupportedPlatform,
    #[error("authentication is required")]
    AuthenticationRequired,
    #[error("the selected authentication provider is unavailable")]
    AuthUnavailable,
    #[error("authentication credentials were rejected")]
    AuthRejected,
    #[error("authentication service is unreachable")]
    AuthNetwork,
    #[error("invalid authentication service response")]
    AuthProtocol,
    #[error("the authentication challenge expired")]
    AuthChallengeExpired,
    #[error("authentication is already in progress")]
    AuthBusy,
    #[error("the authentication flow is unsupported")]
    UnsupportedAuthFlow,
    #[error("secure credential storage is unavailable")]
    CredentialStoreUnavailable,
    #[error("no saved authentication session exists")]
    SavedSessionMissing,
    #[error("filesystem operation failed")]
    Storage(#[source] std::io::Error),
}

impl From<std::io::Error> for LauncherError {
    fn from(error: std::io::Error) -> Self {
        Self::Storage(error)
    }
}

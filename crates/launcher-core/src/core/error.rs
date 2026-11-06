use thiserror::Error;

pub type Result<T> = std::result::Result<T, LauncherError>;

/// Domain errors are mapped to sanitized IPC errors by the desktop adapter.
#[derive(Debug, Error)]
pub enum LauncherError {
    #[error("the requested operation is unavailable in this phase")]
    FeatureUnavailable,
    #[error("invalid {field}")]
    InvalidInput { field: &'static str },
    #[error("the profile is already in use")]
    ProfileBusy,
    #[error("the operation was cancelled")]
    Cancelled,
    #[error("this platform is unsupported")]
    UnsupportedPlatform,
    #[error("authentication is required")]
    AuthenticationRequired,
    #[error("secure credential storage is unavailable")]
    CredentialStoreUnavailable,
    #[error("filesystem operation failed")]
    Storage(#[source] std::io::Error),
}

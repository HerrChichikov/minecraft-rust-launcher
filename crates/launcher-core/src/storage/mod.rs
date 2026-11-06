//! Platform storage adapters are injected into the core rather than resolved from Tauri here.

use std::path::PathBuf;

use crate::{auth::SecretString, core::Result};

#[derive(Debug, Clone)]
pub struct AppPaths {
    pub data: PathBuf,
    pub configuration: PathBuf,
    pub cache: PathBuf,
    pub logs: PathBuf,
}

/// OS-backed credentials only; an unavailable store must not fall back to a plaintext file.
pub trait CredentialStore: Send + Sync {
    fn read(&self, reference: &str) -> Result<Option<SecretString>>;
    fn write(&self, reference: &str, secret: &SecretString) -> Result<()>;
    fn delete(&self, reference: &str) -> Result<()>;
}

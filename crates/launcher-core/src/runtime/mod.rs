//! Managed/custom Java contracts. No executable is downloaded or launched in Phase 1.

use std::{future::Future, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::{Platform, Result};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case", deny_unknown_fields)]
pub enum RuntimeSelection {
    Managed,
    Custom { executable: PathBuf },
}

#[derive(Debug, Clone)]
pub struct JavaRequirement {
    pub component: String,
    pub major_version: u32,
}

#[derive(Debug, Clone)]
pub struct VerifiedRuntime {
    pub executable: PathBuf,
    pub major_version: u32,
    pub platform: Platform,
    pub manifest_sha1: Option<String>,
}

/// Implementations verify metadata, files, architecture, executable permissions, and probe output.
pub trait RuntimeManager: Send + Sync {
    fn ensure(
        &self,
        requirement: &JavaRequirement,
        platform: Platform,
    ) -> impl Future<Output = Result<VerifiedRuntime>> + Send;

    fn probe(
        &self,
        executable: &std::path::Path,
    ) -> impl Future<Output = Result<VerifiedRuntime>> + Send;
}

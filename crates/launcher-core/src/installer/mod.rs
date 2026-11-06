//! Metadata-derived installation boundary; downloading and repair belong to Phase 3.

use std::{future::Future, path::PathBuf};

use crate::{
    core::{EventSink, Platform, Result},
    profiles::{Profile, ProfileId},
};

#[derive(Debug, Clone, Copy)]
pub enum VerificationMode {
    Quick,
    Full,
}

#[derive(Debug, Clone)]
pub struct ManagedResource {
    pub source_url: String,
    pub relative_path: PathBuf,
    pub expected_sha1: String,
    pub expected_size: u64,
}

/// Metadata descriptors are untrusted until the installer validates URL/path/hash policies.
#[derive(Debug, Clone)]
pub struct InstallPlan {
    pub profile_id: ProfileId,
    pub version_id: String,
    pub metadata_sha1: String,
    pub platform: Platform,
    pub resources: Vec<ManagedResource>,
}

#[derive(Debug, Clone)]
pub struct InstalledVersion {
    pub profile_id: ProfileId,
    pub version_id: String,
    pub metadata_sha1: String,
    pub classpath: Vec<PathBuf>,
    pub assets_root: PathBuf,
    pub asset_index_id: String,
}

/// Future implementations own cancellation by operation ID; callers never receive partial success.
pub trait Installer: Send + Sync {
    fn plan(
        &self,
        profile: &Profile,
        platform: Platform,
    ) -> impl Future<Output = Result<InstallPlan>> + Send;

    fn prepare(
        &self,
        plan: InstallPlan,
        mode: VerificationMode,
        events: &dyn EventSink,
    ) -> impl Future<Output = Result<InstalledVersion>> + Send;
}

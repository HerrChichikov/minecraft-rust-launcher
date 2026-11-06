//! Profile model and persistence boundary. Disk persistence is implemented in Phase 2.

use std::future::Future;

use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{core::Result, runtime::RuntimeSelection};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(transparent)]
pub struct ProfileId(pub Uuid);

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LaunchSettings {
    pub min_ram_mib: u32,
    pub max_ram_mib: u32,
    pub runtime: RuntimeSelection,
    pub extra_jvm_args: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct Profile {
    pub schema_version: u32,
    pub id: ProfileId,
    pub name: String,
    pub minecraft_version: String,
    pub settings: LaunchSettings,
}

/// Implementations validate settings and derive directories from the backend-owned ID.
pub trait ProfileRepository: Send + Sync {
    fn load(&self, id: ProfileId) -> impl Future<Output = Result<Profile>> + Send;
    fn create(&self, name: &str) -> impl Future<Output = Result<Profile>> + Send;
    fn save(&self, profile: &Profile) -> impl Future<Output = Result<()>> + Send;
    fn list(&self) -> impl Future<Output = Result<Vec<Profile>>> + Send;
}

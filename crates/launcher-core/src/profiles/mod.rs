//! Profile documents. Filesystem operations run on the desktop blocking worker.

pub(crate) mod validation;
pub use validation::SettingsPolicy;

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
    pub revision: u32,
    pub id: ProfileId,
    pub name: String,
    pub minecraft_version: String,
    pub settings: LaunchSettings,
}

/// Local repository operations must be called from a blocking worker.
pub trait ProfileRepository: Send + Sync {
    fn load(&self, id: ProfileId) -> Result<Profile>;
    fn create(&self, name: &str) -> Result<Profile>;
    fn save(&self, profile: &Profile) -> Result<()>;
    fn list(&self) -> Result<Vec<Profile>>;
}

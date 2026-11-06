//! Rebuildable verification hints. These records alone never prove a file is verified.
use super::{WorkspaceService, filesystem as fs};
use crate::core::{LauncherError as Error, Result};
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerificationRecord {
    pub expected_sha1: String,
    pub expected_size: u64,
    pub observed_size: u64,
    pub modified_unix_ns: u128,
    pub verified_unix_ms: u64,
    pub file_identity: Option<String>,
}
#[derive(Debug, Serialize, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct VerificationIndex {
    pub schema_version: u32,
    pub root_id: Uuid,
    pub records: BTreeMap<String, VerificationRecord>,
}
#[derive(Debug, PartialEq, Eq)]
pub enum IndexStatus {
    Loaded,
    Created,
    Rebuilt,
}
#[derive(Debug)]
pub struct IndexLoad {
    pub index: VerificationIndex,
    pub status: IndexStatus,
}

impl VerificationIndex {
    fn validate(&self, root_id: Uuid) -> Result<()> {
        if self.schema_version != 1 {
            return Err(Error::UnsupportedSchema);
        }
        if self.root_id != root_id || self.records.len() > 4096 {
            return Err(Error::CorruptDocument);
        }
        for (path, record) in &self.records {
            fs::relative(path)?;
            if !path.starts_with("shared/")
                || record.expected_sha1.len() != 40
                || !record.expected_sha1.bytes().all(|b| b.is_ascii_hexdigit())
                || record.expected_size != record.observed_size
                || record
                    .file_identity
                    .as_ref()
                    .is_some_and(|s| s.len() > 256 || s.chars().any(char::is_control))
            {
                return Err(Error::CorruptDocument);
            }
        }
        Ok(())
    }
}

impl WorkspaceService {
    pub fn load_verification_index(&self) -> Result<IndexLoad> {
        let dir = self.root.state()?;
        let loaded = fs::read::<VerificationIndex>(&dir, "verification-index.json");
        let corrupt = match loaded {
            Ok(Some(index)) => match index.validate(self.root.id) {
                Ok(()) => {
                    return Ok(IndexLoad {
                        index,
                        status: IndexStatus::Loaded,
                    });
                }
                Err(Error::UnsupportedSchema) => return Err(Error::UnsupportedSchema),
                Err(_) => true,
            },
            Ok(None) => false,
            Err(Error::CorruptDocument) => true,
            Err(e) => return Err(e),
        };
        if corrupt {
            dir.rename(
                "verification-index.json",
                &dir,
                format!("verification-index.invalid-{}.json", Uuid::new_v4()),
            )?;
            fs::sync(&dir)?;
        }
        let index = VerificationIndex {
            schema_version: 1,
            root_id: self.root.id,
            records: BTreeMap::new(),
        };
        fs::write(&dir, "verification-index.json", &index)?;
        Ok(IndexLoad {
            index,
            status: if corrupt {
                IndexStatus::Rebuilt
            } else {
                IndexStatus::Created
            },
        })
    }
    pub fn save_verification_index(&self, index: &VerificationIndex) -> Result<()> {
        index.validate(self.root.id)?;
        // A newer existing schema may never be overwritten by an older writer.
        fs::read::<VerificationIndex>(&self.root.state()?, "verification-index.json")?;
        fs::write(&self.root.state()?, "verification-index.json", index)
    }
}

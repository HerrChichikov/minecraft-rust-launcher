//! Managed/custom Java contracts. No executable is downloaded or launched in Phase 1.

use std::{future::Future, path::PathBuf};

use serde::{Deserialize, Serialize};

use crate::core::{Platform, Result};

pub(crate) const JAVA_ENV_OVERRIDES: &[&str] = &[
    "JAVA_TOOL_OPTIONS",
    "_JAVA_OPTIONS",
    "JDK_JAVA_OPTIONS",
    "CLASSPATH",
    "LD_PRELOAD",
    "LD_LIBRARY_PATH",
    "LD_AUDIT",
    "DYLD_INSERT_LIBRARIES",
    "DYLD_LIBRARY_PATH",
    "DYLD_FRAMEWORK_PATH",
];

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

/// Filesystem capabilities retain canonical/verbatim paths. The Windows HotSpot
/// launcher needs a normal Win32 executable path when deriving its jimage location.
pub fn execution_path(path: &std::path::Path) -> Result<PathBuf> {
    let text = path
        .to_str()
        .ok_or(crate::core::LauncherError::UnsafePath)?;
    #[cfg(windows)]
    {
        if let Some(unc) = text.strip_prefix("\\\\?\\UNC\\") {
            return Ok(PathBuf::from(format!("\\\\{unc}")));
        }
        if let Some(drive) = text.strip_prefix("\\\\?\\") {
            if drive.as_bytes().get(1) == Some(&b':') {
                return Ok(PathBuf::from(drive));
            }
            return Err(crate::core::LauncherError::UnsafePath);
        }
    }
    let _ = text;
    Ok(path.to_path_buf())
}

pub fn java_architecture(value: &str) -> Result<crate::core::Architecture> {
    match value {
        "amd64" | "x86_64" => Ok(crate::core::Architecture::X86_64),
        "aarch64" | "arm64" => Ok(crate::core::Architecture::Aarch64),
        _ => Err(crate::core::LauncherError::RuntimeIncompatible),
    }
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn jvm_architecture_aliases_are_normalized() -> Result<()> {
        assert_eq!(java_architecture("amd64")?, java_architecture("x86_64")?);
        assert_eq!(java_architecture("arm64")?, java_architecture("aarch64")?);
        assert!(java_architecture("x86").is_err());
        Ok(())
    }
    #[cfg(windows)]
    #[test]
    fn executable_path_avoids_hotspot_verbatim_prefix() -> Result<()> {
        assert_eq!(
            execution_path(std::path::Path::new(r"\\?\C:\Launcher\java.exe"))?,
            PathBuf::from(r"C:\Launcher\java.exe")
        );
        assert_eq!(
            execution_path(std::path::Path::new(r"\\?\UNC\server\share\java.exe"))?,
            PathBuf::from(r"\\server\share\java.exe")
        );
        Ok(())
    }
}

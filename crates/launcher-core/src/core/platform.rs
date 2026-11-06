use serde::Serialize;

use super::{LauncherError, Result};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum OperatingSystem {
    Windows,
    Linux,
    Macos,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Architecture {
    X86_64,
    Aarch64,
}

/// The launcher binary's target. Native host/JVM detection belongs to RuntimeManager.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Platform {
    pub os: OperatingSystem,
    pub architecture: Architecture,
}

impl Platform {
    pub fn current_binary() -> Result<Self> {
        let os = match std::env::consts::OS {
            "windows" => OperatingSystem::Windows,
            "linux" => OperatingSystem::Linux,
            "macos" => OperatingSystem::Macos,
            _ => return Err(LauncherError::UnsupportedPlatform),
        };
        let architecture = match std::env::consts::ARCH {
            "x86_64" => Architecture::X86_64,
            "aarch64" => Architecture::Aarch64,
            _ => return Err(LauncherError::UnsupportedPlatform),
        };
        match (os, architecture) {
            (OperatingSystem::Windows | OperatingSystem::Linux, Architecture::X86_64)
            | (OperatingSystem::Macos, _) => Ok(Self { os, architecture }),
            _ => Err(LauncherError::UnsupportedPlatform),
        }
    }
}

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
    /// JVM platform defaults to the native host, including an x64 launcher under Rosetta.
    pub fn current_host() -> Result<Self> {
        let platform = Self::current_binary()?;
        #[cfg(target_os = "macos")]
        if platform.architecture == Architecture::X86_64 {
            let output = std::process::Command::new("/usr/sbin/sysctl")
                .args(["-in", "sysctl.proc_translated"])
                .output()
                .map_err(|_| LauncherError::UnsupportedPlatform)?;
            if output.status.success() && String::from_utf8_lossy(&output.stdout).trim() == "1" {
                return Ok(Self {
                    architecture: Architecture::Aarch64,
                    ..platform
                });
            }
        }
        Ok(platform)
    }

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

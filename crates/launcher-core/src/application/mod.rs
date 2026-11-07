//! Read-only application bootstrap. Future workflows will be added here.

use crate::core::{Platform, Result};

pub const TARGET_MINECRAFT_VERSION: &str = "1.21.1";

/// A truthful inventory of implemented user features, independent of build flags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct FeatureAvailability {
    pub profiles: bool,
    pub installation: bool,
    pub authentication: bool,
    pub game_launch: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct FoundationSnapshot {
    pub launcher_version: &'static str,
    pub target_minecraft_version: &'static str,
    pub platform: Platform,
    pub development_build: bool,
    pub features: FeatureAvailability,
}

#[derive(Debug, Default)]
pub struct ApplicationService;

impl ApplicationService {
    pub fn snapshot(&self) -> Result<FoundationSnapshot> {
        Ok(FoundationSnapshot {
            launcher_version: env!("CARGO_PKG_VERSION"),
            target_minecraft_version: TARGET_MINECRAFT_VERSION,
            platform: Platform::current_binary()?,
            development_build: cfg!(debug_assertions),
            features: FeatureAvailability {
                profiles: true,
                installation: true,
                authentication: cfg!(all(debug_assertions, feature = "dev-auth")),
                game_launch: false,
            },
        })
    }
}

use launcher_core::{
    application::{ApplicationService, FeatureAvailability, FoundationSnapshot},
    core::{LauncherError, Platform},
};
use serde::Serialize;
use tauri::State;

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ApiError {
    code: &'static str,
    message_key: &'static str,
    retryable: bool,
}

impl From<LauncherError> for ApiError {
    fn from(error: LauncherError) -> Self {
        match error {
            LauncherError::UnsupportedPlatform => Self {
                code: "unsupported_platform",
                message_key: "errors.unsupportedPlatform",
                retryable: false,
            },
            _ => Self {
                code: "bootstrap_failed",
                message_key: "errors.bootstrapFailed",
                retryable: true,
            },
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct FeatureAvailabilityDto {
    profiles: bool,
    installation: bool,
    authentication: bool,
    game_launch: bool,
}

impl From<FeatureAvailability> for FeatureAvailabilityDto {
    fn from(features: FeatureAvailability) -> Self {
        Self {
            profiles: features.profiles,
            installation: features.installation,
            authentication: features.authentication,
            game_launch: features.game_launch,
        }
    }
}

#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct SnapshotDto {
    schema_version: u32,
    phase: &'static str,
    launcher_version: &'static str,
    target_minecraft_version: &'static str,
    platform: Platform,
    development_build: bool,
    features: FeatureAvailabilityDto,
}

impl From<FoundationSnapshot> for SnapshotDto {
    fn from(snapshot: FoundationSnapshot) -> Self {
        Self {
            schema_version: 1,
            phase: "foundation",
            launcher_version: snapshot.launcher_version,
            target_minecraft_version: snapshot.target_minecraft_version,
            platform: snapshot.platform,
            development_build: snapshot.development_build,
            features: snapshot.features.into(),
        }
    }
}

#[tauri::command]
pub fn get_snapshot(service: State<'_, ApplicationService>) -> Result<SnapshotDto, ApiError> {
    service
        .snapshot()
        .map(SnapshotDto::from)
        .map_err(ApiError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    use launcher_core::core::{Architecture, OperatingSystem};

    #[test]
    fn wire_contract_matches_the_frontend_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let mut snapshot = ApplicationService.snapshot()?;
        snapshot.platform = Platform {
            os: OperatingSystem::Macos,
            architecture: Architecture::Aarch64,
        };
        snapshot.development_build = true;
        let actual = serde_json::to_value(SnapshotDto::from(snapshot))?;
        let expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/contracts/foundation-snapshot.v1.json"
        ))?;
        assert_eq!(actual, expected);
        Ok(())
    }
}

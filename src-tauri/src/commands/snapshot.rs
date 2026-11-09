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

impl ApiError {
    pub(super) fn code(&self) -> &'static str {
        self.code
    }

    fn storage(code: &'static str, retryable: bool) -> Self {
        Self {
            code,
            message_key: "errors.storage",
            retryable,
        }
    }
}

impl From<LauncherError> for ApiError {
    fn from(error: LauncherError) -> Self {
        match error {
            LauncherError::CredentialStoreUnavailable => Self {
                code: "credential_store_unavailable",
                message_key: "errors.credentialStoreUnavailable",
                retryable: true,
            },
            LauncherError::SavedSessionMissing => Self {
                code: "saved_session_missing",
                message_key: "errors.savedSessionMissing",
                retryable: false,
            },
            LauncherError::AuthRejected => Self {
                code: "auth_rejected",
                message_key: "errors.authRejected",
                retryable: false,
            },
            LauncherError::AuthNetwork => Self {
                code: "auth_network",
                message_key: "errors.authNetwork",
                retryable: true,
            },
            LauncherError::AuthProtocol => Self {
                code: "auth_protocol",
                message_key: "errors.authProtocol",
                retryable: false,
            },
            LauncherError::AuthChallengeExpired => Self {
                code: "auth_challenge_expired",
                message_key: "errors.authChallengeExpired",
                retryable: false,
            },
            LauncherError::AuthBusy => Self {
                code: "auth_busy",
                message_key: "errors.authBusy",
                retryable: true,
            },
            LauncherError::AuthUnavailable => Self {
                code: "auth_unavailable",
                message_key: "errors.authUnavailable",
                retryable: false,
            },
            LauncherError::AuthenticationRequired => Self {
                code: "authentication_required",
                message_key: "errors.authenticationRequired",
                retryable: false,
            },
            LauncherError::UnsupportedAuthFlow => Self {
                code: "unsupported_auth_flow",
                message_key: "errors.unsupportedAuthFlow",
                retryable: false,
            },
            LauncherError::StorageBusy | LauncherError::ProfileBusy => {
                Self::storage("storage_busy", true)
            }
            LauncherError::CorruptDocument => Self::storage("corrupt_document", false),
            LauncherError::UnsupportedSchema => Self::storage("unsupported_schema", false),
            LauncherError::RevisionConflict => Self::storage("revision_conflict", true),
            LauncherError::UnsafePath | LauncherError::UnmanagedDirectory => {
                Self::storage("unsafe_path", false)
            }
            LauncherError::InvalidInput { .. } => Self::storage("invalid_settings", false),
            LauncherError::InvalidLaunchArguments => {
                Self::storage("invalid_launch_arguments", false)
            }
            LauncherError::GameBusy => Self::storage("game_busy", true),
            LauncherError::GameRecoveryRequired => Self::storage("game_recovery_required", false),
            LauncherError::GameSpawn => Self::storage("game_spawn", true),
            LauncherError::GameMonitor => Self::storage("game_monitor", true),
            LauncherError::Network => Self::storage("network", true),
            LauncherError::Integrity => Self::storage("integrity", true),
            LauncherError::Metadata => Self::storage("metadata", false),
            LauncherError::RuntimeIncompatible => Self::storage("runtime", false),
            LauncherError::Durability(_) => Self::storage("durability_failed", true),
            LauncherError::Storage(_) => Self::storage("storage_failed", true),
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
        let mut expected: serde_json::Value = serde_json::from_str(include_str!(
            "../../../tests/contracts/foundation-snapshot.v1.json"
        ))?;
        expected["features"]["authentication"] = true.into();
        assert_eq!(actual, expected);
        Ok(())
    }
}

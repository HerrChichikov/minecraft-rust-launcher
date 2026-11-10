use super::snapshot::ApiError;
use launcher_core::{
    core::{LauncherError, Result},
    profiles::{LaunchSettings, ProfileId, SettingsPolicy},
    storage::{AppPaths, WorkspaceService, WorkspaceSnapshot},
};
use serde::Deserialize;
use std::{path::PathBuf, sync::Mutex};
use tauri::{AppHandle, Manager, Runtime};

#[derive(Debug, Default)]
pub(crate) struct WorkspaceState {
    service: Mutex<Option<WorkspaceService>>,
    paths: Option<AppPaths>,
}
impl WorkspaceState {
    #[cfg(test)]
    pub(crate) fn isolated(paths: AppPaths) -> Self {
        Self {
            service: Mutex::new(None),
            paths: Some(paths),
        }
    }
}

pub(crate) async fn execute<R: Runtime, T: Send + 'static>(
    app: AppHandle<R>,
    action: impl FnOnce(&mut WorkspaceService) -> Result<T> + Send + 'static,
) -> std::result::Result<T, ApiError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<WorkspaceState>();
        let mut guard = state
            .service
            .lock()
            .map_err(|_| LauncherError::StorageBusy)?;
        if guard.is_none() {
            let paths = match &state.paths {
                Some(paths) => paths.clone(),
                None => {
                    let resolver = app.path();
                    AppPaths {
                        data: resolver
                            .app_local_data_dir()
                            .map_err(|_| LauncherError::UnsafePath)?
                            .join("data"),
                        configuration: resolver
                            .app_config_dir()
                            .map_err(|_| LauncherError::UnsafePath)?,
                        cache: resolver
                            .app_cache_dir()
                            .map_err(|_| LauncherError::UnsafePath)?,
                        logs: resolver
                            .app_log_dir()
                            .map_err(|_| LauncherError::UnsafePath)?,
                    }
                }
            };
            let mut system = sysinfo::System::new().map_err(|_| LauncherError::InvalidInput {
                field: "system memory",
            })?;
            system.refresh_memory();
            let policy = SettingsPolicy::new(system.total_memory() / (1024 * 1024))?;
            *guard = Some(WorkspaceService::open(paths, policy)?);
        }
        action(guard.as_mut().ok_or(LauncherError::StorageBusy)?)
    })
    .await
    .map_err(|_| ApiError::from(LauncherError::StorageBusy))?
    .map_err(ApiError::from)
}

#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateRequest {
    root_id: String,
    name: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectRequest {
    root_id: String,
    profile_id: ProfileId,
    expected_revision: u32,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SaveRequest {
    root_id: String,
    profile_id: ProfileId,
    expected_revision: u32,
    name: String,
    settings: LaunchSettings,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RootRequest {
    root_id: String,
    expected_revision: u32,
    path: PathBuf,
}

fn root_id(value: &str) -> Result<launcher_core::storage::RootId> {
    value
        .parse()
        .map_err(|_| LauncherError::InvalidInput { field: "root ID" })
}

#[tauri::command]
pub async fn get_workspace<R: Runtime>(
    app: AppHandle<R>,
) -> std::result::Result<WorkspaceSnapshot, ApiError> {
    execute(app, |service| service.snapshot()).await
}
#[tauri::command]
pub async fn create_profile<R: Runtime>(
    app: AppHandle<R>,
    request: CreateRequest,
) -> std::result::Result<WorkspaceSnapshot, ApiError> {
    execute(app, move |s| {
        s.create_profile(root_id(&request.root_id)?, &request.name)
    })
    .await
}
#[tauri::command]
pub async fn select_profile<R: Runtime>(
    app: AppHandle<R>,
    request: SelectRequest,
) -> std::result::Result<WorkspaceSnapshot, ApiError> {
    execute(app, move |s| {
        s.select_profile(
            root_id(&request.root_id)?,
            request.profile_id,
            request.expected_revision,
        )
    })
    .await
}
#[tauri::command]
pub async fn save_profile<R: Runtime>(
    app: AppHandle<R>,
    request: SaveRequest,
) -> std::result::Result<WorkspaceSnapshot, ApiError> {
    execute(app, move |s| {
        s.save_profile(
            root_id(&request.root_id)?,
            request.profile_id,
            request.expected_revision,
            &request.name,
            request.settings,
        )
    })
    .await
}
#[tauri::command]
pub async fn switch_data_root<R: Runtime>(
    app: AppHandle<R>,
    request: RootRequest,
) -> std::result::Result<WorkspaceSnapshot, ApiError> {
    execute(app, move |s| {
        s.switch_root(
            root_id(&request.root_id)?,
            request.expected_revision,
            &request.path,
        )
    })
    .await
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn workspace_wire_contract_matches_frontend_fixture()
    -> std::result::Result<(), Box<dyn std::error::Error>> {
        let snapshot = WorkspaceSnapshot {
            schema_version: 1,
            root_id: root_id("12345678-1234-4567-8123-123456789abc")?,
            data_root: "/test/Launcher/data".into(),
            configuration_revision: 1,
            revision: 1,
            active_profile_id: serde_json::from_value(serde_json::json!(
                "23456789-1234-4567-8123-123456789abc"
            ))?,
            installation_recorded: false,
            profiles: vec![launcher_core::profiles::Profile {
                schema_version: 1,
                revision: 1,
                id: serde_json::from_value(serde_json::json!(
                    "23456789-1234-4567-8123-123456789abc"
                ))?,
                name: "Основная конфигурация".into(),
                minecraft_version: "1.21.1".into(),
                settings: SettingsPolicy::new(8192)?.defaults(),
            }],
            policy: SettingsPolicy::new(8192)?,
        };
        assert_eq!(
            serde_json::to_value(snapshot)?,
            serde_json::from_str::<serde_json::Value>(include_str!(
                "../../../tests/contracts/workspace.v1.json"
            ))?
        );
        Ok(())
    }
}

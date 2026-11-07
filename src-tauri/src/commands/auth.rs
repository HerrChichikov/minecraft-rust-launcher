use super::snapshot::ApiError;
use launcher_core::{
    auth::{AuthService, AuthSnapshot},
    core::LauncherError,
};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::State;

#[derive(Debug, Default)]
pub(crate) struct AuthState(Mutex<AuthService>);
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginRequest {
    expected_revision: u32,
    provider_id: String,
    nickname: String,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LogoutRequest {
    expected_revision: u32,
}

#[tauri::command]
pub fn get_auth(state: State<'_, AuthState>) -> Result<AuthSnapshot, ApiError> {
    state
        .0
        .lock()
        .map(|s| s.snapshot())
        .map_err(|_| LauncherError::StorageBusy.into())
}
#[tauri::command]
pub fn login(state: State<'_, AuthState>, request: LoginRequest) -> Result<AuthSnapshot, ApiError> {
    state
        .0
        .lock()
        .map_err(|_| LauncherError::StorageBusy)?
        .login(
            request.expected_revision,
            &request.provider_id,
            &request.nickname,
        )
        .map_err(ApiError::from)
}
#[tauri::command]
pub fn logout(
    state: State<'_, AuthState>,
    request: LogoutRequest,
) -> Result<AuthSnapshot, ApiError> {
    state
        .0
        .lock()
        .map_err(|_| LauncherError::StorageBusy)?
        .logout(request.expected_revision)
        .map_err(ApiError::from)
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_auth_contract_matches_frontend_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let mut actual = serde_json::to_value(AuthService::default().snapshot())?;
        // The shared fixture describes production; opt-in development adds one capability.
        actual["providers"] = serde_json::Value::Array(
            actual["providers"]
                .as_array()
                .ok_or("providers missing")?
                .iter()
                .filter(|p| p["id"] == "ely-by")
                .cloned()
                .collect(),
        );
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/contracts/auth.v1.json"))?;
        assert_eq!(actual, expected);
        Ok(())
    }
}

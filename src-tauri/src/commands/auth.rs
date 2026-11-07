use super::snapshot::ApiError;
use launcher_core::{
    auth::{AuthService, AuthSnapshot, SecretString},
    core::LauncherError,
};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{AppHandle, Manager, Runtime, State};
#[derive(Debug)]
pub(crate) struct AuthState(Mutex<AuthService>);
impl Default for AuthState {
    fn default() -> Self {
        #[cfg(test)]
        let service = AuthService::default();
        #[cfg(not(test))]
        let service = AuthService::with_credentials(std::sync::Arc::new(
            crate::credentials::NativeCredentialStore::default(),
        ));
        Self(Mutex::new(service))
    }
}
#[derive(Debug, Deserialize)]
#[serde(tag = "flow", rename_all = "snake_case", deny_unknown_fields)]
pub enum LoginInput {
    Password {
        login: String,
        password: SecretString,
    },
    #[cfg(all(debug_assertions, feature = "dev-auth"))]
    DevelopmentNickname { nickname: String },
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct LoginRequest {
    expected_revision: u32,
    provider_id: String,
    input: LoginInput,
    #[serde(default)]
    remember: bool,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct RevisionRequest {
    expected_revision: u32,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct ContinueRequest {
    expected_revision: u32,
    challenge_id: String,
    totp: SecretString,
}

async fn execute<R: Runtime>(
    app: AppHandle<R>,
    action: impl FnOnce(&mut AuthService) -> launcher_core::core::Result<AuthSnapshot> + Send + 'static,
) -> Result<AuthSnapshot, ApiError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AuthState>();
        let mut service = state.0.try_lock().map_err(|_| LauncherError::AuthBusy)?;
        action(&mut service)
    })
    .await
    .map_err(|_| ApiError::from(LauncherError::AuthBusy))?
    .map_err(ApiError::from)
}
#[tauri::command]
pub fn get_auth(state: State<'_, AuthState>) -> Result<AuthSnapshot, ApiError> {
    state
        .0
        .try_lock()
        .map(|mut s| {
            s.expire_challenge();
            s.snapshot()
        })
        .map_err(|_| LauncherError::AuthBusy.into())
}
#[tauri::command]
pub async fn login<R: Runtime>(
    app: AppHandle<R>,
    request: LoginRequest,
) -> Result<AuthSnapshot, ApiError> {
    let outcome = execute(app.clone(), move |s| {
        match (request.provider_id.as_str(), request.input) {
            ("ely-by", LoginInput::Password { login, password }) => s.login_password_with_remember(
                request.expected_revision,
                login,
                password,
                request.remember,
            ),
            #[cfg(all(debug_assertions, feature = "dev-auth"))]
            (provider, LoginInput::DevelopmentNickname { nickname }) => {
                if request.remember {
                    return Err(LauncherError::UnsupportedAuthFlow);
                }
                s.login(request.expected_revision, provider, &nickname)
            }
            _ => Err(LauncherError::UnsupportedAuthFlow),
        }
    })
    .await?;
    if outcome.challenge.is_some() {
        tauri::async_runtime::spawn(async move {
            tokio::time::sleep(std::time::Duration::from_secs(120)).await;
            loop {
                let finished = {
                    let state = app.state::<AuthState>();
                    match state.0.try_lock() {
                        Ok(mut s) => {
                            s.expire_challenge();
                            true
                        }
                        Err(std::sync::TryLockError::Poisoned(p)) => {
                            p.into_inner().expire_challenge();
                            true
                        }
                        Err(std::sync::TryLockError::WouldBlock) => false,
                    }
                };
                if finished {
                    break;
                }
                tokio::time::sleep(std::time::Duration::from_millis(100)).await;
            }
        });
    }
    Ok(outcome)
}
#[tauri::command]
pub async fn continue_login<R: Runtime>(
    app: AppHandle<R>,
    request: ContinueRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| {
        let id = request
            .challenge_id
            .parse()
            .map_err(|_| LauncherError::AuthChallengeExpired)?;
        s.continue_login(request.expected_revision, id, request.totp)
    })
    .await
}
#[tauri::command]
pub async fn cancel_login<R: Runtime>(
    app: AppHandle<R>,
    request: RevisionRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| s.cancel_login(request.expected_revision)).await
}
#[tauri::command]
pub async fn refresh_auth<R: Runtime>(
    app: AppHandle<R>,
    request: RevisionRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| s.refresh(request.expected_revision)).await
}
#[tauri::command]
pub async fn logout<R: Runtime>(
    app: AppHandle<R>,
    request: RevisionRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| s.logout(request.expected_revision)).await
}
#[tauri::command]
pub async fn restore_auth<R: Runtime>(
    app: AppHandle<R>,
    request: RevisionRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| s.restore(request.expected_revision)).await
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn public_auth_contract_matches_frontend_fixture() -> Result<(), Box<dyn std::error::Error>> {
        let mut actual = serde_json::to_value(AuthService::default().snapshot())?;
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

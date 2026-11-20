use super::snapshot::ApiError;
use launcher_core::{
    auth::{AuthService, AuthSnapshot, LaunchIdentity, PROVIDER_ID, SecretString},
    core::{LauncherError, Result as CoreResult},
};
use serde::Deserialize;
use std::sync::Mutex;
use tauri::{AppHandle, Emitter, Manager, Runtime, State};

#[derive(Debug)]
pub(crate) struct AuthState(Mutex<AuthService>);
impl AuthState {
    pub(crate) fn launch_session(&self) -> CoreResult<(u32, bool)> {
        self.0
            .try_lock()
            .map_err(|_| LauncherError::AuthBusy)?
            .launch_requirement()
    }

    pub(crate) fn with_launch_identity<T, R: Runtime>(
        &self,
        app: &AppHandle<R>,
        revision: u32,
        launch: impl FnOnce(LaunchIdentity) -> CoreResult<T>,
    ) -> CoreResult<T> {
        let mut service = self.0.try_lock().map_err(|_| LauncherError::AuthBusy)?;
        if service.launch_requirement()?.0 != revision {
            return Err(LauncherError::RevisionConflict);
        }
        let identity = service.launch_identity();
        publish_change(app, revision, &service);
        launch(identity?)
    }
}
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

fn publish_change<R: Runtime>(app: &AppHandle<R>, previous: u32, service: &AuthService) {
    let snapshot = service.snapshot();
    if snapshot.revision != previous {
        let _ = app.emit_to("main", "launcher://auth", snapshot);
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
pub struct RegisterRequest {
    expected_revision: u32,
    nickname: String,
    password: SecretString,
    email: String,
    invitation_code: SecretString,
}
#[derive(Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct SelectRequest {
    expected_revision: u32,
    game_profile_id: String,
}

async fn execute<R: Runtime>(
    app: AppHandle<R>,
    action: impl FnOnce(&mut AuthService) -> CoreResult<AuthSnapshot> + Send + 'static,
) -> Result<AuthSnapshot, ApiError> {
    tauri::async_runtime::spawn_blocking(move || {
        let state = app.state::<AuthState>();
        let mut service = state.0.try_lock().map_err(|_| LauncherError::AuthBusy)?;
        let previous = service.snapshot().revision;
        let result = action(&mut service);
        publish_change(&app, previous, &service);
        result
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
        .map(|s| s.snapshot())
        .map_err(|_| LauncherError::AuthBusy.into())
}
#[tauri::command]
pub async fn login<R: Runtime>(
    app: AppHandle<R>,
    request: LoginRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| {
        match (request.provider_id.as_str(), request.input) {
            (PROVIDER_ID, LoginInput::Password { login, password }) => s
                .login_password_with_remember(
                    request.expected_revision,
                    &login,
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
    .await
}
#[tauri::command]
pub async fn register_account<R: Runtime>(
    app: AppHandle<R>,
    request: RegisterRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| {
        s.register(
            request.expected_revision,
            &request.nickname,
            request.password,
            &request.email,
            request.invitation_code.expose_secret(),
        )
    })
    .await
}
#[tauri::command]
pub async fn select_game_profile<R: Runtime>(
    app: AppHandle<R>,
    request: SelectRequest,
) -> Result<AuthSnapshot, ApiError> {
    execute(app, move |s| {
        let id = request
            .game_profile_id
            .parse()
            .map_err(|_| LauncherError::GameProfileUnavailable)?;
        s.select_profile(request.expected_revision, id)
    })
    .await
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
        let expected: serde_json::Value =
            serde_json::from_str(include_str!("../../../tests/contracts/auth.v2.json"))?;
        let providers = actual["providers"]
            .as_array_mut()
            .ok_or("providers missing")?;
        providers.retain(|p| p["id"] == PROVIDER_ID);
        if let Some(url) = option_env!("LAUNCHER_AUTH_URL") {
            let available = launcher_core::auth::PlatformClient::new(url).is_ok();
            assert_eq!(providers[0]["available"], available);
            providers[0] = expected["providers"][0].clone();
        }
        assert_eq!(actual, expected);
        Ok(())
    }
}

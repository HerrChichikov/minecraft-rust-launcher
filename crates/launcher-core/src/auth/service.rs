#[cfg(all(debug_assertions, feature = "dev-auth"))]
use super::AuthProvider;
use super::{
    LaunchIdentity, SecretString, credentials,
    platform::{Account, GameProfile, PROVIDER_ID, PlatformClient, TokenPair},
};
use crate::{
    core::{LauncherError as Error, Result},
    storage::CredentialStore,
};
use serde::Serialize;
use std::sync::Arc;
use uuid::Uuid;

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub id: String,
    pub available: bool,
    pub development: bool,
    pub flows: Vec<String>,
    pub unavailable_reason: Option<String>,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub provider_id: String,
    pub player_id: Uuid,
    pub player_name: String,
    pub development: bool,
}

#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSnapshot {
    pub schema_version: u32,
    pub revision: u32,
    pub providers: Vec<ProviderCapabilities>,
    pub account: Option<AccountSummary>,
    pub game_profiles: Vec<GameProfile>,
    pub selected_game_profile_id: Option<Uuid>,
    pub persistence: &'static str,
    pub remember_supported: bool,
    pub notice: Option<&'static str>,
}

#[derive(Debug)]
struct PlatformSession {
    account: Account,
    tokens: TokenPair,
    profiles: Vec<GameProfile>,
    selected: Option<Uuid>,
}

#[derive(Debug)]
pub struct AuthService {
    platform: Option<PlatformClient>,
    session: Option<PlatformSession>,
    #[cfg(all(debug_assertions, feature = "dev-auth"))]
    development: Option<super::Session>,
    revision: u32,
    notice: Option<&'static str>,
    credentials: Option<Arc<dyn CredentialStore>>,
    remembered: bool,
}
impl Default for AuthService {
    fn default() -> Self {
        Self {
            platform: option_env!("LAUNCHER_AUTH_URL")
                .and_then(|url| PlatformClient::new(url).ok()),
            session: None,
            #[cfg(all(debug_assertions, feature = "dev-auth"))]
            development: None,
            revision: 0,
            notice: None,
            credentials: None,
            remembered: false,
        }
    }
}
impl AuthService {
    pub fn with_credentials(credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            credentials: Some(credentials),
            ..Self::default()
        }
    }

    pub fn with_platform(
        platform: PlatformClient,
        credentials: Option<Arc<dyn CredentialStore>>,
    ) -> Self {
        Self {
            platform: Some(platform),
            credentials,
            ..Self::default()
        }
    }

    fn client(&self) -> Result<&PlatformClient> {
        self.platform.as_ref().ok_or(Error::AuthUnavailable)
    }

    fn update_profiles(&mut self) {
        if let (Some(platform), Some(session)) = (&self.platform, &mut self.session) {
            match platform.profiles(&session.tokens.access_token) {
                Ok(profiles) => {
                    if !profiles.iter().any(|p| Some(p.id) == session.selected) {
                        session.selected = (profiles.len() == 1).then(|| profiles[0].id);
                    }
                    session.profiles = profiles;
                }
                Err(_) if self.notice.is_none() => self.notice = Some("game_profiles_unavailable"),
                Err(_) => {}
            }
        }
    }

    fn persist(&mut self) {
        if !self.remembered {
            return;
        }
        let saved = (|| {
            let store = self
                .credentials
                .as_ref()
                .ok_or(Error::CredentialStoreUnavailable)?;
            let session = self.session.as_ref().ok_or(Error::AuthenticationRequired)?;
            store.write(
                credentials::REFERENCE,
                &credentials::encode(self.client()?.origin(), &session.tokens.refresh_token)?,
            )
        })();
        if saved.is_err() {
            self.remembered = false;
            self.notice = Some("credential_store_unavailable");
        }
    }

    pub fn snapshot(&self) -> AuthSnapshot {
        let available = self.platform.is_some();
        #[allow(unused_mut)]
        let mut providers = vec![ProviderCapabilities {
            id: PROVIDER_ID.into(),
            available,
            development: false,
            flows: if available {
                vec!["password".into()]
            } else {
                vec![]
            },
            unavailable_reason: (!available).then(|| "platform_url_missing".into()),
        }];
        #[allow(unused_mut)]
        let mut account = self.session.as_ref().map(|s| AccountSummary {
            provider_id: PROVIDER_ID.into(),
            player_id: s.account.id,
            player_name: s.account.nickname.clone(),
            development: false,
        });
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        {
            providers.push(super::MockAuthProvider.capabilities());
            if let Some(s) = &self.development {
                account = Some(AccountSummary {
                    provider_id: s.provider_id.clone(),
                    player_id: s.profile.id,
                    player_name: s.profile.name.clone(),
                    development: true,
                });
            }
        }
        AuthSnapshot {
            schema_version: 2,
            revision: self.revision,
            providers,
            account,
            game_profiles: self
                .session
                .as_ref()
                .map_or_else(Vec::new, |s| s.profiles.clone()),
            selected_game_profile_id: self.session.as_ref().and_then(|s| s.selected),
            persistence: if self.remembered {
                "system_credential_store"
            } else {
                "memory_only"
            },
            remember_supported: self.credentials.is_some() && available,
            notice: self.notice,
        }
    }

    fn next(&self, expected: u32) -> Result<u32> {
        if expected != self.revision {
            return Err(Error::RevisionConflict);
        }
        self.revision.checked_add(1).ok_or(Error::RevisionConflict)
    }

    pub fn register(
        &mut self,
        expected: u32,
        nickname: &str,
        password: SecretString,
        email: &str,
        invite: &str,
    ) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        self.client()?
            .register(nickname, &password, email, invite)?;
        self.revision = next;
        self.notice = Some("registered");
        Ok(self.snapshot())
    }

    pub fn login_password_with_remember(
        &mut self,
        expected: u32,
        login: &str,
        password: SecretString,
        remember: bool,
    ) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        if self.session.is_some() {
            return Err(Error::AuthBusy);
        }
        let result = self.client()?.login(login, &password)?;
        self.complete_login(next, result.user, result.tokens, remember);
        Ok(self.snapshot())
    }

    fn complete_login(&mut self, next: u32, account: Account, tokens: TokenPair, remember: bool) {
        self.session = Some(PlatformSession {
            account,
            tokens,
            profiles: vec![],
            selected: None,
        });
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        {
            self.development = None;
        }
        self.revision = next;
        self.notice = None;
        self.remembered = remember;
        self.persist();
        self.update_profiles();
    }

    pub fn login(&mut self, expected: u32, provider: &str, nickname: &str) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        if provider == "development-mock" && self.session.is_none() {
            self.development = Some(super::MockAuthProvider.login(nickname)?);
            self.revision = next;
            self.notice = None;
            return Ok(self.snapshot());
        }
        let _ = (next, provider, nickname);
        Err(Error::AuthUnavailable)
    }

    pub fn restore(&mut self, expected: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        if self.session.is_some() {
            return Err(Error::AuthBusy);
        }
        let client = self.client()?;
        let store = self
            .credentials
            .as_ref()
            .ok_or(Error::CredentialStoreUnavailable)?;
        let saved = store
            .read(credentials::REFERENCE)?
            .ok_or(Error::SavedSessionMissing)?;
        let record = credentials::decode(&saved, client.origin())?;
        let tokens = match client.refresh(&record.refresh) {
            Err(Error::AuthenticationRequired) => {
                self.clear_local(next);
                return Err(Error::AuthenticationRequired);
            }
            result => result?,
        };
        // Persist the rotated credential even when fetching the account fails transiently.
        let saved = store
            .write(
                credentials::REFERENCE,
                &credentials::encode(client.origin(), &tokens.refresh_token)?,
            )
            .is_ok();
        let account = client.account(&tokens.access_token)?;
        self.complete_login(next, account, tokens, false);
        self.remembered = saved;
        if !saved {
            self.notice = Some("credential_store_unavailable");
        }
        Ok(self.snapshot())
    }

    pub fn refresh(&mut self, expected: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        let session = self.session.as_ref().ok_or(Error::AuthenticationRequired)?;
        let tokens = self.client()?.refresh(&session.tokens.refresh_token);
        if matches!(tokens, Err(Error::AuthenticationRequired)) {
            self.clear_local(next);
            return Err(Error::AuthenticationRequired);
        }
        let tokens = tokens?;
        if tokens.session_id != session.tokens.session_id {
            self.clear_local(next);
            return Err(Error::AuthProtocol);
        }
        self.session
            .as_mut()
            .ok_or(Error::AuthenticationRequired)?
            .tokens = tokens;
        self.revision = next;
        self.notice = None;
        self.persist();
        self.update_profiles();
        Ok(self.snapshot())
    }

    pub fn select_profile(&mut self, expected: u32, id: Uuid) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        let session = self.session.as_mut().ok_or(Error::AuthenticationRequired)?;
        if !session.profiles.iter().any(|p| p.id == id) {
            return Err(Error::GameProfileUnavailable);
        }
        session.selected = Some(id);
        self.revision = next;
        Ok(self.snapshot())
    }

    pub fn logout(&mut self, expected: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        let could_revoke = match (&self.platform, &self.session) {
            (Some(client), Some(session)) => client.logout(&session.tokens.refresh_token).is_ok(),
            (Some(client), None) => match &self.credentials {
                Some(store) => match store.read(credentials::REFERENCE) {
                    Ok(Some(saved)) => credentials::decode(&saved, client.origin())
                        .is_ok_and(|record| client.logout(&record.refresh).is_ok()),
                    Ok(None) => true,
                    Err(_) => false,
                },
                None => true,
            },
            _ => true,
        };
        self.clear_local(next);
        if !could_revoke {
            self.notice = Some(if self.notice.is_some() {
                "logout_cleanup_failed"
            } else {
                "remote_logout_failed"
            });
        }
        Ok(self.snapshot())
    }

    fn clear_local(&mut self, next: u32) {
        self.session = None;
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        {
            self.development = None;
        }
        self.remembered = false;
        self.revision = next;
        self.notice = self
            .credentials
            .as_ref()
            .filter(|store| store.delete(credentials::REFERENCE).is_err())
            .map(|_| "credential_delete_failed");
    }

    pub fn launch_requirement(&self) -> Result<(u32, bool)> {
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        if self.development.is_some() {
            return Ok((self.revision, false));
        }
        let session = self.session.as_ref().ok_or(Error::AuthenticationRequired)?;
        if session.selected.is_none() {
            return Err(Error::GameProfileUnavailable);
        }
        Ok((self.revision, true))
    }

    pub fn launch_identity(&mut self) -> Result<LaunchIdentity> {
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        if let Some(session) = &self.development {
            return super::MockAuthProvider.identity(session);
        }
        let selected = self
            .session
            .as_ref()
            .ok_or(Error::AuthenticationRequired)?
            .selected;
        let issue = |service: &Self| {
            let session = service
                .session
                .as_ref()
                .ok_or(Error::AuthenticationRequired)?;
            let profile = session
                .profiles
                .iter()
                .find(|p| Some(p.id) == selected && selected == session.selected)
                .ok_or(Error::GameProfileUnavailable)?;
            let (game, root) = service
                .client()?
                .launch(&session.tokens.access_token, profile)?;
            Ok(LaunchIdentity {
                profile: game.profile.player(),
                access_token: game.access_token,
                user_type: "mojang".into(),
                agent_api: Some(root),
            })
        };
        match issue(self) {
            Err(Error::AuthenticationRequired) if self.session.is_some() => {
                self.refresh(self.revision)?;
                issue(self)
            }
            other => other,
        }
    }
}

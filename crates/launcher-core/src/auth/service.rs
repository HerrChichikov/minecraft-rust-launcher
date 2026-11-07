use super::*;
use crate::core::LauncherError as Error;
use crate::storage::CredentialStore;
use serde::Serialize;
use std::{
    sync::Arc,
    time::{Duration, Instant},
};
#[derive(Debug, Clone, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ProviderCapabilities {
    pub id: String,
    pub available: bool,
    pub development: bool,
    pub flows: Vec<String>,
    pub unavailable_reason: Option<String>,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AccountSummary {
    pub provider_id: String,
    pub player_id: Uuid,
    pub player_name: String,
    pub development: bool,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct ChallengeSummary {
    pub id: Uuid,
    pub provider_id: &'static str,
    pub expires_in_seconds: u64,
}
#[derive(Debug, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct AuthSnapshot {
    pub schema_version: u32,
    pub revision: u32,
    pub providers: Vec<ProviderCapabilities>,
    pub account: Option<AccountSummary>,
    pub challenge: Option<ChallengeSummary>,
    pub persistence: &'static str,
    pub remember_supported: bool,
    pub notice: Option<&'static str>,
}
#[derive(Debug)]
struct PendingChallenge {
    id: Uuid,
    login: String,
    password: SecretString,
    client_token: SecretString,
    expires: Instant,
    attempts: u8,
    remember: bool,
}
/// Single-flight owner; desktop runs bounded network operations on blocking workers.
#[derive(Debug, Default)]
pub struct AuthService {
    session: Option<Session>,
    pending: Option<PendingChallenge>,
    revision: u32,
    ely: ElyByAuthProvider,
    notice: Option<&'static str>,
    credentials: Option<Arc<dyn CredentialStore>>,
    remembered: bool,
}
impl AuthService {
    pub fn with_credentials(credentials: Arc<dyn CredentialStore>) -> Self {
        Self {
            credentials: Some(credentials),
            ..Self::default()
        }
    }
    fn commit_session(&mut self, session: Session, remember: bool) {
        self.session = Some(session);
        self.remembered = false;
        if remember {
            let saved = (|| {
                let store = self
                    .credentials
                    .as_ref()
                    .ok_or(Error::CredentialStoreUnavailable)?;
                let session = self.session.as_ref().ok_or(Error::AuthenticationRequired)?;
                store.write(
                    super::credentials::REFERENCE,
                    &super::credentials::encode(session)?,
                )
            })()
            .is_ok();
            if saved {
                self.remembered = true;
            } else {
                self.notice = Some("credential_store_unavailable");
            }
        }
    }
    pub fn restore(&mut self, expected: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        if self.pending.is_some() {
            return Err(Error::AuthBusy);
        }
        let store = self
            .credentials
            .as_ref()
            .ok_or(Error::CredentialStoreUnavailable)?;
        let saved = store
            .read(super::credentials::REFERENCE)?
            .ok_or(Error::SavedSessionMissing)?;
        let session = super::credentials::decode(&saved)?;
        self.ely.validate(&session)?;
        // Refresh binds the restored UUID/name to a fresh provider response, even when valid.
        let session = self.ely.refresh(&session)?;
        self.notice = None;
        self.commit_session(session, true);
        self.revision = next;
        Ok(self.snapshot())
    }
    /// The application supervisor calls this at the challenge deadline, even with an idle UI.
    pub fn expire_challenge(&mut self) {
        if self
            .pending
            .as_ref()
            .is_some_and(|p| p.expires <= Instant::now())
        {
            self.pending = None;
            self.revision = self.revision.saturating_add(1);
        }
    }
    pub fn snapshot(&self) -> AuthSnapshot {
        #[allow(unused_mut)]
        let mut providers = vec![self.ely.capabilities()];
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        providers.push(MockAuthProvider.capabilities());
        AuthSnapshot {
            schema_version: 1,
            revision: self.revision,
            providers,
            account: self.session.as_ref().map(|s| AccountSummary {
                provider_id: s.provider_id.clone(),
                player_id: s.profile.id,
                player_name: s.profile.name.clone(),
                development: s.provider_id != "ely-by",
            }),
            challenge: self
                .pending
                .as_ref()
                .filter(|p| p.expires > Instant::now())
                .map(|p| ChallengeSummary {
                    id: p.id,
                    provider_id: "ely-by",
                    expires_in_seconds: p
                        .expires
                        .saturating_duration_since(Instant::now())
                        .as_secs(),
                }),
            persistence: if self.remembered {
                "system_credential_store"
            } else {
                "memory_only"
            },
            remember_supported: self.credentials.is_some(),
            notice: self.notice,
        }
    }
    fn next(&self, expected: u32) -> Result<u32> {
        if expected != self.revision {
            return Err(Error::RevisionConflict);
        }
        self.revision.checked_add(1).ok_or(Error::RevisionConflict)
    }
    pub fn login(&mut self, expected: u32, provider: &str, nickname: &str) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        if provider == MockAuthProvider.id() {
            let session = MockAuthProvider.login(nickname)?;
            self.session = Some(session);
            self.remembered = false;
            self.pending = None;
            self.revision = next;
            self.notice = None;
            return Ok(self.snapshot());
        }
        let _ = (next, provider, nickname);
        Err(Error::AuthUnavailable)
    }
    pub fn login_password(
        &mut self,
        expected: u32,
        login: String,
        password: SecretString,
    ) -> Result<AuthSnapshot> {
        self.login_password_with_remember(expected, login, password, false)
    }
    pub fn login_password_with_remember(
        &mut self,
        expected: u32,
        login: String,
        password: SecretString,
        remember: bool,
    ) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        self.notice = None;
        self.pending = None;
        let client = SecretString::new(Uuid::new_v4().to_string());
        match self.ely.password(&login, &password, None, &client)? {
            AuthOutcome::Authenticated(session) => self.commit_session(session, remember),
            AuthOutcome::TotpRequired { challenge_id } => {
                self.pending = Some(PendingChallenge {
                    id: challenge_id,
                    login,
                    password,
                    client_token: client,
                    expires: Instant::now() + Duration::from_secs(120),
                    attempts: 0,
                    remember,
                })
            }
        }
        self.revision = next;
        Ok(self.snapshot())
    }
    pub fn continue_login(
        &mut self,
        expected: u32,
        id: Uuid,
        totp: SecretString,
    ) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        let pending = self
            .pending
            .as_ref()
            .filter(|p| p.id == id)
            .ok_or(Error::AuthChallengeExpired)?;
        if pending.expires <= Instant::now() {
            self.pending = None;
            self.revision = next;
            return Err(Error::AuthChallengeExpired);
        }
        let mut pending = self.pending.take().ok_or(Error::AuthChallengeExpired)?;
        pending.attempts += 1;
        let outcome = self.ely.password(
            &pending.login,
            &pending.password,
            Some(&totp),
            &pending.client_token,
        );
        match outcome {
            Ok(AuthOutcome::Authenticated(session)) => {
                self.notice = None;
                self.commit_session(session, pending.remember);
                self.revision = next;
                Ok(self.snapshot())
            }
            other => {
                if pending.attempts < 5 && pending.expires > Instant::now() {
                    self.pending = Some(pending);
                } else {
                    self.revision = next;
                }
                match other {
                    Err(e) => Err(e),
                    _ => Err(Error::AuthRejected),
                }
            }
        }
    }
    pub fn cancel_login(&mut self, expected: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        self.pending = None;
        self.revision = next;
        self.notice = None;
        Ok(self.snapshot())
    }
    pub fn refresh(&mut self, expected: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        if self.pending.is_some() {
            return Err(Error::AuthBusy);
        }
        let session = self.session.as_ref().ok_or(Error::AuthenticationRequired)?;
        let fresh = if session.provider_id == self.ely.id() {
            self.ely.refresh(session)?
        } else {
            #[cfg(all(debug_assertions, feature = "dev-auth"))]
            {
                if session.provider_id != MockAuthProvider.id() {
                    return Err(Error::AuthUnavailable);
                }
                MockAuthProvider.refresh(session)?
            }
            #[cfg(not(all(debug_assertions, feature = "dev-auth")))]
            {
                return Err(Error::AuthUnavailable);
            }
        };
        let remember = self.remembered;
        self.notice = None;
        self.commit_session(fresh, remember);
        self.revision = next;
        Ok(self.snapshot())
    }
    pub fn logout(&mut self, expected: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected)?;
        let session = self.session.take();
        self.pending = None;
        self.revision = next;
        self.remembered = false;
        self.notice = None;
        let delete_failed = self
            .credentials
            .as_ref()
            .is_some_and(|s| s.delete(super::credentials::REFERENCE).is_err());
        let remote_failed = session
            .as_ref()
            .is_some_and(|s| s.provider_id == self.ely.id() && self.ely.logout(s).is_err());
        self.notice = match (delete_failed, remote_failed) {
            (true, true) => Some("logout_cleanup_failed"),
            (true, false) => Some("credential_delete_failed"),
            (false, true) => Some("remote_logout_failed"),
            _ => None,
        };
        Ok(self.snapshot())
    }
    /// No caller-supplied session. Validate again after installation, immediately before future spawn.
    pub fn launch_identity(&mut self) -> Result<LaunchIdentity> {
        let session = self.session.as_ref().ok_or(Error::AuthenticationRequired)?;
        if session.provider_id == self.ely.id() {
            if self.ely.validate(session)? == SessionValidity::ReauthenticationRequired {
                self.refresh(self.revision)?;
            }
            return self
                .ely
                .launch_identity(self.session.as_ref().ok_or(Error::AuthenticationRequired)?);
        }
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        if session.provider_id == MockAuthProvider.id() {
            return MockAuthProvider.identity(session);
        }
        Err(Error::AuthUnavailable)
    }
    #[cfg(test)]
    pub(crate) fn set_test_credentials(&mut self, store: Arc<dyn CredentialStore>) {
        self.credentials = Some(store);
    }
    #[cfg(test)]
    pub(crate) fn expire_test_challenge(&mut self) {
        if let Some(p) = self.pending.as_mut() {
            p.expires = Instant::now() - Duration::from_secs(1);
        }
    }
    #[cfg(test)]
    pub(crate) fn with_test_provider(ely: ElyByAuthProvider) -> Self {
        Self {
            ely,
            ..Self::default()
        }
    }
}

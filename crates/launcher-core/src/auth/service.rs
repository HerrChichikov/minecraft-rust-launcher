use super::*;
use crate::core::LauncherError as Error;
use serde::Serialize;

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
pub struct AuthSnapshot {
    pub schema_version: u32,
    pub revision: u32,
    pub providers: Vec<ProviderCapabilities>,
    pub account: Option<AccountSummary>,
    pub persistence: &'static str,
}
/// Process-only sessions: no config restoration, no token serialization or plaintext fallback.
#[derive(Debug, Default)]
pub struct AuthService {
    session: Option<Session>,
    revision: u32,
}
impl AuthService {
    pub fn snapshot(&self) -> AuthSnapshot {
        #[allow(unused_mut)]
        let mut providers = vec![ElyByAuthProvider.capabilities()];
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
                development: true,
            }),
            persistence: "memory_only",
        }
    }
    fn next(&self, expected_revision: u32) -> Result<u32> {
        if expected_revision != self.revision {
            return Err(Error::RevisionConflict);
        }
        self.revision.checked_add(1).ok_or(Error::RevisionConflict)
    }
    pub fn login(
        &mut self,
        expected_revision: u32,
        provider_id: &str,
        nickname: &str,
    ) -> Result<AuthSnapshot> {
        let next = self.next(expected_revision)?;
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        if provider_id == MockAuthProvider.id() {
            let session = MockAuthProvider.login(nickname)?;
            self.session = Some(session);
            self.revision = next;
            return Ok(self.snapshot());
        }
        let _ = (next, provider_id, nickname);
        Err(Error::AuthUnavailable)
    }
    pub fn logout(&mut self, expected_revision: u32) -> Result<AuthSnapshot> {
        let next = self.next(expected_revision)?;
        self.session = None;
        self.revision = next;
        Ok(self.snapshot())
    }
    /// Rechecks the build/provider gate at the final launch boundary. No caller-supplied session.
    pub fn launch_identity(&self) -> Result<LaunchIdentity> {
        let session = self.session.as_ref().ok_or(Error::AuthenticationRequired)?;
        #[cfg(all(debug_assertions, feature = "dev-auth"))]
        if session.provider_id == MockAuthProvider.id() {
            return MockAuthProvider.identity(session);
        }
        let _ = session;
        Err(Error::AuthUnavailable)
    }
}

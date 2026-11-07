//! Backend-owned sessions. Only sanitized account summaries cross IPC.

pub mod agent;
mod ely;
mod service;
pub use ely::{ElyByAuthProvider, ElyByConfiguration};
pub use service::{AccountSummary, AuthService, AuthSnapshot, ProviderCapabilities};
#[cfg(all(debug_assertions, feature = "dev-auth"))]
mod mock;
#[cfg(all(debug_assertions, feature = "dev-auth"))]
pub use mock::MockAuthProvider;

use std::{fmt, future::Future};

use uuid::Uuid;
use zeroize::Zeroizing;

use crate::core::Result;

/// Deliberately not serializable; secrets belong only to the backend.
pub struct SecretString(Zeroizing<String>);

impl SecretString {
    pub fn new(value: String) -> Self {
        Self(Zeroizing::new(value))
    }

    pub fn expose_secret(&self) -> &str {
        &self.0
    }
}

impl fmt::Debug for SecretString {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("SecretString([REDACTED])")
    }
}

#[derive(Debug, Clone)]
pub struct PlayerProfile {
    pub id: Uuid,
    pub name: String,
}

#[derive(Debug)]
pub struct Session {
    pub provider_id: String,
    pub profile: PlayerProfile,
    pub access_token: SecretString,
    pub client_token: Option<SecretString>,
}

#[derive(Debug)]
pub enum AuthInput {
    Password {
        login: String,
        password: SecretString,
        totp: Option<SecretString>,
    },
    #[cfg(all(debug_assertions, feature = "dev-auth"))]
    DevelopmentNickname { nickname: String },
}

#[derive(Debug)]
pub enum AuthOutcome {
    Authenticated(Session),
    TotpRequired { challenge_id: Uuid },
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SessionValidity {
    Valid,
    ReauthenticationRequired,
}

#[derive(Debug)]
pub struct LaunchIdentity {
    pub profile: PlayerProfile,
    pub access_token: SecretString,
    pub user_type: String,
}

/// Providers must distinguish rejected credentials from transport failures and never log inputs.
pub trait AuthProvider: Send + Sync {
    fn id(&self) -> &str;
    fn capabilities(&self) -> ProviderCapabilities;
    fn authenticate(&self, input: AuthInput) -> impl Future<Output = Result<AuthOutcome>> + Send;
    fn refresh(&self, session: &Session) -> impl Future<Output = Result<Session>> + Send;
    fn validate(&self, session: &Session) -> impl Future<Output = Result<SessionValidity>> + Send;
    fn logout(&self, session: &Session) -> impl Future<Output = Result<()>> + Send;
    fn launch_identity(
        &self,
        session: &Session,
    ) -> impl Future<Output = Result<LaunchIdentity>> + Send;
}

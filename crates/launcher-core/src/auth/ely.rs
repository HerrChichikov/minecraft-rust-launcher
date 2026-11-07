use super::*;
use crate::core::LauncherError;

/// Reviewed endpoints; never read executable/provider URLs from profile JSON or IPC.
#[derive(Debug, Clone, Copy)]
pub struct ElyByConfiguration {
    pub provider_id: &'static str,
    pub authentication_base: &'static str,
    pub injector_api_root: &'static str,
}
impl Default for ElyByConfiguration {
    fn default() -> Self {
        Self {
            provider_id: "ely-by",
            authentication_base: "https://authserver.ely.by",
            injector_api_root: "https://account.ely.by/api/authlib-injector",
        }
    }
}

/// Prepared provider boundary. A product-approved real flow is required before enabling it.
#[derive(Debug, Default)]
pub struct ElyByAuthProvider;
impl AuthProvider for ElyByAuthProvider {
    fn id(&self) -> &str {
        "ely-by"
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            available: false,
            development: false,
            flows: vec![],
            unavailable_reason: Some("real_flow_not_enabled".into()),
        }
    }
    async fn authenticate(&self, _input: AuthInput) -> Result<AuthOutcome> {
        Err(LauncherError::AuthUnavailable)
    }
    async fn refresh(&self, _session: &Session) -> Result<Session> {
        Err(LauncherError::AuthUnavailable)
    }
    async fn validate(&self, _session: &Session) -> Result<SessionValidity> {
        Err(LauncherError::AuthUnavailable)
    }
    async fn logout(&self, _session: &Session) -> Result<()> {
        Err(LauncherError::AuthUnavailable)
    }
    async fn launch_identity(&self, _session: &Session) -> Result<LaunchIdentity> {
        Err(LauncherError::AuthUnavailable)
    }
}

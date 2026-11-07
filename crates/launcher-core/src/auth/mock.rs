use super::*;
use crate::core::LauncherError as Error;
use md5::{Digest, Md5};

#[derive(Debug, Default)]
pub struct MockAuthProvider;
impl MockAuthProvider {
    pub fn login(&self, nickname: &str) -> Result<Session> {
        if !(3..=16).contains(&nickname.len())
            || !nickname
                .bytes()
                .all(|c| c.is_ascii_alphanumeric() || c == b'_')
        {
            return Err(Error::InvalidInput {
                field: "development nickname",
            });
        }
        // Java UUID.nameUUIDFromBytes hashes these exact bytes, without a UUID namespace.
        let bytes: [u8; 16] = Md5::digest(format!("OfflinePlayer:{nickname}").as_bytes()).into();
        Ok(Session {
            provider_id: self.id().into(),
            profile: PlayerProfile {
                id: uuid::Builder::from_md5_bytes(bytes).into_uuid(),
                name: nickname.into(),
            },
            access_token: SecretString::new(format!("development-only-{}", Uuid::new_v4())),
            client_token: None,
        })
    }
    fn check(&self, session: &Session) -> Result<()> {
        if session.provider_id != self.id()
            || self.login(&session.profile.name)?.profile.id != session.profile.id
        {
            return Err(Error::AuthenticationRequired);
        }
        Ok(())
    }
    pub(crate) fn identity(&self, session: &Session) -> Result<LaunchIdentity> {
        self.check(session)?;
        Ok(LaunchIdentity {
            profile: session.profile.clone(),
            access_token: SecretString::new(session.access_token.expose_secret().into()),
            user_type: "mojang".into(),
        })
    }
}
impl AuthProvider for MockAuthProvider {
    fn id(&self) -> &str {
        "development-mock"
    }
    fn capabilities(&self) -> ProviderCapabilities {
        ProviderCapabilities {
            id: self.id().into(),
            available: true,
            development: true,
            flows: vec!["development_nickname".into()],
            unavailable_reason: None,
        }
    }
    async fn authenticate(&self, input: AuthInput) -> Result<AuthOutcome> {
        match input {
            AuthInput::DevelopmentNickname { nickname } => {
                self.login(&nickname).map(AuthOutcome::Authenticated)
            }
            _ => Err(Error::UnsupportedAuthFlow),
        }
    }
    async fn validate(&self, session: &Session) -> Result<SessionValidity> {
        self.check(session)?;
        Ok(SessionValidity::Valid)
    }
    async fn refresh(&self, session: &Session) -> Result<Session> {
        self.check(session)?;
        self.login(&session.profile.name)
    }
    async fn logout(&self, session: &Session) -> Result<()> {
        self.check(session)
    }
    async fn launch_identity(&self, session: &Session) -> Result<LaunchIdentity> {
        self.identity(session)
    }
}

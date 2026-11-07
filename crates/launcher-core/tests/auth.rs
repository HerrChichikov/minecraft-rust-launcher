use launcher_core::{
    auth::{AuthInput, AuthProvider, AuthService, ElyByAuthProvider, SecretString},
    core::{LauncherError as Error, Result},
};

#[test]
fn production_registry_and_configuration_cannot_enable_mock() -> Result<()> {
    let mut service = AuthService::default();
    assert!(service.snapshot().account.is_none());
    assert!(matches!(
        service.launch_identity(),
        Err(Error::AuthenticationRequired)
    ));
    for id in ["ely-by", "mock", "offline", "unknown", "Development-Mock"] {
        assert!(matches!(
            service.login(0, id, "Notch"),
            Err(Error::AuthUnavailable)
        ));
        assert_eq!(service.snapshot().revision, 0);
    }
    #[cfg(not(all(debug_assertions, feature = "dev-auth")))]
    {
        assert_eq!(service.snapshot().providers.len(), 1);
        assert!(matches!(
            service.login(0, "development-mock", "Notch"),
            Err(Error::AuthUnavailable)
        ));
    }
    assert!(matches!(service.logout(99), Err(Error::RevisionConflict)));
    Ok(())
}
#[test]
fn real_provider_rejects_invalid_input_before_transmitting_credentials() {
    let provider = ElyByAuthProvider::default();
    assert!(provider.capabilities().available);
    let input = AuthInput::Password {
        login: "".into(),
        password: SecretString::new("never-transmit-password".into()),
        totp: None,
    };
    assert!(!format!("{input:?}").contains("never-transmit-password"));
    assert!(matches!(
        provider.authenticate(input),
        Err(Error::InvalidInput { .. })
    ));
}
#[cfg(all(debug_assertions, feature = "dev-auth"))]
mod development {
    use super::*;
    use launcher_core::auth::{AuthOutcome, MockAuthProvider, SessionValidity};
    #[test]
    fn java_uuid_vectors_case_and_provider_lifecycle() -> Result<()> {
        let provider = MockAuthProvider;
        let AuthOutcome::Authenticated(session) =
            provider.authenticate(AuthInput::DevelopmentNickname {
                nickname: "Notch".into(),
            })?
        else {
            return Err(Error::AuthenticationRequired);
        };
        assert_eq!(
            session.profile.id.to_string(),
            "b50ad385-829d-3141-a216-7e7d7539ba7f"
        );
        assert_eq!(provider.login("Notch")?.profile.id, session.profile.id);
        assert_ne!(provider.login("notch")?.profile.id, session.profile.id);
        assert_eq!(provider.validate(&session)?, SessionValidity::Valid);
        let fresh = provider.refresh(&session)?;
        assert_eq!(fresh.profile.id, session.profile.id);
        assert_ne!(
            fresh.access_token.expose_secret(),
            session.access_token.expose_secret()
        );
        assert_eq!(provider.launch_identity(&fresh)?.user_type, "mojang");
        provider.logout(&fresh)?;
        for name in [
            "",
            "ab",
            "abcdefghijklmnopq",
            "a b",
            "Иван",
            "test\n",
            "a/b",
            "a:b",
        ] {
            assert!(provider.login(name).is_err());
        }
        assert!(provider.login("Valid_123").is_ok());
        Ok(())
    }
    #[test]
    fn owned_session_revisions_logout_and_redaction() -> Result<()> {
        let mut service = AuthService::default();
        let snapshot = service.login(0, "development-mock", "Notch")?;
        let wire = serde_json::to_string(&snapshot).map_err(|_| Error::Metadata)?;
        assert!(wire.contains("development-mock"));
        assert!(!wire.contains("token"));
        assert!(!wire.contains("development-only-"));
        let identity = service.launch_identity()?;
        assert!(!format!("{service:?}").contains(identity.access_token.expose_secret()));
        assert!(service.login(0, "development-mock", "Another").is_err());
        assert!(service.login(1, "development-mock", "bad!").is_err());
        assert_eq!(service.launch_identity()?.profile.name, "Notch");
        assert!(service.logout(1)?.account.is_none());
        assert!(matches!(
            service.launch_identity(),
            Err(Error::AuthenticationRequired)
        ));
        assert!(AuthService::default().snapshot().account.is_none());
        Ok(())
    }
}

use launcher_core::{
    auth::{AuthService, PlatformClient, SecretString},
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
    for id in [
        "platform",
        "ely-by",
        "mock",
        "offline",
        "unknown",
        "Development-Mock",
    ] {
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
    let Ok(client) = PlatformClient::new("https://auth.example.test") else {
        panic!("valid platform URL");
    };
    let mut service = AuthService::with_platform(client, None);
    let password = SecretString::new("never-transmit-password".into());
    assert!(!format!("{password:?}").contains("never-transmit-password"));
    assert!(matches!(
        service.login_password_with_remember(0, "", password, false),
        Err(Error::InvalidInput { .. })
    ));
    for url in [
        "http://auth.example.test",
        "https://other.example.test@auth.example.test",
        "https://auth.example.test/path",
        "https://auth.example.test/?query=1",
    ] {
        assert!(PlatformClient::new(url).is_err());
    }
}
#[cfg(all(debug_assertions, feature = "dev-auth"))]
mod development {
    use super::*;
    use launcher_core::auth::{
        AuthInput, AuthOutcome, AuthProvider, MockAuthProvider, SessionValidity,
    };
    #[test]
    fn java_uuid_vectors_case_and_provider_lifecycle() -> Result<()> {
        let provider = MockAuthProvider;
        let AuthOutcome::Authenticated(session) =
            provider.authenticate(AuthInput::DevelopmentNickname {
                nickname: "Notch".into(),
            })?;
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

#[cfg(debug_assertions)]
#[test]
#[ignore = "requires an isolated mine_go_auth service with two test profiles and a registration invite"]
fn platform_lifecycle_against_mine_go_auth() -> Result<()> {
    use launcher_core::{auth::CREDENTIAL_REFERENCE, storage::CredentialStore};
    use std::sync::{Arc, Mutex};

    #[derive(Debug, Default)]
    struct TestStore(Mutex<Option<SecretString>>);
    impl CredentialStore for TestStore {
        fn read(&self, reference: &str) -> Result<Option<SecretString>> {
            assert_eq!(reference, CREDENTIAL_REFERENCE);
            let record = self
                .0
                .lock()
                .map_err(|_| Error::CredentialStoreUnavailable)?;
            Ok(record
                .as_ref()
                .map(|s| SecretString::new(s.expose_secret().into())))
        }
        fn write(&self, reference: &str, secret: &SecretString) -> Result<()> {
            assert_eq!(reference, CREDENTIAL_REFERENCE);
            *self
                .0
                .lock()
                .map_err(|_| Error::CredentialStoreUnavailable)? =
                Some(SecretString::new(secret.expose_secret().into()));
            Ok(())
        }
        fn delete(&self, reference: &str) -> Result<()> {
            assert_eq!(reference, CREDENTIAL_REFERENCE);
            *self
                .0
                .lock()
                .map_err(|_| Error::CredentialStoreUnavailable)? = None;
            Ok(())
        }
    }
    fn wire(value: &launcher_core::auth::AuthSnapshot) -> Result<String> {
        serde_json::to_string(value).map_err(|_| Error::AuthProtocol)
    }

    let url = std::env::var("MINECRAFT_AUTH_SMOKE_ORIGIN").map_err(|_| Error::AuthUnavailable)?;
    let store = Arc::new(TestStore::default());
    let password = || SecretString::new("launcher-smoke-test-password".into());
    #[cfg(feature = "dev-auth")]
    let ca = std::env::var_os("MINECRAFT_AUTH_SMOKE_CA")
        .map(std::fs::read)
        .transpose()
        .map_err(|_| Error::AuthUnavailable)?;
    #[cfg(feature = "dev-auth")]
    if let Some(cert) = &ca {
        for origin in [
            "https://auth.example.test",
            "http://127.0.0.1",
            "https://localhost",
        ] {
            assert!(matches!(
                PlatformClient::with_test_ca(origin, cert),
                Err(Error::AuthUnavailable)
            ));
        }
        let mut strict = AuthService::with_platform(PlatformClient::new(&url)?, None);
        assert!(matches!(
            strict.login_password_with_remember(0, "SmokeAccount", password(), false),
            Err(Error::AuthNetwork)
        ));
    }
    let client = || {
        #[cfg(feature = "dev-auth")]
        if let Some(cert) = &ca {
            return PlatformClient::with_test_ca(&url, cert);
        }
        PlatformClient::new(&url)
    };
    let mut service = AuthService::with_platform(client()?, Some(store.clone()));
    let registered = service.register(0, "SmokeRegistration", password(), "", "SMOKEREG")?;
    assert_eq!(registered.notice, Some("registered"));
    let session = service.login_password_with_remember(
        registered.revision,
        "SmokeAccount",
        password(),
        true,
    )?;
    assert_eq!(session.persistence, "system_credential_store");
    assert_eq!(session.game_profiles.len(), 2);
    assert_eq!(session.selected_game_profile_id, None);
    assert!(matches!(
        service.launch_identity(),
        Err(Error::GameProfileUnavailable)
    ));
    let profile = session
        .game_profiles
        .iter()
        .find(|p| p.realm == "survival")
        .ok_or(Error::GameProfileUnavailable)?;
    let selected = service.select_profile(session.revision, profile.id)?;
    let old = store
        .read(CREDENTIAL_REFERENCE)?
        .ok_or(Error::SavedSessionMissing)?;
    assert!(!old.expose_secret().contains(password().expose_secret()));
    let refreshed = service.refresh(selected.revision)?;
    let new = store
        .read(CREDENTIAL_REFERENCE)?
        .ok_or(Error::SavedSessionMissing)?;
    assert_ne!(old.expose_secret(), new.expose_secret());
    let identity = service.launch_identity()?;
    assert_eq!(identity.profile.id, profile.id);
    assert_eq!(identity.profile.name, profile.nickname);
    assert!(identity.access_token.expose_secret().starts_with("mc_"));
    assert_eq!(
        identity.agent_api.as_ref().map(reqwest::Url::path),
        Some("/yggdrasil/survival/")
    );
    assert!(!wire(&refreshed)?.contains(identity.access_token.expose_secret()));
    assert!(!format!("{identity:?}").contains(identity.access_token.expose_secret()));
    drop(service);

    let mut service = AuthService::with_platform(client()?, Some(store.clone()));
    let restored = service.restore(0)?;
    assert_eq!(restored.persistence, "system_credential_store");
    let account = restored.account.ok_or(Error::AuthenticationRequired)?;
    assert_eq!(account.player_name, "SmokeAccount");
    assert!(service.logout(restored.revision)?.account.is_none());
    assert!(store.read(CREDENTIAL_REFERENCE)?.is_none());
    assert!(matches!(
        service.restore(restored.revision + 1),
        Err(Error::SavedSessionMissing)
    ));
    Ok(())
}

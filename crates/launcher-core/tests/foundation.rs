use std::{ffi::OsString, path::PathBuf};

use launcher_core::{
    application::ApplicationService,
    auth::{PlayerProfile, SecretString, Session},
    core::Result,
    launcher::LaunchSpec,
};
use uuid::Uuid;

#[test]
fn foundation_does_not_advertise_unimplemented_features() -> Result<()> {
    let snapshot = ApplicationService.snapshot()?;
    assert_eq!(snapshot.target_minecraft_version, "1.21.1");
    assert!(snapshot.features.profiles);
    assert!(snapshot.features.installation);
    assert!(snapshot.features.authentication);
    assert!(!snapshot.features.game_launch);
    Ok(())
}

#[test]
fn session_debug_does_not_expose_either_token() {
    let session = Session {
        provider_id: "test-provider".into(),
        profile: PlayerProfile {
            id: Uuid::nil(),
            name: "Player".into(),
        },
        access_token: SecretString::new("sensitive-access-token".into()),
        client_token: Some(SecretString::new("sensitive-client-token".into())),
    };
    let diagnostics = format!("{session:?}");
    assert!(!diagnostics.contains("sensitive-access-token"));
    assert!(!diagnostics.contains("sensitive-client-token"));
    assert!(diagnostics.contains("[REDACTED]"));
}

#[test]
fn launch_spec_debug_cannot_leak_argv_or_user_paths() {
    let spec = LaunchSpec {
        executable: PathBuf::from("/private/user/java"),
        arguments: vec![
            OsString::from("--accessToken"),
            OsString::from("private-token"),
        ],
        working_directory: PathBuf::from("/private/user/game"),
    };
    let diagnostics = format!("{spec:?}");
    assert!(!diagnostics.contains("private-token"));
    assert!(!diagnostics.contains("/private/user"));
}

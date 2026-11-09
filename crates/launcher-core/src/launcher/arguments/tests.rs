use super::*;
use crate::{
    auth::{PlayerProfile, SecretString},
    core::{Architecture, Platform},
    installer::PreparedInstallation,
    profiles::SettingsPolicy,
    storage::{AppPaths, WorkspaceService},
};

#[test]
fn official_metadata_builds_argv_for_all_four_targets() -> Result<()> {
    let temp = tempfile::tempdir()?;
    let root = temp.path().canonicalize()?;
    let policy = SettingsPolicy::new(8192)?;
    let workspace = WorkspaceService::open(
        AppPaths {
            data: root.join("data with spaces"),
            configuration: root.join("config"),
            cache: root.join("cache"),
            logs: root.join("logs"),
        },
        policy,
    )?;
    let snapshot = workspace.snapshot()?;
    let profile = snapshot.profiles.first().ok_or(Error::Metadata)?.clone();
    let store = workspace.installation_store(snapshot.root_id, profile.id)?;
    let value: serde_json::Value =
        serde_json::from_str(include_str!("../../../../../docs/research/1.21.1.json"))
            .map_err(|_| Error::Metadata)?;
    let identity = LaunchIdentity {
        profile: PlayerProfile {
            id: uuid::Uuid::from_u128(123),
            name: "Example".into(),
        },
        access_token: SecretString::new("test-token".into()),
        user_type: "mojang".into(),
        requires_ely_by_agent: true,
    };
    for (os, architecture) in [
        (OperatingSystem::Windows, Architecture::X86_64),
        (OperatingSystem::Linux, Architecture::X86_64),
        (OperatingSystem::Macos, Architecture::X86_64),
        (OperatingSystem::Macos, Architecture::Aarch64),
    ] {
        let platform = Platform { os, architecture };
        let context = metadata::RuleContext::new(platform, "15.0".into());
        let plan = metadata::version_plan(
            value.clone(),
            "82ba6501da174a665c1787775f946487422f2432",
            &context,
        )?;
        let installed = PreparedInstallation {
            schema_version: 1,
            profile_id: profile.id,
            metadata_sha1: plan.metadata_sha1.clone(),
            asset_index_id: format!("{}-{}", plan.asset_index_id, plan.asset_index.sha1),
            classpath: plan.classpath.clone(),
            java_executable: String::new(),
            platform,
            downloaded_bytes: 0,
            reused_bytes: 0,
        };
        let launch = PreparedLaunch {
            store: std::sync::Arc::clone(&store),
            scratch: super::super::NativeScratch::create(&store, profile.id)?,
            context,
            plan,
            installed,
            profile: profile.clone(),
            agent: None,
            working_directory: root.join("game with spaces"),
            natives_directory: root.join("natives"),
            logging_config: root.join("console.xml"),
        };
        let args = build(&launch, &identity)?;
        let value = |flag: &str| {
            args.windows(2)
                .find(|pair| pair[0] == flag)
                .map(|pair| pair[1].as_os_str())
        };
        let cp = value("-cp")
            .and_then(|path| path.to_str())
            .ok_or(Error::Metadata)?;
        let separator = if os == OperatingSystem::Windows {
            ';'
        } else {
            ':'
        };
        assert_eq!(cp.split(separator).count(), launch.plan.classpath.len());
        assert!(cp.ends_with("/client.jar"));
        assert!(
            cp.len() < 24_000,
            "classpath exceeds Windows command line budget"
        );
        assert_eq!(
            args.iter().any(|a| a == "-XstartOnFirstThread"),
            os == OperatingSystem::Macos
        );
        assert!(args.iter().any(|a| a == "net.minecraft.client.main.Main"));
        assert!(args.iter().any(|a| a == "-Xms1024M"));
        assert!(args.iter().any(|a| a == "-Xmx4096M"));
        assert_eq!(
            value("--uuid"),
            Some(std::ffi::OsStr::new("0000000000000000000000000000007b"))
        );
        assert_eq!(
            value("--accessToken"),
            Some(std::ffi::OsStr::new("test-token"))
        );
        assert_eq!(value("--xuid"), Some(std::ffi::OsStr::new("")));
        assert_eq!(
            value("--gameDir"),
            Some(execution_path(&launch.working_directory)?.as_os_str())
        );
        assert!(value("--demo").is_none());
        assert!(!args.iter().any(|a| a.to_string_lossy().contains("${")));
    }
    Ok(())
}

#[test]
fn substitutes_once_and_rejects_unresolved_variables() -> Result<()> {
    let values = BTreeMap::from([("path", "a b/${literal}".into())]);
    assert_eq!(
        substitute("-Dpath=${path}", &values)?,
        "-Dpath=a b/${literal}"
    );
    assert!(substitute("${unknown}", &values).is_err());
    assert!(substitute("${path", &values).is_err());
    assert!(classpath(&["shared/file;extra".into()], ";").is_err());
    Ok(())
}

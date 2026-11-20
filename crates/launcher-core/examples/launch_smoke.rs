//! Opt-in graphical development smoke. Give it a dedicated installer smoke root.
#[cfg(all(debug_assertions, feature = "dev-auth"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use launcher_core::{
        auth::{AuthService, PlatformClient, SecretString},
        core::{EventSink, OperationEvent, Platform},
        installer::{Cancellation, InstallEngine, metadata::RuleContext},
        launcher::PreparedLaunch,
        profiles::SettingsPolicy,
        runtime::RuntimeSelection,
        storage::{AppPaths, WorkspaceService},
    };
    use std::{fs, io, path::PathBuf, time::Duration};

    struct IgnoreProgress;
    impl EventSink for IgnoreProgress {
        fn publish(&self, _: OperationEvent) {}
    }

    let mut args = std::env::args_os().skip(1);
    let base = PathBuf::from(
        args.next()
            .ok_or("usage: launch_smoke ABSOLUTE_TEST_ROOT [STOP_SECONDS]")?,
    );
    let timeout = args
        .next()
        .map(|value| value.to_string_lossy().parse::<u64>())
        .transpose()?;
    let workspace = WorkspaceService::open(
        AppPaths {
            data: base.join("data"),
            configuration: base.join("config"),
            cache: base.join("cache"),
            logs: base.join("logs"),
        },
        SettingsPolicy::new(4096)?,
    )?;
    let snapshot = workspace.snapshot()?;
    let mut profile = snapshot
        .profiles
        .into_iter()
        .find(|profile| profile.id == snapshot.active_profile_id)
        .ok_or("profile missing")?;
    if let Some(executable) = std::env::var_os("MINECRAFT_AUTH_SMOKE_JAVA_WRAPPER") {
        profile.settings.runtime = RuntimeSelection::Custom {
            executable: executable.into(),
        };
    }
    let store = workspace.installation_store(snapshot.root_id, profile.id)?;
    let cancel = Cancellation::default();
    let engine = InstallEngine::new(store, profile.id, cancel.clone())?;
    let mut auth = match std::env::var("MINECRAFT_AUTH_SMOKE_ORIGIN") {
        Ok(origin) => {
            let client = match std::env::var_os("MINECRAFT_AUTH_SMOKE_CA") {
                Some(path) => PlatformClient::with_test_ca(&origin, &fs::read(path)?)?,
                None => PlatformClient::new(&origin)?,
            };
            let mut service = AuthService::with_platform(client, None);
            let mut password = String::new();
            io::stdin().read_line(&mut password)?;
            password.truncate(password.trim_end_matches(['\r', '\n']).len());
            let snapshot = service.login_password_with_remember(
                0,
                "SmokeAccount",
                SecretString::new(password),
                false,
            )?;
            let realm =
                std::env::var("MINECRAFT_AUTH_SMOKE_REALM").unwrap_or_else(|_| "survival".into());
            let profile = snapshot
                .game_profiles
                .iter()
                .find(|p| p.realm == realm)
                .ok_or("smoke server profile missing")?;
            service.select_profile(snapshot.revision, profile.id)?;
            service
        }
        Err(std::env::VarError::NotPresent) => {
            let mut service = AuthService::default();
            service.login(0, "development-mock", "LauncherSmoke")?;
            service
        }
        Err(error) => return Err(error.into()),
    };
    let (_, needs_agent) = auth.launch_requirement()?;
    let context = RuleContext::new(Platform::current_host()?, String::new());
    let prepared = PreparedLaunch::prepare(
        &engine,
        profile,
        snapshot.policy,
        context,
        &IgnoreProgress,
        needs_agent,
    )?;
    drop(engine);
    let identity = auth.launch_identity()?;
    let revoke = needs_agent && std::env::var_os("MINECRAFT_AUTH_SMOKE_REVOKE").is_some();
    if revoke {
        let signed_out = auth.logout(auth.snapshot().revision)?;
        if signed_out.notice.is_some() {
            return Err("could not confirm test token revocation".into());
        }
    }
    let game = prepared.spawn(identity, &cancel)?;
    if needs_agent {
        println!("Minecraft process started (mine_go_auth + Authlib Injector)");
    } else {
        println!("Minecraft process started (development mock)");
    }
    if let Some(timeout) = timeout {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(timeout));
            cancel.cancel();
        });
    }
    let completed = game.wait(&cancel)?;
    if needs_agent && !revoke {
        auth.logout(auth.snapshot().revision)?;
    }
    println!("{completed:?}");
    if !completed.log_saved
        || completed.recovery_required
        || !(completed.exit.success || completed.exit.stopped)
    {
        return Err("game failed or diagnostic output could not be saved".into());
    }
    Ok(())
}

#[cfg(not(all(debug_assertions, feature = "dev-auth")))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    Err("launch_smoke requires debug with --features dev-auth".into())
}

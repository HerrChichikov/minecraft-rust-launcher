//! Opt-in graphical development smoke. Give it a dedicated installer smoke root.
#[cfg(all(debug_assertions, feature = "dev-auth"))]
fn main() -> Result<(), Box<dyn std::error::Error>> {
    use launcher_core::{
        auth::AuthService,
        core::{EventSink, OperationEvent, Platform},
        installer::{Cancellation, InstallEngine, metadata::RuleContext},
        launcher::PreparedLaunch,
        profiles::SettingsPolicy,
        storage::{AppPaths, WorkspaceService},
    };
    use std::{path::PathBuf, time::Duration};

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
    let profile = snapshot
        .profiles
        .into_iter()
        .find(|profile| profile.id == snapshot.active_profile_id)
        .ok_or("profile missing")?;
    let store = workspace.installation_store(snapshot.root_id, profile.id)?;
    let cancel = Cancellation::default();
    let engine = InstallEngine::new(store, profile.id, cancel.clone())?;
    let mut auth = AuthService::default();
    auth.login(0, "development-mock", "LauncherSmoke")?;
    let context = RuleContext::new(Platform::current_host()?, String::new());
    let prepared = PreparedLaunch::prepare(
        &engine,
        profile,
        snapshot.policy,
        context,
        &IgnoreProgress,
        false,
    )?;
    drop(engine);
    let game = prepared.spawn(auth.launch_identity()?, &cancel)?;
    println!("Minecraft process started (development mock)");
    if let Some(timeout) = timeout {
        let cancel = cancel.clone();
        std::thread::spawn(move || {
            std::thread::sleep(Duration::from_secs(timeout));
            cancel.cancel();
        });
    }
    let completed = game.wait(&cancel)?;
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

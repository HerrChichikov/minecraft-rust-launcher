fn main() -> Result<(), Box<dyn std::error::Error>> {
    println!("cargo:rerun-if-env-changed=CARGO_FEATURE_DEV_AUTH");
    println!("cargo:rerun-if-env-changed=PROFILE");

    if std::env::var_os("CARGO_FEATURE_DEV_AUTH").is_some()
        && std::env::var("PROFILE").as_deref() == Ok("release")
    {
        return Err(
            "LAUNCHER_DEV_AUTH_FORBIDDEN: dev-auth is forbidden in a release profile".into(),
        );
    }

    tauri_build::try_build(
        tauri_build::Attributes::new()
            .app_manifest(tauri_build::AppManifest::new().commands(&["get_snapshot"])),
    )?;
    Ok(())
}

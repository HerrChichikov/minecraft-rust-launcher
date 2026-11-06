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

    let mut attributes =
        tauri_build::Attributes::new().app_manifest(tauri_build::AppManifest::new().commands(&[
            "start_install",
            "get_installation",
            "cancel_install",
            "get_snapshot",
            "get_workspace",
            "create_profile",
            "select_profile",
            "save_profile",
            "switch_data_root",
        ]));

    if std::env::var("CARGO_CFG_TARGET_OS").as_deref() == Ok("windows") {
        let manifest = std::path::PathBuf::from(std::env::var("CARGO_MANIFEST_DIR")?)
            .join("windows-app-manifest.xml");
        println!("cargo:rerun-if-changed={}", manifest.display());
        // Tauri's resource embedding covers the app binary only. Link the manifest into
        // test executables too, so Common Controls v6 imports resolve before tests start.
        attributes = attributes
            .windows_attributes(tauri_build::WindowsAttributes::new_without_app_manifest());
        println!("cargo:rustc-link-arg=/MANIFEST:EMBED");
        println!("cargo:rustc-link-arg=/MANIFESTINPUT:{}", manifest.display());
    }

    tauri_build::try_build(attributes)?;
    Ok(())
}

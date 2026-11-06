use tauri::{
    ipc::{CallbackFn, InvokeBody},
    test::{INVOKE_KEY, MockRuntime, get_ipc_response, mock_builder},
    webview::InvokeRequest,
};

fn request(command: &str, origin: &str) -> Result<InvokeRequest, Box<dyn std::error::Error>> {
    Ok(InvokeRequest {
        cmd: command.into(),
        callback: CallbackFn(0),
        error: CallbackFn(1),
        url: origin.parse()?,
        body: InvokeBody::default(),
        headers: Default::default(),
        invoke_key: INVOKE_KEY.into(),
    })
}

fn app() -> tauri::Result<tauri::App<MockRuntime>> {
    super::configure(mock_builder()).build(super::context())
}

fn bundled_origin() -> &'static str {
    if cfg!(windows) {
        "http://tauri.localhost"
    } else {
        "tauri://localhost"
    }
}

#[test]
fn main_window_can_read_the_real_application_snapshot() -> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    let response = get_ipc_response(&main, request("get_snapshot", bundled_origin())?)
        .map_err(|value| value.to_string())?;
    let snapshot: serde_json::Value = response.deserialize()?;
    assert_eq!(snapshot["phase"], "foundation");
    assert_eq!(snapshot["features"]["gameLaunch"], false);
    assert_eq!(snapshot["developmentBuild"], cfg!(debug_assertions));
    Ok(())
}

#[test]
fn other_windows_and_remote_origins_cannot_read_application_state()
-> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let secondary =
        tauri::WebviewWindowBuilder::new(&app, "untrusted", Default::default()).build()?;
    assert!(get_ipc_response(&secondary, request("get_snapshot", bundled_origin())?).is_err());

    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    assert!(get_ipc_response(&main, request("get_snapshot", "https://example.com")?).is_err());
    Ok(())
}

#[test]
fn auth_launch_and_filesystem_commands_are_unavailable() -> Result<(), Box<dyn std::error::Error>> {
    let app = app()?;
    let main = tauri::WebviewWindowBuilder::new(&app, "main", Default::default()).build()?;
    for command in [
        "login",
        "mock_login",
        "play",
        "plugin:fs|read_text_file",
        "plugin:shell|execute",
    ] {
        assert!(get_ipc_response(&main, request(command, bundled_origin())?).is_err());
    }
    Ok(())
}

//! Tauri adapters only. Domain services live in launcher-core.

mod commands;

use launcher_core::application::ApplicationService;
use tauri::Runtime;

fn configure<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    configure_with_storage(builder, commands::workspace::WorkspaceState::default())
}

fn configure_with_storage<R: Runtime>(
    builder: tauri::Builder<R>,
    storage: commands::workspace::WorkspaceState,
) -> tauri::Builder<R> {
    builder
        .manage(ApplicationService)
        .manage(storage)
        .manage(commands::installation::InstallationState::default())
        .manage(commands::auth::AuthState::default())
        .invoke_handler(tauri::generate_handler![
            commands::snapshot::get_snapshot,
            commands::workspace::get_workspace,
            commands::workspace::create_profile,
            commands::workspace::select_profile,
            commands::workspace::save_profile,
            commands::workspace::switch_data_root,
            commands::installation::start_install,
            commands::installation::get_installation,
            commands::installation::cancel_install,
            commands::auth::get_auth,
            commands::auth::login,
            commands::auth::logout
        ])
}

pub fn run() -> tauri::Result<()> {
    configure(tauri::Builder::default()).run(context())
}

fn context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg(test)]
mod tests;

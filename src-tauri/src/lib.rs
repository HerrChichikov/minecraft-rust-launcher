//! Tauri adapters only. Domain services live in launcher-core.

mod commands;

use launcher_core::application::ApplicationService;
use tauri::Runtime;

fn configure<R: Runtime>(builder: tauri::Builder<R>) -> tauri::Builder<R> {
    builder
        .manage(ApplicationService)
        .invoke_handler(tauri::generate_handler![commands::snapshot::get_snapshot])
}

pub fn run() -> tauri::Result<()> {
    configure(tauri::Builder::default()).run(context())
}

fn context<R: Runtime>() -> tauri::Context<R> {
    tauri::generate_context!()
}

#[cfg(test)]
mod tests;

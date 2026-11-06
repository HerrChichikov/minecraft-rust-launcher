#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

fn main() {
    if let Err(error) = launcher_desktop::run() {
        eprintln!("Launcher could not start: {error}");
        std::process::exit(1);
    }
}

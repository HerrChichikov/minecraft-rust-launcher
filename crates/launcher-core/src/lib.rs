//! Desktop-independent storage, installation, authentication and launch services.

#[cfg(all(feature = "dev-auth", not(debug_assertions)))]
compile_error!("LAUNCHER_DEV_AUTH_FORBIDDEN: dev-auth cannot be compiled without debug assertions");

pub mod application;
pub mod auth;
pub mod core;
pub mod installer;
pub mod launcher;
pub mod profiles;
pub mod runtime;
pub mod storage;

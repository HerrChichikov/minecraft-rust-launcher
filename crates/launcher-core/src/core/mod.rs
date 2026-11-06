//! Shared error, platform, and event contracts.

mod error;
mod events;
mod platform;

pub use error::{LauncherError, Result};
pub use events::{EventSink, OperationEvent, OperationId, OperationStage};
pub use platform::{Architecture, OperatingSystem, Platform};

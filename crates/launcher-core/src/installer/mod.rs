//! Metadata-derived installation boundary; downloading and repair belong to Phase 3.

/// Quick currently performs the same full SHA1 verification as Full. A fingerprint
/// shortcut can be added after profiling; neither mode trusts file existence.
#[derive(Debug, Clone, Copy)]
pub enum VerificationMode {
    Quick,
    Full,
}

pub mod metadata;
mod network;
pub use network::{Cancellation, Http};

mod engine;
pub use engine::{InstallEngine, PreparedInstallation};

mod java;

#[cfg(test)]
mod transfer_tests;

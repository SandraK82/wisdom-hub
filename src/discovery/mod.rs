//! Hub discovery module
//!
//! Implements hub registration, heartbeat, and federation.
//! Will be fully implemented in Phase 5.

mod registry;
mod client;
mod federation;

pub use registry::*;
pub use client::*;
pub use federation::*;

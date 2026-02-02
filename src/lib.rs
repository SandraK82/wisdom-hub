//! Wisdom Hub - High-Performance Federation Hub
//!
//! A Rust-based hub for the Wisdom Network, providing:
//! - Entity storage (Agents, Fragments, Relations, Tags, Transforms)
//! - Trust path calculation
//! - Hub discovery and federation
//! - REST and gRPC APIs

pub mod config;
pub mod models;
pub mod crypto;
pub mod store;
pub mod services;
pub mod trust;
pub mod discovery;
pub mod api;
pub mod metrics;
pub mod resources;

/// Generated protobuf types for gRPC
#[path = "wisdom.hub.v1.rs"]
#[allow(clippy::all)]
pub mod proto;

// Re-export commonly used types
pub use config::Settings;
pub use models::{Agent, Fragment, Relation, Tag, Transform, HubError, HubResult};
pub use crypto::{KeyPair, sign, verify};
pub use store::{RocksStore, EntityStore, Cursor, ListResult};
pub use services::EntityService;
pub use trust::{TrustPathFinder, TrustCalculator};
pub use discovery::{HubRegistry, DiscoveryClient, FederatedSearch};
pub use resources::{ResourceMonitor, ResourceLevel, ResourceStatus, HubStatusSummary};

/// Version of the wisdom-hub
pub const VERSION: &str = env!("CARGO_PKG_VERSION");

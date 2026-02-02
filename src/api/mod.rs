//! API module for the Wisdom Hub
//!
//! Provides both REST (Actix-Web) and gRPC (tonic) APIs.

mod rest;
mod grpc;
mod health;
mod responses;

pub use rest::*;
pub use grpc::*;
pub use health::*;
pub use responses::*;

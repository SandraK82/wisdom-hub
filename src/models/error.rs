//! Error types for the Wisdom Hub

use thiserror::Error;

/// Hub errors
#[derive(Debug, Error)]
pub enum HubError {
    #[error("Entity not found: {entity_type} with id {id}")]
    NotFound { entity_type: String, id: String },

    #[error("Entity already exists: {entity_type} with id {id}")]
    AlreadyExists { entity_type: String, id: String },

    #[error("Invalid signature for entity: {entity_type}")]
    InvalidSignature { entity_type: String },

    #[error("Invalid content hash")]
    InvalidContentHash,

    #[error("Invalid public key: {0}")]
    InvalidPublicKey(String),

    #[error("Crypto error: {0}")]
    CryptoError(String),

    #[error("Database error: {0}")]
    DatabaseError(String),

    #[error("Serialization error: {0}")]
    SerializationError(String),

    #[error("Configuration error: {0}")]
    ConfigError(String),

    #[error("Network error: {0}")]
    NetworkError(String),

    #[error("Trust path not found from {from} to {to}")]
    TrustPathNotFound { from: String, to: String },

    #[error("Federation error: {0}")]
    FederationError(String),

    #[error("Rate limit exceeded")]
    RateLimitExceeded,

    #[error("Unauthorized: {0}")]
    Unauthorized(String),

    #[error("Validation error: {0}")]
    ValidationError(String),

    #[error("Internal error: {0}")]
    Internal(String),

    #[error("Resource limit exceeded: {0}")]
    ResourceLimitExceeded(String),
}

impl HubError {
    pub fn not_found(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        HubError::NotFound {
            entity_type: entity_type.into(),
            id: id.into(),
        }
    }

    pub fn already_exists(entity_type: impl Into<String>, id: impl Into<String>) -> Self {
        HubError::AlreadyExists {
            entity_type: entity_type.into(),
            id: id.into(),
        }
    }

    pub fn invalid_signature(entity_type: impl Into<String>) -> Self {
        HubError::InvalidSignature {
            entity_type: entity_type.into(),
        }
    }
}

// Convert from standard library errors
impl From<std::io::Error> for HubError {
    fn from(err: std::io::Error) -> Self {
        HubError::Internal(err.to_string())
    }
}

impl From<serde_json::Error> for HubError {
    fn from(err: serde_json::Error) -> Self {
        HubError::SerializationError(err.to_string())
    }
}

impl From<config::ConfigError> for HubError {
    fn from(err: config::ConfigError) -> Self {
        HubError::ConfigError(err.to_string())
    }
}

/// Result type for hub operations
pub type HubResult<T> = Result<T, HubError>;

//! Common API response types

use actix_web::{HttpResponse, http::StatusCode};
use serde::Serialize;

use crate::models::HubError;
use crate::resources::HubStatusSummary;

/// Standard API response wrapper
#[derive(Debug, Serialize)]
pub struct ApiResponse<T: Serialize> {
    pub success: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub data: Option<T>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub error: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub hub_status: Option<HubStatusSummary>,
}

impl<T: Serialize> ApiResponse<T> {
    /// Create a success response
    pub fn success(data: T) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            hub_status: None,
        }
    }

    /// Create a success response with hub status
    pub fn success_with_status(data: T, hub_status: Option<HubStatusSummary>) -> Self {
        Self {
            success: true,
            data: Some(data),
            error: None,
            hub_status,
        }
    }

    /// Create an error response
    pub fn error(message: impl Into<String>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
            hub_status: None,
        }
    }

    /// Create an error response with hub status
    pub fn error_with_status(message: impl Into<String>, hub_status: Option<HubStatusSummary>) -> ApiResponse<()> {
        ApiResponse {
            success: false,
            data: None,
            error: Some(message.into()),
            hub_status,
        }
    }
}

/// Paginated response
#[derive(Debug, Serialize)]
pub struct PaginatedResponse<T: Serialize> {
    pub items: Vec<T>,
    pub total: usize,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub next_cursor: Option<String>,
}

impl<T: Serialize> PaginatedResponse<T> {
    pub fn new(items: Vec<T>, total: usize, next_cursor: Option<String>) -> Self {
        Self {
            items,
            total,
            next_cursor,
        }
    }
}

/// Convert HubError to HTTP response
impl From<HubError> for HttpResponse {
    fn from(error: HubError) -> Self {
        let (status, message) = match &error {
            HubError::NotFound { .. } => (StatusCode::NOT_FOUND, error.to_string()),
            HubError::AlreadyExists { .. } => (StatusCode::CONFLICT, error.to_string()),
            HubError::InvalidSignature { .. } => (StatusCode::BAD_REQUEST, error.to_string()),
            HubError::InvalidContentHash => (StatusCode::BAD_REQUEST, error.to_string()),
            HubError::InvalidPublicKey(_) => (StatusCode::BAD_REQUEST, error.to_string()),
            HubError::CryptoError(_) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            HubError::DatabaseError(_) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            HubError::SerializationError(_) => (StatusCode::BAD_REQUEST, error.to_string()),
            HubError::ConfigError(_) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            HubError::NetworkError(_) => (StatusCode::BAD_GATEWAY, error.to_string()),
            HubError::TrustPathNotFound { .. } => (StatusCode::NOT_FOUND, error.to_string()),
            HubError::FederationError(_) => (StatusCode::BAD_GATEWAY, error.to_string()),
            HubError::RateLimitExceeded => (StatusCode::TOO_MANY_REQUESTS, error.to_string()),
            HubError::Unauthorized(_) => (StatusCode::UNAUTHORIZED, error.to_string()),
            HubError::Internal(_) => (StatusCode::INTERNAL_SERVER_ERROR, error.to_string()),
            HubError::ValidationError(_) => (StatusCode::BAD_REQUEST, error.to_string()),
            HubError::ResourceLimitExceeded(_) => (StatusCode::SERVICE_UNAVAILABLE, error.to_string()),
        };

        HttpResponse::build(status).json(ApiResponse::<()>::error(message))
    }
}

/// Result type that can be converted to HttpResponse
pub type ApiResult<T> = Result<T, HubError>;

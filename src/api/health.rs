//! Health check endpoint

use actix_web::{get, HttpResponse, web};
use serde::Serialize;
use chrono::{DateTime, Utc};

/// Health check response
#[derive(Debug, Serialize)]
pub struct HealthResponse {
    pub status: String,
    pub version: String,
    pub hub_id: String,
    pub timestamp: DateTime<Utc>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub uptime_seconds: Option<f64>,
}

impl HealthResponse {
    pub fn healthy(hub_id: &str, version: &str) -> Self {
        Self {
            status: "healthy".to_string(),
            version: version.to_string(),
            hub_id: hub_id.to_string(),
            timestamp: Utc::now(),
            uptime_seconds: None,
        }
    }

    pub fn with_uptime(mut self, uptime: f64) -> Self {
        self.uptime_seconds = Some(uptime);
        self
    }
}

/// Shared application state
#[derive(Clone)]
pub struct AppState {
    pub hub_id: String,
    pub version: String,
    pub start_time: DateTime<Utc>,
}

impl AppState {
    pub fn new(hub_id: impl Into<String>) -> Self {
        Self {
            hub_id: hub_id.into(),
            version: env!("CARGO_PKG_VERSION").to_string(),
            start_time: Utc::now(),
        }
    }

    pub fn uptime_seconds(&self) -> f64 {
        let duration = Utc::now().signed_duration_since(self.start_time);
        duration.num_milliseconds() as f64 / 1000.0
    }
}

/// Health check endpoint
#[get("/health")]
pub async fn health_check(state: web::Data<AppState>) -> HttpResponse {
    let response = HealthResponse::healthy(&state.hub_id, &state.version)
        .with_uptime(state.uptime_seconds());

    HttpResponse::Ok().json(response)
}

/// Readiness check endpoint
#[get("/ready")]
pub async fn readiness_check(state: web::Data<AppState>) -> HttpResponse {
    // For now, just return healthy
    // Later we can check database connection, etc.
    let response = HealthResponse::healthy(&state.hub_id, &state.version);
    HttpResponse::Ok().json(response)
}

/// Liveness check endpoint
#[get("/live")]
pub async fn liveness_check() -> HttpResponse {
    HttpResponse::Ok().json(serde_json::json!({
        "status": "alive"
    }))
}

/// Configure health routes
pub fn configure_health_routes(cfg: &mut web::ServiceConfig) {
    cfg.service(health_check)
        .service(readiness_check)
        .service(liveness_check);
}

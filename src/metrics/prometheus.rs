//! Prometheus metrics integration
//!
//! Will be fully implemented in Phase 7.

use actix_web::{get, HttpResponse};
use once_cell::sync::Lazy;
use prometheus::{Encoder, TextEncoder, IntCounter, IntGauge, Histogram, HistogramOpts, opts, register_int_counter, register_int_gauge, register_histogram};

// Define metrics
static HTTP_REQUESTS_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        opts!("http_requests_total", "Total number of HTTP requests")
    )
    .expect("Failed to create HTTP requests counter")
});

static HTTP_REQUEST_DURATION: Lazy<Histogram> = Lazy::new(|| {
    register_histogram!(HistogramOpts::new(
        "http_request_duration_seconds",
        "HTTP request duration in seconds"
    ))
    .expect("Failed to create HTTP request duration histogram")
});

static ENTITIES_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        opts!("entities_total", "Total number of entities in storage")
    )
    .expect("Failed to create entities gauge")
});

static AGENTS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        opts!("agents_total", "Total number of agents")
    )
    .expect("Failed to create agents gauge")
});

static FRAGMENTS_TOTAL: Lazy<IntGauge> = Lazy::new(|| {
    register_int_gauge!(
        opts!("fragments_total", "Total number of fragments")
    )
    .expect("Failed to create fragments gauge")
});

static TRUST_PATH_QUERIES_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        opts!("trust_path_queries_total", "Total number of trust path queries")
    )
    .expect("Failed to create trust path queries counter")
});

static FEDERATED_SEARCHES_TOTAL: Lazy<IntCounter> = Lazy::new(|| {
    register_int_counter!(
        opts!("federated_searches_total", "Total number of federated searches")
    )
    .expect("Failed to create federated searches counter")
});

/// Initialize all metrics
pub fn init_metrics() {
    // Force lazy initialization
    Lazy::force(&HTTP_REQUESTS_TOTAL);
    Lazy::force(&HTTP_REQUEST_DURATION);
    Lazy::force(&ENTITIES_TOTAL);
    Lazy::force(&AGENTS_TOTAL);
    Lazy::force(&FRAGMENTS_TOTAL);
    Lazy::force(&TRUST_PATH_QUERIES_TOTAL);
    Lazy::force(&FEDERATED_SEARCHES_TOTAL);
}

/// Record an HTTP request
pub fn record_http_request() {
    HTTP_REQUESTS_TOTAL.inc();
}

/// Record HTTP request duration
pub fn record_request_duration(duration_secs: f64) {
    HTTP_REQUEST_DURATION.observe(duration_secs);
}

/// Set total entities count
pub fn set_entities_total(count: i64) {
    ENTITIES_TOTAL.set(count);
}

/// Set agents count
pub fn set_agents_total(count: i64) {
    AGENTS_TOTAL.set(count);
}

/// Set fragments count
pub fn set_fragments_total(count: i64) {
    FRAGMENTS_TOTAL.set(count);
}

/// Record a trust path query
pub fn record_trust_path_query() {
    TRUST_PATH_QUERIES_TOTAL.inc();
}

/// Record a federated search
pub fn record_federated_search() {
    FEDERATED_SEARCHES_TOTAL.inc();
}

/// Prometheus metrics endpoint
#[get("/metrics")]
pub async fn metrics_endpoint() -> HttpResponse {
    let encoder = TextEncoder::new();
    let metric_families = prometheus::gather();

    let mut buffer = Vec::new();
    if let Err(e) = encoder.encode(&metric_families, &mut buffer) {
        return HttpResponse::InternalServerError().body(format!("Failed to encode metrics: {}", e));
    }

    HttpResponse::Ok()
        .content_type("text/plain; version=0.0.4; charset=utf-8")
        .body(buffer)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_metrics_initialization() {
        init_metrics();

        // Just verify they can be incremented
        record_http_request();
        record_request_duration(0.1);
        set_entities_total(100);
    }
}

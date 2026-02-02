//! Wisdom Hub - Main Entry Point
//!
//! Starts the HTTP and gRPC servers for the Wisdom Hub.

use actix_web::{web, App, HttpServer, middleware};
use std::sync::Arc;
use tonic::transport::Server as TonicServer;
use tracing::{info, error};
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt, EnvFilter};

use wisdom_hub::api::{configure_routes, create_grpc_service, AppState, ApiState};
use wisdom_hub::config::Settings;
use wisdom_hub::metrics::{init_metrics, metrics_endpoint};
use wisdom_hub::resources::ResourceMonitor;
use wisdom_hub::services::{EntityService, DiscoveryConfig};
use wisdom_hub::store::{RocksStore, EntityStore};

#[actix_web::main]
async fn main() -> std::io::Result<()> {
    // Initialize logging with RUST_LOG environment variable support
    // Default: info level for wisdom_hub, warn for everything else
    let env_filter = EnvFilter::try_from_default_env()
        .unwrap_or_else(|_| EnvFilter::new("warn,wisdom_hub=info"));

    tracing_subscriber::registry()
        .with(env_filter)
        .with(tracing_subscriber::fmt::layer()
            .with_target(true)
            .with_thread_ids(true)
            .with_file(true)
            .with_line_number(true))
        .init();

    // Load configuration
    let settings = Settings::load().unwrap_or_else(|e| {
        tracing::warn!("Failed to load config: {}, using defaults", e);
        Settings::default()
    });

    info!(
        "Starting Wisdom Hub v{} ({})",
        env!("CARGO_PKG_VERSION"),
        settings.hub.hub_id
    );
    info!("Role: {:?}", settings.hub.role);
    info!("HTTP: {}:{}", settings.server.host, settings.server.http_port);
    info!("gRPC: {}:{}", settings.server.host, settings.server.grpc_port);

    // Initialize metrics
    init_metrics();

    // Initialize database
    let rocks_store = RocksStore::open_with_opts(
        &settings.database.data_dir,
        settings.database.cache_size_mb,
        settings.database.compression,
    )
    .expect("Failed to open database");

    info!("Database initialized at: {}", settings.database.data_dir);

    // Create entity store and service
    let entity_store = Arc::new(EntityStore::new(rocks_store));
    let entity_service = Arc::new(EntityService::new(Arc::clone(&entity_store)));

    // Create discovery configuration
    let heartbeat_timeout = settings.discovery.registration_interval_sec
        * settings.discovery.heartbeat_timeout_multiplier as u64;

    let discovery_config = DiscoveryConfig {
        role: settings.hub.role,
        hub_id: settings.hub.hub_id.clone(),
        public_url: settings.hub.public_url.clone(),
        primary_hub_url: settings.discovery.primary_hub_url.clone(),
        heartbeat_timeout_sec: heartbeat_timeout,
        registration_interval_sec: settings.discovery.registration_interval_sec,
        hub_list_refresh_sec: settings.discovery.hub_list_refresh_sec,
    };

    // Initialize resource monitor
    let resource_monitor = Arc::new(ResourceMonitor::new(settings.resources.clone()));
    resource_monitor.update_status(); // Initial status check
    let monitor_handle = Arc::clone(&resource_monitor).start_monitoring();
    info!(
        "Resource monitor started (warning: {}%, critical: {}%)",
        settings.resources.warning_threshold,
        settings.resources.critical_threshold
    );

    // Create application state for HTTP server
    let app_state = AppState::new(&settings.hub.hub_id);
    let api_state = ApiState::new(Arc::clone(&entity_store), discovery_config, Arc::clone(&resource_monitor));

    // Create gRPC service
    let grpc_service = create_grpc_service(Arc::clone(&entity_service), Arc::clone(&entity_store));

    // Start gRPC server in a separate task
    let grpc_addr = format!("{}:{}", settings.server.host, settings.server.grpc_port);
    let grpc_addr_parsed = grpc_addr.parse().expect("Invalid gRPC address");

    info!("Starting gRPC server on {}", grpc_addr);

    // Spawn gRPC server as a background task
    actix_web::rt::spawn(async move {
        if let Err(e) = TonicServer::builder()
            .add_service(grpc_service)
            .serve(grpc_addr_parsed)
            .await
        {
            error!("gRPC server error: {}", e);
        }
    });

    // Start HTTP server
    let http_addr = format!("{}:{}", settings.server.host, settings.server.http_port);
    info!("Starting HTTP server on {}", http_addr);

    let server = HttpServer::new(move || {
        App::new()
            // Add shared state
            .app_data(web::Data::new(app_state.clone()))
            .app_data(web::Data::new(api_state.clone()))
            // Add middleware
            .wrap(middleware::Logger::default())
            .wrap(middleware::Compress::default())
            // Add routes
            .configure(configure_routes)
            // Add metrics endpoint
            .service(metrics_endpoint)
    })
    .workers(settings.server.workers)
    .bind(&http_addr)?
    .run();

    // Keep the monitor handle alive for the lifetime of the server
    let _monitor_handle = monitor_handle;

    server.await
}

//! Hub configuration settings

use config::{Config, ConfigError, Environment, File};
use serde::Deserialize;
use std::path::Path;

/// Main hub configuration
#[derive(Debug, Clone, Deserialize)]
pub struct Settings {
    pub hub: HubSettings,
    pub server: ServerSettings,
    pub database: DatabaseSettings,
    pub discovery: DiscoverySettings,
    pub trust: TrustSettings,
    pub metrics: MetricsSettings,
    #[serde(default)]
    pub resources: ResourceSettings,
}

/// Hub identity settings
#[derive(Debug, Clone, Deserialize)]
pub struct HubSettings {
    /// Hub role: "primary" or "secondary"
    pub role: HubRole,
    /// Unique hub identifier (UUID)
    pub hub_id: String,
    /// Public URL for this hub
    pub public_url: String,
    /// Path to Ed25519 private key file
    pub private_key_path: Option<String>,
    /// Hub capabilities
    #[serde(default = "default_capabilities")]
    pub capabilities: Vec<String>,
}

fn default_capabilities() -> Vec<String> {
    vec![
        "entities".to_string(),
        "trust".to_string(),
        "search".to_string(),
    ]
}

/// Hub role
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HubRole {
    Primary,
    Secondary,
}

impl Default for HubRole {
    fn default() -> Self {
        HubRole::Secondary
    }
}

/// Server settings
#[derive(Debug, Clone, Deserialize)]
pub struct ServerSettings {
    /// HTTP server host
    #[serde(default = "default_host")]
    pub host: String,
    /// HTTP server port
    #[serde(default = "default_http_port")]
    pub http_port: u16,
    /// gRPC server port
    #[serde(default = "default_grpc_port")]
    pub grpc_port: u16,
    /// Number of worker threads
    #[serde(default = "default_workers")]
    pub workers: usize,
}

fn default_host() -> String {
    "0.0.0.0".to_string()
}

fn default_http_port() -> u16 {
    8080
}

fn default_grpc_port() -> u16 {
    50051
}

fn default_workers() -> usize {
    num_cpus::get()
}

/// Database settings
#[derive(Debug, Clone, Deserialize)]
pub struct DatabaseSettings {
    /// RocksDB data directory
    #[serde(default = "default_data_dir")]
    pub data_dir: String,
    /// Enable compression
    #[serde(default = "default_true")]
    pub compression: bool,
    /// Cache size in MB
    #[serde(default = "default_cache_size")]
    pub cache_size_mb: usize,
}

fn default_data_dir() -> String {
    "./data".to_string()
}

fn default_cache_size() -> usize {
    256
}

fn default_true() -> bool {
    true
}

/// Discovery settings
#[derive(Debug, Clone, Deserialize)]
pub struct DiscoverySettings {
    /// Enable hub discovery
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Primary hub URL (for secondary hubs)
    pub primary_hub_url: Option<String>,
    /// Registration interval in seconds
    #[serde(default = "default_registration_interval")]
    pub registration_interval_sec: u64,
    /// Hub list refresh interval in seconds
    #[serde(default = "default_hub_list_refresh")]
    pub hub_list_refresh_sec: u64,
    /// Heartbeat timeout multiplier (times registration_interval)
    #[serde(default = "default_heartbeat_timeout_multiplier")]
    pub heartbeat_timeout_multiplier: u32,
}

fn default_registration_interval() -> u64 {
    300 // 5 minutes
}

fn default_hub_list_refresh() -> u64 {
    60 // 1 minute
}

fn default_heartbeat_timeout_multiplier() -> u32 {
    3
}

/// Trust calculation settings
#[derive(Debug, Clone, Deserialize)]
pub struct TrustSettings {
    /// Maximum trust path depth
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
    /// Trust damping factor per hop
    #[serde(default = "default_damping_factor")]
    pub damping_factor: f32,
    /// Minimum effective trust threshold
    #[serde(default = "default_min_trust")]
    pub min_trust_threshold: f32,
}

fn default_max_depth() -> u8 {
    5
}

fn default_damping_factor() -> f32 {
    0.8
}

fn default_min_trust() -> f32 {
    0.01
}

/// Metrics settings
#[derive(Debug, Clone, Deserialize)]
pub struct MetricsSettings {
    /// Enable Prometheus metrics
    #[serde(default = "default_true")]
    pub enabled: bool,
    /// Metrics endpoint path
    #[serde(default = "default_metrics_path")]
    pub path: String,
}

fn default_metrics_path() -> String {
    "/metrics".to_string()
}

/// Resource monitoring settings
#[derive(Debug, Clone, Deserialize)]
pub struct ResourceSettings {
    /// Warning threshold percentage (default: 60)
    #[serde(default = "default_warning_threshold")]
    pub warning_threshold: u8,
    /// Critical threshold percentage (default: 80)
    #[serde(default = "default_critical_threshold")]
    pub critical_threshold: u8,
    /// Path to monitor for disk usage (default: data directory)
    pub monitor_path: Option<String>,
    /// Check interval in seconds (default: 60)
    #[serde(default = "default_check_interval")]
    pub check_interval_sec: u64,
    /// Project URL for hints
    #[serde(default = "default_project_url")]
    pub project_url: String,
}

fn default_warning_threshold() -> u8 {
    60
}

fn default_critical_threshold() -> u8 {
    80
}

fn default_check_interval() -> u64 {
    60
}

fn default_project_url() -> String {
    "https://github.com/SandraK82/wisdom-hub".to_string()
}

impl Default for ResourceSettings {
    fn default() -> Self {
        ResourceSettings {
            warning_threshold: default_warning_threshold(),
            critical_threshold: default_critical_threshold(),
            monitor_path: None,
            check_interval_sec: default_check_interval(),
            project_url: default_project_url(),
        }
    }
}

impl Settings {
    /// Load settings from file and environment
    pub fn load() -> Result<Self, ConfigError> {
        Self::load_from("config")
    }

    /// Load settings from a specific config file path (without extension)
    pub fn load_from<P: AsRef<Path>>(path: P) -> Result<Self, ConfigError> {
        let config_path = path.as_ref();

        let builder = Config::builder()
            // Start with default values
            .set_default("hub.role", "secondary")?
            .set_default("hub.hub_id", uuid::Uuid::new_v4().to_string())?
            .set_default("hub.public_url", "http://localhost:8080")?
            .set_default("server.host", "0.0.0.0")?
            .set_default("server.http_port", 8080)?
            .set_default("server.grpc_port", 50051)?
            .set_default("server.workers", num_cpus::get() as i64)?
            .set_default("database.data_dir", "./data")?
            .set_default("database.compression", true)?
            .set_default("database.cache_size_mb", 256)?
            .set_default("discovery.enabled", true)?
            .set_default("discovery.registration_interval_sec", 300)?
            .set_default("discovery.hub_list_refresh_sec", 60)?
            .set_default("discovery.heartbeat_timeout_multiplier", 3)?
            .set_default("trust.max_depth", 5)?
            .set_default("trust.damping_factor", 0.8)?
            .set_default("trust.min_trust_threshold", 0.01)?
            .set_default("metrics.enabled", true)?
            .set_default("metrics.path", "/metrics")?
            // Add config file if it exists
            .add_source(File::with_name(config_path.to_str().unwrap_or("config")).required(false))
            // Add environment variables with prefix WISDOM_HUB_
            .add_source(Environment::with_prefix("WISDOM_HUB").separator("__"));

        builder.build()?.try_deserialize()
    }
}

impl Default for Settings {
    fn default() -> Self {
        Settings::load().unwrap_or_else(|_| {
            // Provide minimal defaults if config loading fails
            Settings {
                hub: HubSettings {
                    role: HubRole::Secondary,
                    hub_id: uuid::Uuid::new_v4().to_string(),
                    public_url: "http://localhost:8080".to_string(),
                    private_key_path: None,
                    capabilities: default_capabilities(),
                },
                server: ServerSettings {
                    host: default_host(),
                    http_port: default_http_port(),
                    grpc_port: default_grpc_port(),
                    workers: default_workers(),
                },
                database: DatabaseSettings {
                    data_dir: default_data_dir(),
                    compression: true,
                    cache_size_mb: default_cache_size(),
                },
                discovery: DiscoverySettings {
                    enabled: true,
                    primary_hub_url: None,
                    registration_interval_sec: default_registration_interval(),
                    hub_list_refresh_sec: default_hub_list_refresh(),
                    heartbeat_timeout_multiplier: default_heartbeat_timeout_multiplier(),
                },
                trust: TrustSettings {
                    max_depth: default_max_depth(),
                    damping_factor: default_damping_factor(),
                    min_trust_threshold: default_min_trust(),
                },
                metrics: MetricsSettings {
                    enabled: true,
                    path: default_metrics_path(),
                },
                resources: ResourceSettings::default(),
            }
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_default_settings() {
        let settings = Settings::default();
        assert_eq!(settings.server.http_port, 8080);
        assert_eq!(settings.server.grpc_port, 50051);
        assert_eq!(settings.trust.max_depth, 5);
    }
}

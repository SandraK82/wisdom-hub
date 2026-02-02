//! Discovery service for hub registration and federation
//!
//! Manages hub registration for both primary and secondary hubs.

use std::sync::Arc;
use chrono::Utc;
use parking_lot::RwLock;
use tracing::{info, warn, error};

use crate::config::HubRole;
use crate::discovery::{HubRegistry, HubInfo, HubList, HubStats, HubStatus, DiscoveryClient};
use crate::models::{HubResult, HubError};
use crate::store::EntityStore;

/// Request to register a hub
#[derive(Debug, Clone, serde::Deserialize)]
pub struct RegisterHubRequest {
    pub hub_id: String,
    pub public_url: String,
    pub capabilities: Vec<String>,
    pub version: Option<String>,
    pub public_key: Option<String>,
}

/// Response from hub registration
#[derive(Debug, Clone, serde::Serialize)]
pub struct RegisterHubResponse {
    pub registered: bool,
    pub message: Option<String>,
    pub hub_list: Option<HubList>,
}

/// Request for heartbeat
#[derive(Debug, Clone, serde::Deserialize)]
pub struct HeartbeatRequest {
    pub hub_id: String,
    pub status: String,
    pub stats: HubStats,
}

/// Response from heartbeat
#[derive(Debug, Clone, serde::Serialize)]
pub struct HeartbeatResponse {
    pub acknowledged: bool,
    pub message: Option<String>,
}

/// Discovery service configuration
#[derive(Debug, Clone)]
pub struct DiscoveryConfig {
    /// Role of this hub (primary or secondary)
    pub role: HubRole,
    /// This hub's ID
    pub hub_id: String,
    /// This hub's public URL
    pub public_url: String,
    /// Primary hub URL (for secondary hubs)
    pub primary_hub_url: Option<String>,
    /// Heartbeat timeout in seconds
    pub heartbeat_timeout_sec: u64,
    /// Registration interval in seconds (for secondary hubs)
    pub registration_interval_sec: u64,
    /// Hub list refresh interval in seconds
    pub hub_list_refresh_sec: u64,
}

impl Default for DiscoveryConfig {
    fn default() -> Self {
        Self {
            role: HubRole::Primary,
            hub_id: "default-hub".to_string(),
            public_url: "http://localhost:8080".to_string(),
            primary_hub_url: None,
            heartbeat_timeout_sec: 900, // 15 minutes
            registration_interval_sec: 300, // 5 minutes
            hub_list_refresh_sec: 60, // 1 minute
        }
    }
}

/// Discovery service for managing hub federation
pub struct DiscoveryService {
    config: DiscoveryConfig,
    registry: Option<HubRegistry>,
    client: Option<DiscoveryClient>,
    store: Arc<EntityStore>,
    self_info: Arc<RwLock<HubInfo>>,
}

impl DiscoveryService {
    /// Create a new discovery service
    pub fn new(config: DiscoveryConfig, store: Arc<EntityStore>) -> Self {
        let (registry, client) = match config.role {
            HubRole::Primary => {
                // Primary hub has a registry, no client
                let registry = HubRegistry::new(config.heartbeat_timeout_sec);
                (Some(registry), None)
            }
            HubRole::Secondary => {
                // Secondary hub has a client, no registry
                let primary_url = config.primary_hub_url.as_ref()
                    .expect("Secondary hub requires primary_hub_url");
                let client = DiscoveryClient::new(
                    primary_url,
                    &config.hub_id,
                    &config.public_url,
                    vec!["entities".to_string(), "trust".to_string(), "search".to_string()],
                );
                (None, Some(client))
            }
        };

        let self_info = HubInfo {
            hub_id: config.hub_id.clone(),
            public_url: config.public_url.clone(),
            role: format!("{:?}", config.role).to_lowercase(),
            status: HubStatus::Healthy,
            last_seen: Utc::now(),
            capabilities: vec!["entities".to_string(), "trust".to_string(), "search".to_string()],
            stats: HubStats::default(),
            public_key: None,
        };

        Self {
            config,
            registry,
            client,
            store,
            self_info: Arc::new(RwLock::new(self_info)),
        }
    }

    /// Get current stats for this hub
    pub fn get_stats(&self) -> HubStats {
        let agents_count = self.store.count_agents().unwrap_or(0);
        let fragments_count = self.store.count_fragments().unwrap_or(0);
        HubStats {
            entities_count: agents_count + fragments_count,
            agents_count,
            fragments_count,
            uptime_seconds: 0.0, // TODO: Track actual uptime
        }
    }

    /// Update self info with current stats
    pub fn update_self_info(&self) {
        let stats = self.get_stats();
        let mut info = self.self_info.write();
        info.stats = stats;
        info.last_seen = Utc::now();
    }

    // ========================================================================
    // Primary Hub Operations
    // ========================================================================

    /// Register a hub (primary hub only)
    pub fn register_hub(&self, req: RegisterHubRequest) -> HubResult<RegisterHubResponse> {
        let registry = self.registry.as_ref()
            .ok_or_else(|| HubError::FederationError("Not a primary hub".to_string()))?;

        info!("Registering hub: {} at {}", req.hub_id, req.public_url);

        let hub_info = HubInfo {
            hub_id: req.hub_id,
            public_url: req.public_url,
            role: "secondary".to_string(),
            status: HubStatus::Healthy,
            last_seen: Utc::now(),
            capabilities: req.capabilities,
            stats: HubStats::default(),
            public_key: req.public_key,
        };

        registry.register(hub_info);

        // Return current hub list
        let hub_list = registry.list();

        Ok(RegisterHubResponse {
            registered: true,
            message: Some("Hub registered successfully".to_string()),
            hub_list: Some(hub_list),
        })
    }

    /// Process heartbeat from a hub (primary hub only)
    pub fn process_heartbeat(&self, req: HeartbeatRequest) -> HubResult<HeartbeatResponse> {
        let registry = self.registry.as_ref()
            .ok_or_else(|| HubError::FederationError("Not a primary hub".to_string()))?;

        let success = registry.heartbeat(&req.hub_id, req.stats);

        if success {
            Ok(HeartbeatResponse {
                acknowledged: true,
                message: None,
            })
        } else {
            warn!("Heartbeat from unknown hub: {}", req.hub_id);
            Ok(HeartbeatResponse {
                acknowledged: false,
                message: Some("Hub not registered".to_string()),
            })
        }
    }

    /// Get list of all known hubs (primary hub only)
    pub fn get_known_hubs(&self) -> HubResult<HubList> {
        if let Some(ref registry) = self.registry {
            // Primary hub: return from registry
            let mut list = registry.list();

            // Add self to the list
            let self_info = self.self_info.read().clone();
            list.hubs.insert(0, self_info);

            Ok(list)
        } else if let Some(ref client) = self.client {
            // Secondary hub: return cached list
            client.get_cached_hub_list()
                .ok_or_else(|| HubError::FederationError("Hub list not available".to_string()))
        } else {
            Err(HubError::FederationError("Discovery not configured".to_string()))
        }
    }

    /// Check for inactive hubs (primary hub only)
    pub fn check_inactive_hubs(&self) {
        if let Some(ref registry) = self.registry {
            registry.check_inactive();
        }
    }

    // ========================================================================
    // Secondary Hub Operations
    // ========================================================================

    /// Register with primary hub (secondary hub only)
    pub async fn register_with_primary(&self, public_key: Option<&str>) -> HubResult<HubList> {
        let client = self.client.as_ref()
            .ok_or_else(|| HubError::FederationError("Not a secondary hub".to_string()))?;

        info!("Registering with primary hub: {}", self.config.primary_hub_url.as_ref().unwrap());

        let result = client.register(public_key).await;

        match &result {
            Ok(_) => info!("Successfully registered with primary hub"),
            Err(e) => error!("Failed to register with primary hub: {}", e),
        }

        result
    }

    /// Send heartbeat to primary hub (secondary hub only)
    pub async fn send_heartbeat(&self) -> HubResult<()> {
        let client = self.client.as_ref()
            .ok_or_else(|| HubError::FederationError("Not a secondary hub".to_string()))?;

        let stats = self.get_stats();
        client.heartbeat(stats).await
    }

    /// Refresh hub list from primary (secondary hub only)
    pub async fn refresh_hub_list(&self) -> HubResult<HubList> {
        let client = self.client.as_ref()
            .ok_or_else(|| HubError::FederationError("Not a secondary hub".to_string()))?;

        client.refresh_hub_list().await
    }

    /// Check if registration is needed (secondary hub only)
    pub fn needs_registration(&self) -> bool {
        self.client.as_ref()
            .map(|c| c.needs_registration(self.config.registration_interval_sec))
            .unwrap_or(false)
    }

    /// Get other healthy hubs for federation
    pub fn get_federation_targets(&self) -> Vec<HubInfo> {
        if let Some(ref client) = self.client {
            client.get_other_hubs()
        } else if let Some(ref registry) = self.registry {
            registry.list_healthy()
        } else {
            vec![]
        }
    }

    // ========================================================================
    // Getters
    // ========================================================================

    /// Check if this is a primary hub
    pub fn is_primary(&self) -> bool {
        matches!(self.config.role, HubRole::Primary)
    }

    /// Get this hub's ID
    pub fn hub_id(&self) -> &str {
        &self.config.hub_id
    }

    /// Get this hub's info
    pub fn self_info(&self) -> HubInfo {
        self.self_info.read().clone()
    }
}

impl Clone for DiscoveryService {
    fn clone(&self) -> Self {
        Self {
            config: self.config.clone(),
            registry: self.registry.clone(),
            client: self.client.clone(),
            store: Arc::clone(&self.store),
            self_info: Arc::clone(&self.self_info),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::RocksStore;
    use tempfile::tempdir;

    fn setup_primary_service() -> DiscoveryService {
        let dir = tempdir().unwrap();
        let rocks = RocksStore::open(dir.path().to_str().unwrap()).unwrap();
        let store = Arc::new(EntityStore::new(rocks));

        let config = DiscoveryConfig {
            role: HubRole::Primary,
            hub_id: "primary-hub".to_string(),
            public_url: "https://primary.example.com".to_string(),
            ..Default::default()
        };

        DiscoveryService::new(config, store)
    }

    #[test]
    fn test_register_hub() {
        let service = setup_primary_service();

        let req = RegisterHubRequest {
            hub_id: "secondary-1".to_string(),
            public_url: "https://secondary1.example.com".to_string(),
            capabilities: vec!["entities".to_string()],
            version: Some("0.1.0".to_string()),
            public_key: None,
        };

        let response = service.register_hub(req).unwrap();
        assert!(response.registered);
        assert!(response.hub_list.is_some());

        let hub_list = response.hub_list.unwrap();
        assert_eq!(hub_list.hubs.len(), 1);
    }

    #[test]
    fn test_heartbeat() {
        let service = setup_primary_service();

        // First register
        let req = RegisterHubRequest {
            hub_id: "secondary-1".to_string(),
            public_url: "https://secondary1.example.com".to_string(),
            capabilities: vec!["entities".to_string()],
            version: None,
            public_key: None,
        };
        service.register_hub(req).unwrap();

        // Then heartbeat
        let heartbeat_req = HeartbeatRequest {
            hub_id: "secondary-1".to_string(),
            status: "healthy".to_string(),
            stats: HubStats {
                entities_count: 100,
                agents_count: 10,
                fragments_count: 50,
                uptime_seconds: 3600.0,
            },
        };

        let response = service.process_heartbeat(heartbeat_req).unwrap();
        assert!(response.acknowledged);
    }

    #[test]
    fn test_get_known_hubs() {
        let service = setup_primary_service();

        // Register some hubs
        for i in 1..=3 {
            let req = RegisterHubRequest {
                hub_id: format!("secondary-{}", i),
                public_url: format!("https://secondary{}.example.com", i),
                capabilities: vec!["entities".to_string()],
                version: None,
                public_key: None,
            };
            service.register_hub(req).unwrap();
        }

        let hub_list = service.get_known_hubs().unwrap();
        // 3 secondary + 1 primary (self)
        assert_eq!(hub_list.hubs.len(), 4);
    }
}

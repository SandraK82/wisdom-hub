//! Hub registry for primary hub
//!
//! Will be fully implemented in Phase 5.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::sync::Arc;

/// Information about a registered hub
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubInfo {
    pub hub_id: String,
    pub public_url: String,
    pub role: String,
    pub status: HubStatus,
    pub last_seen: DateTime<Utc>,
    pub capabilities: Vec<String>,
    pub stats: HubStats,
    pub public_key: Option<String>,
}

/// Hub status
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum HubStatus {
    Healthy,
    Degraded,
    Inactive,
    Unknown,
}

impl Default for HubStatus {
    fn default() -> Self {
        HubStatus::Unknown
    }
}

/// Hub statistics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct HubStats {
    pub entities_count: u64,
    pub agents_count: u64,
    pub fragments_count: u64,
    pub uptime_seconds: f64,
}

/// Hub list response
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct HubList {
    pub hubs: Vec<HubInfo>,
    pub version: u64,
    pub updated_at: DateTime<Utc>,
}

/// Hub registry (for primary hub)
#[derive(Debug)]
pub struct HubRegistry {
    hubs: Arc<RwLock<HashMap<String, HubInfo>>>,
    version: Arc<RwLock<u64>>,
    heartbeat_timeout_sec: u64,
}

impl HubRegistry {
    /// Create a new hub registry
    pub fn new(heartbeat_timeout_sec: u64) -> Self {
        Self {
            hubs: Arc::new(RwLock::new(HashMap::new())),
            version: Arc::new(RwLock::new(0)),
            heartbeat_timeout_sec,
        }
    }

    /// Register a hub
    pub fn register(&self, hub: HubInfo) {
        let mut hubs = self.hubs.write();
        hubs.insert(hub.hub_id.clone(), hub);

        let mut version = self.version.write();
        *version += 1;
    }

    /// Update hub heartbeat
    pub fn heartbeat(&self, hub_id: &str, stats: HubStats) -> bool {
        let mut hubs = self.hubs.write();
        if let Some(hub) = hubs.get_mut(hub_id) {
            hub.last_seen = Utc::now();
            hub.status = HubStatus::Healthy;
            hub.stats = stats;
            true
        } else {
            false
        }
    }

    /// Get a hub by ID
    pub fn get(&self, hub_id: &str) -> Option<HubInfo> {
        let hubs = self.hubs.read();
        hubs.get(hub_id).cloned()
    }

    /// Get all hubs
    pub fn list(&self) -> HubList {
        let hubs = self.hubs.read();
        let version = *self.version.read();

        HubList {
            hubs: hubs.values().cloned().collect(),
            version,
            updated_at: Utc::now(),
        }
    }

    /// Get only healthy hubs
    pub fn list_healthy(&self) -> Vec<HubInfo> {
        let hubs = self.hubs.read();
        hubs.values()
            .filter(|h| h.status == HubStatus::Healthy)
            .cloned()
            .collect()
    }

    /// Check for inactive hubs and update their status
    pub fn check_inactive(&self) {
        let mut hubs = self.hubs.write();
        let now = Utc::now();
        let timeout = chrono::Duration::seconds(self.heartbeat_timeout_sec as i64);

        for hub in hubs.values_mut() {
            if now.signed_duration_since(hub.last_seen) > timeout {
                hub.status = HubStatus::Inactive;
            }
        }
    }

    /// Remove a hub
    pub fn remove(&self, hub_id: &str) -> bool {
        let mut hubs = self.hubs.write();
        let removed = hubs.remove(hub_id).is_some();

        if removed {
            let mut version = self.version.write();
            *version += 1;
        }

        removed
    }
}

impl Clone for HubRegistry {
    fn clone(&self) -> Self {
        Self {
            hubs: Arc::clone(&self.hubs),
            version: Arc::clone(&self.version),
            heartbeat_timeout_sec: self.heartbeat_timeout_sec,
        }
    }
}

impl Default for HubRegistry {
    fn default() -> Self {
        Self::new(900) // 15 minutes default timeout
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_hub_registry() {
        let registry = HubRegistry::new(60);

        let hub = HubInfo {
            hub_id: "test-hub".to_string(),
            public_url: "https://test.example.com".to_string(),
            role: "secondary".to_string(),
            status: HubStatus::Healthy,
            last_seen: Utc::now(),
            capabilities: vec!["entities".to_string()],
            stats: HubStats::default(),
            public_key: None,
        };

        registry.register(hub);

        let retrieved = registry.get("test-hub").unwrap();
        assert_eq!(retrieved.public_url, "https://test.example.com");

        let list = registry.list();
        assert_eq!(list.hubs.len(), 1);
    }

    #[test]
    fn test_heartbeat() {
        let registry = HubRegistry::new(60);

        let hub = HubInfo {
            hub_id: "test-hub".to_string(),
            public_url: "https://test.example.com".to_string(),
            role: "secondary".to_string(),
            status: HubStatus::Unknown,
            last_seen: Utc::now() - chrono::Duration::minutes(5),
            capabilities: vec![],
            stats: HubStats::default(),
            public_key: None,
        };

        registry.register(hub);

        let success = registry.heartbeat(
            "test-hub",
            HubStats {
                entities_count: 100,
                ..Default::default()
            },
        );

        assert!(success);

        let updated = registry.get("test-hub").unwrap();
        assert_eq!(updated.status, HubStatus::Healthy);
        assert_eq!(updated.stats.entities_count, 100);
    }
}

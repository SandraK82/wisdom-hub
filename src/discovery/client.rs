//! Hub discovery client for secondary hubs
//!
//! Will be fully implemented in Phase 5.

use chrono::{DateTime, Utc};
use parking_lot::RwLock;
use std::sync::Arc;
use std::time::Duration;

use super::{HubInfo, HubList, HubStats, HubStatus};
use crate::models::{HubError, HubResult};

/// Discovery client for secondary hubs
pub struct DiscoveryClient {
    primary_hub_url: String,
    hub_id: String,
    public_url: String,
    capabilities: Vec<String>,
    http_client: reqwest::Client,
    cached_hub_list: Arc<RwLock<Option<HubList>>>,
    last_registration: Arc<RwLock<Option<DateTime<Utc>>>>,
}

impl DiscoveryClient {
    /// Create a new discovery client
    pub fn new(
        primary_hub_url: impl Into<String>,
        hub_id: impl Into<String>,
        public_url: impl Into<String>,
        capabilities: Vec<String>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(30))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            primary_hub_url: primary_hub_url.into(),
            hub_id: hub_id.into(),
            public_url: public_url.into(),
            capabilities,
            http_client,
            cached_hub_list: Arc::new(RwLock::new(None)),
            last_registration: Arc::new(RwLock::new(None)),
        }
    }

    /// Register this hub with the primary hub
    pub async fn register(&self, public_key: Option<&str>) -> HubResult<HubList> {
        let url = format!("{}/api/v1/discovery/register", self.primary_hub_url);

        let body = serde_json::json!({
            "hub_id": self.hub_id,
            "public_url": self.public_url,
            "capabilities": self.capabilities,
            "version": env!("CARGO_PKG_VERSION"),
            "public_key": public_key,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(HubError::FederationError(format!(
                "Registration failed: {}",
                response.status()
            )));
        }

        #[derive(serde::Deserialize)]
        struct RegisterResponse {
            registered: bool,
            #[allow(dead_code)]
            message: Option<String>,
            hub_list: Option<HubList>,
        }

        let result: RegisterResponse = response
            .json()
            .await
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        if !result.registered {
            return Err(HubError::FederationError("Registration rejected".to_string()));
        }

        // Update cached hub list
        if let Some(ref list) = result.hub_list {
            *self.cached_hub_list.write() = Some(list.clone());
        }

        *self.last_registration.write() = Some(Utc::now());

        result
            .hub_list
            .ok_or_else(|| HubError::FederationError("No hub list in response".to_string()))
    }

    /// Send heartbeat to primary hub
    pub async fn heartbeat(&self, stats: HubStats) -> HubResult<()> {
        let url = format!("{}/api/v1/discovery/heartbeat", self.primary_hub_url);

        let body = serde_json::json!({
            "hub_id": self.hub_id,
            "status": "healthy",
            "stats": stats,
        });

        let response = self
            .http_client
            .post(&url)
            .json(&body)
            .send()
            .await
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(HubError::FederationError(format!(
                "Heartbeat failed: {}",
                response.status()
            )));
        }

        Ok(())
    }

    /// Refresh the hub list from primary hub
    pub async fn refresh_hub_list(&self) -> HubResult<HubList> {
        let url = format!("{}/api/v1/discovery/hubs", self.primary_hub_url);

        let response = self
            .http_client
            .get(&url)
            .send()
            .await
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(HubError::FederationError(format!(
                "Failed to get hub list: {}",
                response.status()
            )));
        }

        let list: HubList = response
            .json()
            .await
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        *self.cached_hub_list.write() = Some(list.clone());

        Ok(list)
    }

    /// Get cached hub list
    pub fn get_cached_hub_list(&self) -> Option<HubList> {
        self.cached_hub_list.read().clone()
    }

    /// Get other healthy hubs (excluding self)
    pub fn get_other_hubs(&self) -> Vec<HubInfo> {
        self.cached_hub_list
            .read()
            .as_ref()
            .map(|list| {
                list.hubs
                    .iter()
                    .filter(|h| h.hub_id != self.hub_id && h.status == HubStatus::Healthy)
                    .cloned()
                    .collect()
            })
            .unwrap_or_default()
    }

    /// Check if we need to re-register
    pub fn needs_registration(&self, registration_interval_sec: u64) -> bool {
        match *self.last_registration.read() {
            None => true,
            Some(last) => {
                let elapsed = Utc::now().signed_duration_since(last);
                elapsed.num_seconds() as u64 > registration_interval_sec
            }
        }
    }
}

impl Clone for DiscoveryClient {
    fn clone(&self) -> Self {
        Self {
            primary_hub_url: self.primary_hub_url.clone(),
            hub_id: self.hub_id.clone(),
            public_url: self.public_url.clone(),
            capabilities: self.capabilities.clone(),
            http_client: self.http_client.clone(),
            cached_hub_list: Arc::clone(&self.cached_hub_list),
            last_registration: Arc::clone(&self.last_registration),
        }
    }
}

impl std::fmt::Debug for DiscoveryClient {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("DiscoveryClient")
            .field("primary_hub_url", &self.primary_hub_url)
            .field("hub_id", &self.hub_id)
            .finish()
    }
}

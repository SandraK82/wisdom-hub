//! Federated Search Service
//!
//! Coordinates local search with federation to other hubs.

use std::sync::Arc;
use std::time::Duration;
use futures::future::join_all;
use serde::{Deserialize, Serialize};
use tracing::{debug, warn};

use crate::models::{Fragment, HubResult, HubError};
use crate::discovery::HubInfo;
use super::{EntityService, DiscoveryService};

/// Search result with source information
#[derive(Debug, Clone, Serialize)]
pub struct SearchResultItem {
    pub fragment: Fragment,
    pub source_hub_id: String,
    pub relevance_score: f64,
}

/// Source hub contribution
#[derive(Debug, Clone, Serialize)]
pub struct SearchSource {
    pub hub_id: String,
    pub count: usize,
}

/// Federated search response
#[derive(Debug, Clone, Serialize)]
pub struct FederatedSearchResponse {
    pub results: Vec<SearchResultItem>,
    pub sources: Vec<SearchSource>,
    pub federated: bool,
    pub total: usize,
}

/// Federated search service
pub struct FederatedSearchService {
    entity_service: Arc<EntityService>,
    discovery_service: Arc<DiscoveryService>,
    http_client: reqwest::Client,
    timeout: Duration,
}

impl FederatedSearchService {
    /// Create a new federated search service
    pub fn new(
        entity_service: Arc<EntityService>,
        discovery_service: Arc<DiscoveryService>,
    ) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            entity_service,
            discovery_service,
            http_client,
            timeout: Duration::from_secs(5),
        }
    }

    /// Perform a search, optionally federating to other hubs
    pub async fn search(
        &self,
        query: &str,
        limit: usize,
        federate: bool,
        min_results: Option<usize>,
    ) -> HubResult<FederatedSearchResponse> {
        let local_hub_id = self.discovery_service.hub_id().to_string();

        // First, perform local search
        let local_results = self.entity_service.search_fragments(query, limit)?;
        let local_count = local_results.len();

        debug!(
            "Local search for '{}' returned {} results",
            query, local_count
        );

        let mut all_results: Vec<SearchResultItem> = local_results
            .into_iter()
            .map(|fragment| SearchResultItem {
                fragment,
                source_hub_id: local_hub_id.clone(),
                relevance_score: 1.0, // Local results have highest relevance
            })
            .collect();

        let min_results = min_results.unwrap_or(limit);

        // Check if we need to federate
        let should_federate = federate && all_results.len() < min_results;

        if !should_federate {
            return Ok(FederatedSearchResponse {
                total: all_results.len(),
                results: all_results,
                sources: vec![SearchSource {
                    hub_id: local_hub_id,
                    count: local_count,
                }],
                federated: false,
            });
        }

        // Get other hubs for federation
        let other_hubs = self.discovery_service.get_federation_targets();

        if other_hubs.is_empty() {
            debug!("No other hubs available for federation");
            return Ok(FederatedSearchResponse {
                total: all_results.len(),
                results: all_results,
                sources: vec![SearchSource {
                    hub_id: local_hub_id,
                    count: local_count,
                }],
                federated: false,
            });
        }

        debug!("Federating search to {} other hubs", other_hubs.len());

        // Query other hubs in parallel
        let remaining_needed = min_results.saturating_sub(all_results.len());
        let futures: Vec<_> = other_hubs
            .iter()
            .map(|hub| self.query_remote_hub(hub, query, remaining_needed))
            .collect();

        let remote_results = join_all(futures).await;

        let mut sources = vec![SearchSource {
            hub_id: local_hub_id,
            count: local_count,
        }];

        // Aggregate results from remote hubs
        for (hub, result) in other_hubs.iter().zip(remote_results.into_iter()) {
            match result {
                Ok(fragments) => {
                    let count = fragments.len();
                    debug!("Hub {} returned {} results", hub.hub_id, count);

                    for fragment in fragments {
                        // Deduplicate by UUID
                        if !all_results.iter().any(|r| r.fragment.uuid == fragment.uuid) {
                            all_results.push(SearchResultItem {
                                fragment,
                                source_hub_id: hub.hub_id.clone(),
                                relevance_score: 0.9, // Remote results slightly lower
                            });
                        }
                    }

                    sources.push(SearchSource {
                        hub_id: hub.hub_id.clone(),
                        count,
                    });
                }
                Err(e) => {
                    warn!("Failed to query hub {}: {}", hub.hub_id, e);
                }
            }
        }

        // Sort by relevance score (local first, then remote)
        all_results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        // Apply limit
        if all_results.len() > limit {
            all_results.truncate(limit);
        }

        Ok(FederatedSearchResponse {
            total: all_results.len(),
            results: all_results,
            sources,
            federated: true,
        })
    }

    /// Query a remote hub for search results
    async fn query_remote_hub(
        &self,
        hub: &HubInfo,
        query: &str,
        limit: usize,
    ) -> HubResult<Vec<Fragment>> {
        let url = format!(
            "{}/api/v1/fragments/search?q={}&limit={}",
            hub.public_url,
            urlencoding::encode(query),
            limit
        );

        let response = tokio::time::timeout(self.timeout, self.http_client.get(&url).send())
            .await
            .map_err(|_| HubError::NetworkError(format!("Timeout querying hub {}", hub.hub_id)))?
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(HubError::FederationError(format!(
                "Hub {} returned error: {}",
                hub.hub_id,
                response.status()
            )));
        }

        // Parse the API response
        #[derive(Deserialize)]
        struct ApiResponse<T> {
            success: bool,
            data: Option<T>,
            #[allow(dead_code)]
            error: Option<String>,
        }

        #[derive(Deserialize)]
        struct SearchData {
            items: Vec<Fragment>,
        }

        let api_response: ApiResponse<SearchData> = response
            .json()
            .await
            .map_err(|e| HubError::NetworkError(format!("Failed to parse response: {}", e)))?;

        if !api_response.success {
            return Err(HubError::FederationError(format!(
                "Hub {} returned error in response",
                hub.hub_id
            )));
        }

        Ok(api_response.data.map(|d| d.items).unwrap_or_default())
    }

    /// Set query timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::store::{RocksStore, EntityStore};
    use crate::services::DiscoveryConfig;
    use crate::config::HubRole;
    use tempfile::tempdir;

    fn setup_service() -> FederatedSearchService {
        let dir = tempdir().unwrap();
        let rocks = RocksStore::open(dir.path().to_str().unwrap()).unwrap();
        let store = Arc::new(EntityStore::new(rocks));

        let entity_service = Arc::new(EntityService::new(Arc::clone(&store)));

        let discovery_config = DiscoveryConfig {
            role: HubRole::Primary,
            hub_id: "test-hub".to_string(),
            public_url: "http://localhost:8080".to_string(),
            ..Default::default()
        };
        let discovery_service = Arc::new(DiscoveryService::new(discovery_config, store));

        FederatedSearchService::new(entity_service, discovery_service)
    }

    #[tokio::test]
    async fn test_local_only_search() {
        let service = setup_service();

        let response = service.search("test", 10, false, None).await.unwrap();

        assert!(!response.federated);
        assert_eq!(response.sources.len(), 1);
        assert_eq!(response.sources[0].hub_id, "test-hub");
    }

    #[tokio::test]
    async fn test_federated_search_no_other_hubs() {
        let service = setup_service();

        // Even with federate=true, if no other hubs, should not federate
        let response = service.search("test", 10, true, Some(10)).await.unwrap();

        assert!(!response.federated);
        assert_eq!(response.sources.len(), 1);
    }
}

//! Federated search module
//!
//! Will be fully implemented in Phase 5b.

use std::time::Duration;
use futures::future::join_all;

use super::{DiscoveryClient, HubInfo};
use crate::models::{Fragment, HubResult, HubError};

/// Result from a federated search
#[derive(Debug, Clone)]
pub struct SearchResult {
    pub fragment: Fragment,
    pub source_hub_id: String,
    pub relevance_score: f64,
}

/// Source information for search results
#[derive(Debug, Clone)]
pub struct SearchSource {
    pub hub_id: String,
    pub count: usize,
}

/// Federated search response
#[derive(Debug, Clone)]
pub struct FederatedSearchResponse {
    pub results: Vec<SearchResult>,
    pub sources: Vec<SearchSource>,
    pub federated: bool,
    pub total: usize,
}

/// Federated search coordinator
pub struct FederatedSearch {
    discovery_client: Option<DiscoveryClient>,
    http_client: reqwest::Client,
    timeout: Duration,
}

impl FederatedSearch {
    /// Create a new federated search coordinator
    pub fn new(discovery_client: Option<DiscoveryClient>) -> Self {
        let http_client = reqwest::Client::builder()
            .timeout(Duration::from_secs(10))
            .build()
            .expect("Failed to create HTTP client");

        Self {
            discovery_client,
            http_client,
            timeout: Duration::from_secs(5),
        }
    }

    /// Perform a federated search across hubs
    pub async fn search(
        &self,
        query: &str,
        min_results: usize,
        local_results: Vec<Fragment>,
        local_hub_id: &str,
    ) -> HubResult<FederatedSearchResponse> {
        let mut all_results: Vec<SearchResult> = local_results
            .into_iter()
            .map(|f| SearchResult {
                fragment: f,
                source_hub_id: local_hub_id.to_string(),
                relevance_score: 1.0, // Local results have highest relevance
            })
            .collect();

        let local_count = all_results.len();

        // If we have enough local results, return them
        if all_results.len() >= min_results {
            return Ok(FederatedSearchResponse {
                total: all_results.len(),
                results: all_results,
                sources: vec![SearchSource {
                    hub_id: local_hub_id.to_string(),
                    count: local_count,
                }],
                federated: false,
            });
        }

        // Get other hubs to query
        let other_hubs = self.get_other_hubs();

        if other_hubs.is_empty() {
            return Ok(FederatedSearchResponse {
                total: all_results.len(),
                results: all_results,
                sources: vec![SearchSource {
                    hub_id: local_hub_id.to_string(),
                    count: local_count,
                }],
                federated: false,
            });
        }

        // Query other hubs in parallel
        let futures: Vec<_> = other_hubs
            .iter()
            .map(|hub| self.query_hub(hub, query))
            .collect();

        let remote_results = join_all(futures).await;

        let mut sources = vec![SearchSource {
            hub_id: local_hub_id.to_string(),
            count: local_count,
        }];

        // Aggregate results
        for (hub, result) in other_hubs.iter().zip(remote_results.into_iter()) {
            if let Ok(fragments) = result {
                let count = fragments.len();
                for fragment in fragments {
                    // Check for duplicates by UUID
                    if !all_results.iter().any(|r| r.fragment.uuid == fragment.uuid) {
                        all_results.push(SearchResult {
                            fragment,
                            source_hub_id: hub.hub_id.clone(),
                            relevance_score: 0.9, // Remote results have slightly lower relevance
                        });
                    }
                }
                sources.push(SearchSource {
                    hub_id: hub.hub_id.clone(),
                    count,
                });
            }
        }

        // Sort by relevance
        all_results.sort_by(|a, b| {
            b.relevance_score
                .partial_cmp(&a.relevance_score)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(FederatedSearchResponse {
            total: all_results.len(),
            results: all_results,
            sources,
            federated: true,
        })
    }

    /// Query a single hub for search results
    async fn query_hub(&self, hub: &HubInfo, query: &str) -> HubResult<Vec<Fragment>> {
        let url = format!(
            "{}/api/v1/fragments/search?q={}",
            hub.public_url,
            urlencoding::encode(query)
        );

        let response = tokio::time::timeout(self.timeout, self.http_client.get(&url).send())
            .await
            .map_err(|_| HubError::NetworkError("Query timeout".to_string()))?
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        if !response.status().is_success() {
            return Err(HubError::FederationError(format!(
                "Hub {} returned error: {}",
                hub.hub_id,
                response.status()
            )));
        }

        #[derive(serde::Deserialize)]
        struct SearchResponse {
            items: Vec<Fragment>,
        }

        let result: SearchResponse = response
            .json()
            .await
            .map_err(|e| HubError::NetworkError(e.to_string()))?;

        Ok(result.items)
    }

    /// Get other hubs from discovery client
    fn get_other_hubs(&self) -> Vec<HubInfo> {
        self.discovery_client
            .as_ref()
            .map(|c| c.get_other_hubs())
            .unwrap_or_default()
    }

    /// Set query timeout
    pub fn with_timeout(mut self, timeout: Duration) -> Self {
        self.timeout = timeout;
        self
    }
}

impl Default for FederatedSearch {
    fn default() -> Self {
        Self::new(None)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Address;

    #[tokio::test]
    async fn test_local_only_search() {
        let search = FederatedSearch::default();

        let creator = Address::agent("hub:8080", "agent-uuid");
        let local_results = vec![
            Fragment::new("Result 1", creator.clone()).with_signature("sig1"),
            Fragment::new("Result 2", creator.clone()).with_signature("sig2"),
            Fragment::new("Result 3", creator.clone()).with_signature("sig3"),
        ];

        let response = search
            .search("test", 3, local_results, "local-hub")
            .await
            .unwrap();

        assert!(!response.federated);
        assert_eq!(response.total, 3);
        assert_eq!(response.sources.len(), 1);
    }
}

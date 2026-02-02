//! Trust path finding using BFS/Dijkstra

use crate::models::{Address, TrustPath, TrustPathHop, HubResult};

/// Configuration for trust path finding
#[derive(Debug, Clone)]
pub struct TrustPathConfig {
    /// Maximum depth to search
    pub max_depth: u8,
    /// Damping factor per hop (e.g., 0.8)
    pub damping_factor: f32,
    /// Minimum effective trust to consider
    pub min_trust_threshold: f32,
}

impl Default for TrustPathConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            damping_factor: 0.8,
            min_trust_threshold: 0.01,
        }
    }
}

/// Trust path finder using BFS with damping
pub struct TrustPathFinder {
    config: TrustPathConfig,
}

impl TrustPathFinder {
    /// Create a new trust path finder
    pub fn new(config: TrustPathConfig) -> Self {
        Self { config }
    }

    /// Find the best trust path between two entities
    pub async fn find_best_path(
        &self,
        from: &Address,
        to: &Address,
    ) -> HubResult<Option<TrustPath>> {
        // Self-trust: direct path with trust 1.0
        if from == to {
            return Ok(Some(TrustPath::direct(from.clone(), to.clone(), 1.0)));
        }

        // For now, return None (no path found)
        // Full implementation would query the trust graph
        Ok(None)
    }

    /// Find all trust paths up to max_depth
    pub async fn find_all_paths(
        &self,
        from: &Address,
        to: &Address,
    ) -> HubResult<Vec<TrustPath>> {
        if let Some(path) = self.find_best_path(from, to).await? {
            Ok(vec![path])
        } else {
            Ok(vec![])
        }
    }

    /// Calculate effective trust from a path
    pub fn calculate_effective_trust(&self, hops: &[TrustPathHop]) -> f32 {
        if hops.is_empty() {
            return 0.0;
        }

        let mut trust = hops[0].trust_level;
        for hop in hops.iter().skip(1) {
            trust *= hop.trust_level * self.config.damping_factor;
        }

        trust.clamp(-1.0, 1.0)
    }

    /// Get the configuration
    pub fn config(&self) -> &TrustPathConfig {
        &self.config
    }
}

impl Default for TrustPathFinder {
    fn default() -> Self {
        Self::new(TrustPathConfig::default())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn test_self_trust() {
        let finder = TrustPathFinder::default();
        let agent = Address::agent("hub:8080", "agent-1");

        let path = finder.find_best_path(&agent, &agent).await.unwrap();
        assert!(path.is_some());

        let path = path.unwrap();
        assert_eq!(path.effective_trust, 1.0);
        assert_eq!(path.depth, 1);
        assert!(path.is_trusted());
    }

    #[test]
    fn test_effective_trust_calculation() {
        let finder = TrustPathFinder::new(TrustPathConfig {
            damping_factor: 0.8,
            ..Default::default()
        });

        let hops = vec![
            TrustPathHop {
                agent: Address::agent("hub:8080", "agent-1"),
                trust_level: 0.9,
            },
            TrustPathHop {
                agent: Address::agent("hub:8080", "agent-2"),
                trust_level: 0.8,
            },
        ];

        let effective = finder.calculate_effective_trust(&hops);
        // 0.9 * 0.8 * 0.8 = 0.576
        assert!((effective - 0.576).abs() < 0.001);
    }

    #[test]
    fn test_negative_trust() {
        let finder = TrustPathFinder::default();

        let hops = vec![
            TrustPathHop {
                agent: Address::agent("hub:8080", "agent-1"),
                trust_level: -0.5,
            },
        ];

        let effective = finder.calculate_effective_trust(&hops);
        assert_eq!(effective, -0.5);
    }
}

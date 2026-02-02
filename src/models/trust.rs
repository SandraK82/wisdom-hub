//! Trust models for trust path calculation
//!
//! Note: Direct trust relationships are now embedded in Agent (TrustStore).
//! This module provides types for trust path queries and results.

use serde::{Deserialize, Serialize};

use super::Address;

/// A hop in a trust path
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPathHop {
    /// Agent address at this hop
    pub agent: Address,
    /// Trust level to this agent (-1.0 to 1.0)
    pub trust_level: f32,
}

/// A trust path from one agent to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPath {
    /// Source agent
    pub from: Address,
    /// Target (can be agent, fragment, or other entity)
    pub to: Address,
    /// The hops in this path
    pub hops: Vec<TrustPathHop>,
    /// Effective trust after damping (-1.0 to 1.0)
    pub effective_trust: f32,
    /// Path depth
    pub depth: usize,
}

impl TrustPath {
    /// Create a new empty path
    pub fn empty(from: Address, to: Address) -> Self {
        Self {
            from,
            to,
            hops: Vec::new(),
            effective_trust: 0.0,
            depth: 0,
        }
    }

    /// Create a direct trust path (single hop)
    pub fn direct(from: Address, to: Address, trust_level: f32) -> Self {
        Self {
            from: from.clone(),
            to: to.clone(),
            hops: vec![TrustPathHop {
                agent: to,
                trust_level,
            }],
            effective_trust: trust_level,
            depth: 1,
        }
    }

    /// Add a hop to the path with damping
    pub fn add_hop(&mut self, agent: Address, trust_level: f32, damping_factor: f32) {
        let clamped_trust = trust_level.clamp(-1.0, 1.0);

        let new_trust = if self.hops.is_empty() {
            clamped_trust
        } else {
            self.effective_trust * clamped_trust * damping_factor
        };

        self.hops.push(TrustPathHop {
            agent,
            trust_level: clamped_trust,
        });
        self.effective_trust = new_trust.clamp(-1.0, 1.0);
        self.depth = self.hops.len();
    }

    /// Check if path is empty (no trust relationship found)
    pub fn is_empty(&self) -> bool {
        self.hops.is_empty()
    }

    /// Check if this represents a positive trust path
    pub fn is_trusted(&self) -> bool {
        self.effective_trust > 0.0
    }

    /// Check if this represents distrust
    pub fn is_distrusted(&self) -> bool {
        self.effective_trust < 0.0
    }
}

/// Trust score for an entity from a viewer's perspective
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScore {
    /// Entity being evaluated
    pub entity: Address,
    /// Viewer's perspective (who is asking)
    pub viewer: Address,
    /// Aggregated trust score (-1.0 to 1.0)
    pub score: f32,
    /// Number of trust paths found
    pub path_count: usize,
    /// Best path found (if any)
    #[serde(skip_serializing_if = "Option::is_none")]
    pub best_path: Option<TrustPath>,
}

impl TrustScore {
    /// Create a new trust score
    pub fn new(entity: Address, viewer: Address, score: f32, path_count: usize) -> Self {
        Self {
            entity,
            viewer,
            score: score.clamp(-1.0, 1.0),
            path_count,
            best_path: None,
        }
    }

    /// Set the best path
    pub fn with_best_path(mut self, path: TrustPath) -> Self {
        self.best_path = Some(path);
        self
    }

    /// Create a neutral score (no trust information)
    pub fn neutral(entity: Address, viewer: Address) -> Self {
        Self::new(entity, viewer, 0.0, 0)
    }
}

/// Request for trust path calculation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustPathRequest {
    /// Agent requesting the path
    pub from: Address,
    /// Entity to find trust path to
    pub to: Address,
    /// Maximum path depth (default: 5)
    #[serde(default = "default_max_depth")]
    pub max_depth: u8,
}

fn default_max_depth() -> u8 {
    5
}

/// Request for trust score
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct TrustScoreRequest {
    /// Entity to score
    pub entity: Address,
    /// Viewer's perspective
    pub viewer: Address,
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_trust_path_direct() {
        let from = Address::agent("hub:8080", "agent-1");
        let to = Address::agent("hub:8080", "agent-2");

        let path = TrustPath::direct(from.clone(), to.clone(), 0.8);

        assert_eq!(path.from, from);
        assert_eq!(path.to, to);
        assert_eq!(path.depth, 1);
        assert_eq!(path.effective_trust, 0.8);
        assert!(path.is_trusted());
    }

    #[test]
    fn test_trust_path_with_damping() {
        let from = Address::agent("hub:8080", "agent-1");
        let to = Address::agent("hub:8080", "agent-3");
        let hop = Address::agent("hub:8080", "agent-2");

        let mut path = TrustPath::empty(from, to);
        path.add_hop(hop, 0.9, 0.8);
        path.add_hop(Address::agent("hub:8080", "agent-3"), 0.8, 0.8);

        assert_eq!(path.depth, 2);
        // 0.9 * 0.8 * 0.8 = 0.576
        assert!(path.effective_trust < 0.9);
        assert!(path.is_trusted());
    }

    #[test]
    fn test_distrust_path() {
        let from = Address::agent("hub:8080", "agent-1");
        let to = Address::agent("hub:8080", "agent-2");

        let path = TrustPath::direct(from, to, -0.5);

        assert!(path.is_distrusted());
        assert!(!path.is_trusted());
    }

    #[test]
    fn test_trust_score() {
        let entity = Address::fragment("hub:8080", "frag-1");
        let viewer = Address::agent("hub:8080", "agent-1");

        let score = TrustScore::new(entity.clone(), viewer.clone(), 0.75, 3);

        assert_eq!(score.entity, entity);
        assert_eq!(score.viewer, viewer);
        assert_eq!(score.score, 0.75);
        assert_eq!(score.path_count, 3);
    }
}

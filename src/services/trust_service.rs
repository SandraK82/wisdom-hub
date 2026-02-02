//! Trust service for path calculations
//!
//! Trust relationships are now embedded in Agent (TrustStore).
//! This service provides path finding and score calculation.

use std::collections::{HashMap, HashSet, VecDeque};
use std::sync::Arc;

use crate::models::{
    Address, TrustPath, TrustPathHop, TrustScore,
    HubResult, HubError, Domain,
};
use crate::store::EntityStore;

/// Configuration for trust calculations
#[derive(Debug, Clone)]
pub struct TrustConfig {
    /// Maximum depth to search for trust paths
    pub max_depth: u8,
    /// Damping factor applied per hop (e.g., 0.8 means 20% reduction per hop)
    pub damping_factor: f32,
    /// Minimum trust threshold to consider a path valid
    pub min_trust_threshold: f32,
}

impl Default for TrustConfig {
    fn default() -> Self {
        Self {
            max_depth: 5,
            damping_factor: 0.8,
            min_trust_threshold: 0.01,
        }
    }
}

/// Service for trust path calculations
pub struct TrustService {
    store: Arc<EntityStore>,
    config: TrustConfig,
}

impl TrustService {
    /// Create a new trust service
    pub fn new(store: Arc<EntityStore>, config: TrustConfig) -> Self {
        Self { store, config }
    }

    /// Get the store reference
    pub fn store(&self) -> &EntityStore {
        &self.store
    }

    /// Find the best trust path from one agent to another using BFS
    pub fn find_best_path(
        &self,
        from: &Address,
        to: &Address,
    ) -> HubResult<Option<TrustPath>> {
        // Self-trust is always 1.0
        if from == to {
            return Ok(Some(TrustPath::direct(from.clone(), to.clone(), 1.0)));
        }

        // Find all paths using BFS
        let paths = self.find_all_paths(from, to)?;

        // Return the path with highest effective trust
        Ok(paths.into_iter().max_by(|a, b| {
            a.effective_trust
                .partial_cmp(&b.effective_trust)
                .unwrap_or(std::cmp::Ordering::Equal)
        }))
    }

    /// Find all trust paths up to max_depth using BFS
    pub fn find_all_paths(
        &self,
        from: &Address,
        to: &Address,
    ) -> HubResult<Vec<TrustPath>> {
        if from == to {
            return Ok(vec![TrustPath::direct(from.clone(), to.clone(), 1.0)]);
        }

        // Ensure from is an agent
        if from.domain != Domain::Agent {
            return Err(HubError::ValidationError(
                "Trust paths must start from an agent".to_string(),
            ));
        }

        let mut paths = Vec::new();
        let mut visited = HashSet::new();

        // BFS queue: (current_address, path_so_far, current_trust)
        let mut queue: VecDeque<(Address, Vec<TrustPathHop>, f32)> = VecDeque::new();

        queue.push_back((from.clone(), Vec::new(), 1.0));

        while let Some((current, path, cumulative_trust)) = queue.pop_front() {
            // Check depth limit
            if path.len() >= self.config.max_depth as usize {
                continue;
            }

            // Check if below minimum threshold
            if cumulative_trust.abs() < self.config.min_trust_threshold {
                continue;
            }

            // Mark as visited for this path
            visited.insert(current.entity.clone());

            // Get the current agent to access their trust store
            if let Some(agent) = self.store.get_agent(&current.entity)? {
                for trust in &agent.trust.trusts {
                    let trustee = &trust.agent;

                    // Skip if already in path (avoid cycles)
                    if path.iter().any(|h| h.agent == *trustee) || trustee.entity == from.entity {
                        continue;
                    }

                    // Calculate new trust level with damping
                    let hop_trust = trust.trust;
                    let damping = if path.is_empty() { 1.0 } else { self.config.damping_factor };
                    let new_cumulative = cumulative_trust * hop_trust * damping;

                    // Build new path
                    let mut new_path = path.clone();
                    new_path.push(TrustPathHop {
                        agent: trustee.clone(),
                        trust_level: hop_trust,
                    });

                    // Check if we reached the target
                    if trustee.entity == to.entity {
                        let trust_path = TrustPath {
                            from: from.clone(),
                            to: to.clone(),
                            effective_trust: new_cumulative,
                            depth: new_path.len(),
                            hops: new_path,
                        };
                        paths.push(trust_path);
                    } else if !visited.contains(&trustee.entity) && trustee.domain == Domain::Agent {
                        // Continue exploring (only follow agent nodes)
                        queue.push_back((trustee.clone(), new_path, new_cumulative));
                    }
                }
            }

            // Allow revisiting for different paths
            visited.remove(&current.entity);
        }

        // Sort by effective trust (highest first)
        paths.sort_by(|a, b| {
            b.effective_trust
                .partial_cmp(&a.effective_trust)
                .unwrap_or(std::cmp::Ordering::Equal)
        });

        Ok(paths)
    }

    /// Calculate trust score for an entity from a viewer's perspective
    pub fn calculate_trust_score(
        &self,
        entity: &Address,
        viewer: &Address,
    ) -> HubResult<TrustScore> {
        // Calculate from viewer's perspective using best path
        if let Some(path) = self.find_best_path(viewer, entity)? {
            Ok(TrustScore::new(
                entity.clone(),
                viewer.clone(),
                path.effective_trust,
                1,
            ).with_best_path(path))
        } else {
            // No path found - neutral score
            Ok(TrustScore::neutral(entity.clone(), viewer.clone()))
        }
    }

    /// Get direct trust level between two agents
    pub fn get_direct_trust(
        &self,
        from: &Address,
        to: &Address,
    ) -> HubResult<Option<f32>> {
        if from.domain != Domain::Agent {
            return Ok(None);
        }

        if let Some(agent) = self.store.get_agent(&from.entity)? {
            for trust in &agent.trust.trusts {
                if trust.agent.entity == to.entity {
                    return Ok(Some(trust.trust));
                }
            }
        }

        Ok(None)
    }

    /// Build a trust graph for visualization/analysis
    pub fn build_trust_graph(
        &self,
        center: &Address,
        max_depth: u8,
    ) -> HubResult<TrustGraph> {
        let mut nodes = HashMap::new();
        let mut edges = Vec::new();
        let mut visited = HashSet::new();
        let mut queue = VecDeque::new();

        queue.push_back((center.clone(), 0u8));

        while let Some((current, depth)) = queue.pop_front() {
            if depth > max_depth || visited.contains(&current.entity) {
                continue;
            }
            visited.insert(current.entity.clone());

            // Add node
            if let Some(agent) = self.store.get_agent(&current.entity)? {
                nodes.insert(current.entity.clone(), TrustGraphNode {
                    address: current.clone(),
                    description: agent.description,
                    depth,
                });

                // Get outgoing trust relations from embedded TrustStore
                for trust in &agent.trust.trusts {
                    edges.push(TrustGraphEdge {
                        from: current.clone(),
                        to: trust.agent.clone(),
                        trust_level: trust.trust,
                    });

                    if !visited.contains(&trust.agent.entity) && trust.agent.domain == Domain::Agent {
                        queue.push_back((trust.agent.clone(), depth + 1));
                    }
                }
            }
        }

        Ok(TrustGraph { nodes, edges })
    }
}

/// Node in a trust graph
#[derive(Debug, Clone)]
pub struct TrustGraphNode {
    pub address: Address,
    pub description: String,
    pub depth: u8,
}

/// Edge in a trust graph
#[derive(Debug, Clone)]
pub struct TrustGraphEdge {
    pub from: Address,
    pub to: Address,
    pub trust_level: f32,
}

/// Trust graph structure
#[derive(Debug, Clone)]
pub struct TrustGraph {
    pub nodes: HashMap<String, TrustGraphNode>,
    pub edges: Vec<TrustGraphEdge>,
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::{Agent, CreateAgentRequest};
    use crate::store::RocksStore;
    use tempfile::tempdir;

    fn setup_test_service() -> (TrustService, tempfile::TempDir) {
        let dir = tempdir().unwrap();
        let rocks = RocksStore::open(dir.path()).unwrap();
        let store = Arc::new(EntityStore::new(rocks));
        (TrustService::new(store, TrustConfig::default()), dir)
    }

    fn create_test_agent(store: &EntityStore, uuid: &str) -> Agent {
        let req = CreateAgentRequest {
            uuid: Some(uuid.to_string()),
            public_key: "dGVzdC1rZXk=".to_string(),
            description: Some(format!("Agent {}", uuid)),
            primary_hub: None,
            signature: "sig".to_string(),
        };
        let agent = Agent::from(req);
        store.put_agent(&agent).unwrap();
        agent
    }

    #[test]
    fn test_self_trust() {
        let (service, _dir) = setup_test_service();
        let _alice = create_test_agent(&service.store, "alice");
        let alice_addr = Address::agent("hub:8080", "alice");

        let path = service.find_best_path(&alice_addr, &alice_addr).unwrap();
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.effective_trust, 1.0);
        assert_eq!(path.depth, 1);
    }

    #[test]
    fn test_direct_trust() {
        let (service, _dir) = setup_test_service();

        let mut alice = create_test_agent(&service.store, "alice");
        let _bob = create_test_agent(&service.store, "bob");

        let bob_addr = Address::agent("hub:8080", "bob");
        alice.add_trust(bob_addr.clone(), 0.9);
        service.store.put_agent(&alice).unwrap();

        let alice_addr = Address::agent("hub:8080", "alice");

        // Find path from Alice to Bob
        let path = service.find_best_path(&alice_addr, &bob_addr).unwrap();
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.depth, 1);
        assert!((path.effective_trust - 0.9).abs() < 0.001);
    }

    #[test]
    fn test_transitive_trust() {
        let (service, _dir) = setup_test_service();

        let mut alice = create_test_agent(&service.store, "alice");
        let mut bob = create_test_agent(&service.store, "bob");
        let _charlie = create_test_agent(&service.store, "charlie");

        let bob_addr = Address::agent("hub:8080", "bob");
        let charlie_addr = Address::agent("hub:8080", "charlie");

        // Alice trusts Bob with 0.9
        alice.add_trust(bob_addr.clone(), 0.9);
        service.store.put_agent(&alice).unwrap();

        // Bob trusts Charlie with 0.8
        bob.add_trust(charlie_addr.clone(), 0.8);
        service.store.put_agent(&bob).unwrap();

        let alice_addr = Address::agent("hub:8080", "alice");

        // Find path from Alice to Charlie
        let path = service.find_best_path(&alice_addr, &charlie_addr).unwrap();
        assert!(path.is_some());
        let path = path.unwrap();
        assert_eq!(path.depth, 2);

        // Effective trust: 0.9 * 0.8 * 0.8 (damping) = 0.576
        assert!((path.effective_trust - 0.576).abs() < 0.001);
    }

    #[test]
    fn test_no_path() {
        let (service, _dir) = setup_test_service();

        let _alice = create_test_agent(&service.store, "alice");
        let _bob = create_test_agent(&service.store, "bob");

        let alice_addr = Address::agent("hub:8080", "alice");
        let bob_addr = Address::agent("hub:8080", "bob");

        // No trust relation between Alice and Bob
        let path = service.find_best_path(&alice_addr, &bob_addr).unwrap();
        assert!(path.is_none());
    }

    #[test]
    fn test_trust_score() {
        let (service, _dir) = setup_test_service();

        let mut alice = create_test_agent(&service.store, "alice");
        let _bob = create_test_agent(&service.store, "bob");

        let alice_addr = Address::agent("hub:8080", "alice");
        let bob_addr = Address::agent("hub:8080", "bob");

        // Alice trusts Bob with 0.9
        alice.add_trust(bob_addr.clone(), 0.9);
        service.store.put_agent(&alice).unwrap();

        // Score from Alice's perspective
        let score = service.calculate_trust_score(&bob_addr, &alice_addr).unwrap();
        assert!((score.score - 0.9).abs() < 0.001);
        assert_eq!(score.path_count, 1);
    }
}

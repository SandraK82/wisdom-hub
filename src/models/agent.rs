//! Agent model representing participants in the wisdom network

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use std::collections::HashMap;

use super::Address;

/// A domain of expertise (renamed from Domain to avoid conflict with address::Domain)
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum ExpertiseDomain {
    /// Programming-related (e.g., "rust", "python", "typescript")
    Programming(String),
    /// Science-related (e.g., "physics", "biology", "chemistry")
    Science(String),
    /// Business-related (e.g., "finance", "marketing", "strategy")
    Business(String),
    /// Custom domain
    Custom(String),
}

impl ExpertiseDomain {
    /// Create a programming domain
    pub fn programming(lang: impl Into<String>) -> Self {
        ExpertiseDomain::Programming(lang.into())
    }

    /// Create a science domain
    pub fn science(field: impl Into<String>) -> Self {
        ExpertiseDomain::Science(field.into())
    }

    /// Create a business domain
    pub fn business(area: impl Into<String>) -> Self {
        ExpertiseDomain::Business(area.into())
    }

    /// Create a custom domain
    pub fn custom(name: impl Into<String>) -> Self {
        ExpertiseDomain::Custom(name.into())
    }
}

impl std::fmt::Display for ExpertiseDomain {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ExpertiseDomain::Programming(s) => write!(f, "programming:{}", s),
            ExpertiseDomain::Science(s) => write!(f, "science:{}", s),
            ExpertiseDomain::Business(s) => write!(f, "business:{}", s),
            ExpertiseDomain::Custom(s) => write!(f, "custom:{}", s),
        }
    }
}

/// A known bias or tendency of an agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Bias {
    /// The domain this bias applies to
    pub domain: ExpertiseDomain,
    /// Description of the bias
    pub description: String,
    /// Severity of the bias (0.0 to 1.0)
    pub severity: f32,
}

impl Bias {
    /// Create a new bias
    pub fn new(domain: ExpertiseDomain, description: impl Into<String>, severity: f32) -> Self {
        Self {
            domain,
            description: description.into(),
            severity: severity.clamp(0.0, 1.0),
        }
    }
}

/// Profile of an agent's expertise and characteristics
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct AgentProfile {
    /// Expertise scores per domain (0.0 to 1.0)
    pub specializations: HashMap<String, f32>,
    /// Known biases or tendencies
    pub known_biases: Vec<Bias>,
    /// Average confidence in created fragments
    pub avg_confidence: f32,
    /// Total number of fragments created
    pub fragment_count: u64,
    /// Historical accuracy score (0.0 to 1.0)
    pub historical_accuracy: f32,
}

impl AgentProfile {
    /// Create a new empty profile
    pub fn new() -> Self {
        Self::default()
    }

    /// Add or update a specialization
    pub fn add_specialization(&mut self, domain: impl Into<String>, score: f32) {
        self.specializations.insert(domain.into(), score.clamp(0.0, 1.0));
    }

    /// Get specialization score for a domain
    pub fn get_specialization(&self, domain: &str) -> f32 {
        self.specializations.get(domain).copied().unwrap_or(0.0)
    }

    /// Add a known bias
    pub fn add_bias(&mut self, bias: Bias) {
        self.known_biases.push(bias);
    }

    /// Update statistics after creating a fragment
    pub fn update_stats(&mut self, confidence: f32) {
        let total = self.fragment_count as f32 * self.avg_confidence + confidence;
        self.fragment_count += 1;
        self.avg_confidence = total / self.fragment_count as f32;
    }
}

/// An agent participating in the wisdom network
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Agent {
    /// Unique identifier
    pub uuid: String,
    /// Base64-encoded Ed25519 public key
    pub public_key: String,
    /// Version number (incremented on updates)
    pub version: u32,
    /// Human-readable description
    pub description: String,
    /// Embedded trust relationships for efficient path-finding
    pub trust: TrustStore,
    /// Where this agent's data primarily lives
    pub primary_hub: String,
    /// Ed25519 signature over the agent data
    pub signature: String,
    /// When the agent was created
    pub created_at: DateTime<Utc>,
    /// When the agent was last updated
    pub updated_at: DateTime<Utc>,
    /// Agent's expertise profile
    #[serde(default)]
    pub profile: AgentProfile,
}

/// Contains an agent's direct trust relationships
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct TrustStore {
    /// Total number of trust relationships
    pub num_trusts: u64,
    /// Direct trust relationships
    pub trusts: Vec<Trust>,
}

/// A direct trust relationship from one agent to another
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Trust {
    /// The agent being trusted/distrusted
    pub agent: Address,
    /// Trust level: -1.0 (distrust) to 1.0 (full trust), 0 = neutral
    pub trust: f32,
}

impl Agent {
    /// Create a new agent
    pub fn new(uuid: impl Into<String>, public_key: impl Into<String>) -> Self {
        let now = Utc::now();
        Self {
            uuid: uuid.into(),
            public_key: public_key.into(),
            version: 1,
            description: String::new(),
            trust: TrustStore::default(),
            primary_hub: String::new(),
            signature: String::new(),
            created_at: now,
            updated_at: now,
            profile: AgentProfile::default(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Set primary hub
    pub fn with_primary_hub(mut self, hub: impl Into<String>) -> Self {
        self.primary_hub = hub.into();
        self
    }

    /// Set signature
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = signature.into();
        self
    }

    /// Add a trust relationship
    pub fn add_trust(&mut self, agent: Address, trust_level: f32) {
        let clamped = trust_level.clamp(-1.0, 1.0);
        self.trust.trusts.push(Trust {
            agent,
            trust: clamped,
        });
        self.trust.num_trusts = self.trust.trusts.len() as u64;
    }

    /// Get trust for a specific agent
    pub fn get_trust_for(&self, agent_addr: &Address) -> f32 {
        for t in &self.trust.trusts {
            if t.agent == *agent_addr {
                return t.trust;
            }
        }
        0.0
    }

    /// Increment version
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Validate the agent data
    pub fn validate(&self) -> Result<(), String> {
        if self.uuid.is_empty() {
            return Err("uuid is required".to_string());
        }
        if self.public_key.is_empty() {
            return Err("public_key is required".to_string());
        }
        if self.signature.is_empty() {
            return Err("signature is required".to_string());
        }
        // Validate trust values
        for (i, t) in self.trust.trusts.iter().enumerate() {
            if t.trust < -1.0 || t.trust > 1.0 {
                return Err(format!("trust[{}].trust must be between -1.0 and 1.0", i));
            }
        }
        Ok(())
    }
}

impl TrustStore {
    /// Create an empty trust store
    pub fn new() -> Self {
        Self::default()
    }
}

/// Request to create a new agent
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateAgentRequest {
    pub uuid: Option<String>,
    pub public_key: String,
    pub description: Option<String>,
    pub trust: Option<TrustStore>,
    pub primary_hub: Option<String>,
    pub signature: String,
}

impl From<CreateAgentRequest> for Agent {
    fn from(req: CreateAgentRequest) -> Self {
        let uuid = req.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut agent = Agent::new(uuid, req.public_key)
            .with_signature(req.signature);

        if let Some(desc) = req.description {
            agent = agent.with_description(desc);
        }
        if let Some(trust) = req.trust {
            agent.trust = trust;
        }
        if let Some(hub) = req.primary_hub {
            agent = agent.with_primary_hub(hub);
        }
        agent
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_agent() {
        let agent = Agent::new("test-uuid", "base64-public-key")
            .with_signature("test-sig");
        assert_eq!(agent.uuid, "test-uuid");
        assert_eq!(agent.public_key, "base64-public-key");
        assert_eq!(agent.version, 1);
    }

    #[test]
    fn test_trust_store() {
        let mut agent = Agent::new("test-uuid", "key")
            .with_signature("sig");

        let other = Address::agent("hub:8080", "other-agent");
        agent.add_trust(other.clone(), 0.8);

        assert_eq!(agent.trust.num_trusts, 1);
        assert_eq!(agent.get_trust_for(&other), 0.8);
    }

    #[test]
    fn test_trust_clamping() {
        let mut agent = Agent::new("test", "key")
            .with_signature("sig");

        let other = Address::agent("hub:8080", "other");
        agent.add_trust(other.clone(), 1.5); // Should clamp to 1.0

        assert_eq!(agent.get_trust_for(&other), 1.0);
    }
}

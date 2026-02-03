//! Relation model representing relationships between entities

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::Address;

/// Known relation types
/// Note: Fragment typing (QUESTION, HYPOTHESIS, etc.) now uses TYPE tags instead.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum RelationType {
    /// Agent trusts/distrusts the From entity
    Trust,
    /// From supports To
    Supports,
    /// From contradicts To
    Contradicts,
    /// From extends To
    Extends,
    /// From supersedes To
    Supersedes,
    /// From is derived from To
    DerivedFrom,
    /// Generic relation (also used for tag application)
    RelatedTo,
    /// From is an example of To
    ExampleOf,
    /// From specializes To
    Specializes,
    /// From clarifies To
    Clarifies,
    /// From generalizes To
    Generalizes,
}

impl Default for RelationType {
    fn default() -> Self {
        RelationType::RelatedTo
    }
}

impl fmt::Display for RelationType {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RelationType::Trust => write!(f, "TRUST"),
            RelationType::Supports => write!(f, "SUPPORTS"),
            RelationType::Contradicts => write!(f, "CONTRADICTS"),
            RelationType::Extends => write!(f, "EXTENDS"),
            RelationType::Supersedes => write!(f, "SUPERSEDES"),
            RelationType::DerivedFrom => write!(f, "DERIVED_FROM"),
            RelationType::RelatedTo => write!(f, "RELATED_TO"),
            RelationType::ExampleOf => write!(f, "EXAMPLE_OF"),
            RelationType::Specializes => write!(f, "SPECIALIZES"),
            RelationType::Clarifies => write!(f, "CLARIFIES"),
            RelationType::Generalizes => write!(f, "GENERALIZES"),
        }
    }
}

impl FromStr for RelationType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "TRUST" => Ok(RelationType::Trust),
            "SUPPORTS" => Ok(RelationType::Supports),
            "CONTRADICTS" => Ok(RelationType::Contradicts),
            "EXTENDS" => Ok(RelationType::Extends),
            "SUPERSEDES" => Ok(RelationType::Supersedes),
            "DERIVED_FROM" => Ok(RelationType::DerivedFrom),
            "RELATED_TO" => Ok(RelationType::RelatedTo),
            "EXAMPLE_OF" => Ok(RelationType::ExampleOf),
            "SPECIALIZES" => Ok(RelationType::Specializes),
            "CLARIFIES" => Ok(RelationType::Clarifies),
            "GENERALIZES" => Ok(RelationType::Generalizes),
            _ => Err(format!("Invalid relation type: {}", s)),
        }
    }
}

/// A relation between entities in the wisdom network.
/// Relations can express content relationships, trust, or type fragments.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Relation {
    /// Unique identifier
    pub uuid: String,
    /// Source entity (required)
    pub from: Address,
    /// Target entity (optional for self-reference)
    #[serde(default)]
    pub to: Address,
    /// Agent who asserts this relation
    #[serde(default)]
    pub by: Address,
    /// Type of relationship
    #[serde(rename = "type")]
    pub relation_type: RelationType,
    /// Optional reasoning/explanation
    #[serde(default)]
    pub content: String,
    /// Agent who created this relation
    pub creator: Address,
    /// Version number
    pub version: u32,
    /// Ed25519 signature over the relation data
    pub signature: String,
    /// Content timestamp (when the relation was asserted)
    pub when: DateTime<Utc>,
    /// When the relation was stored in the database
    pub created_at: DateTime<Utc>,
    /// Strength of this relationship (0.0 to 1.0)
    #[serde(default = "default_relation_confidence")]
    pub confidence: f32,
}

fn default_relation_confidence() -> f32 {
    1.0
}

impl Relation {
    /// Create a new relation
    pub fn new(from: Address, to: Address, creator: Address, relation_type: RelationType) -> Self {
        let now = Utc::now();
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            from,
            to,
            by: creator.clone(),
            relation_type,
            content: String::new(),
            creator,
            version: 1,
            signature: String::new(),
            when: now,
            created_at: now,
            confidence: 1.0,
        }
    }

    /// Create a self-referencing relation (for typing fragments)
    pub fn self_reference(from: Address, creator: Address, relation_type: RelationType) -> Self {
        Self::new(from, Address::default(), creator, relation_type)
    }

    /// Set signature
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = signature.into();
        self
    }

    /// Set confidence level (clamped to 0.0 - 1.0)
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Check if this is a self-reference (typing relation)
    pub fn is_self_reference(&self) -> bool {
        self.to.entity.is_empty() || self.from == self.to
    }

    /// Validate the relation data
    pub fn validate(&self) -> Result<(), String> {
        if self.uuid.is_empty() {
            return Err("uuid is required".to_string());
        }
        if self.from.entity.is_empty() {
            return Err("from is required".to_string());
        }
        if self.creator.entity.is_empty() {
            return Err("creator is required".to_string());
        }
        if self.signature.is_empty() {
            return Err("signature is required".to_string());
        }
        Ok(())
    }
}

/// Request to create a new relation
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateRelationRequest {
    pub uuid: Option<String>,
    pub from: Address,
    #[serde(default)]
    pub to: Address,
    /// Agent who asserts this relation
    #[serde(default)]
    pub by: Address,
    #[serde(alias = "relation_type")]
    pub r#type: String,
    /// Optional reasoning/explanation
    #[serde(default)]
    pub content: Option<String>,
    pub creator: Address,
    /// Creation timestamp
    #[serde(default)]
    pub when: Option<DateTime<Utc>>,
    pub signature: String,
    /// Strength of this relationship (0.0 to 1.0)
    #[serde(default)]
    pub confidence: Option<f32>,
}

impl CreateRelationRequest {
    /// Get the relation type string (handles both "type" and "relation_type" fields)
    pub fn relation_type(&self) -> &str {
        &self.r#type
    }
}

impl From<CreateRelationRequest> for Relation {
    fn from(req: CreateRelationRequest) -> Self {
        let uuid = req.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let relation_type = req.r#type.parse().unwrap_or_default();
        let mut relation = Relation::new(req.from, req.to, req.creator, relation_type);
        relation.uuid = uuid;
        relation.signature = req.signature;
        if !req.by.entity.is_empty() {
            relation.by = req.by;
        }
        if let Some(content) = req.content {
            relation.content = content;
        }
        if let Some(when) = req.when {
            relation.when = when;
        }
        if let Some(confidence) = req.confidence {
            relation = relation.with_confidence(confidence);
        }
        relation
    }
}

/// Returns all valid relation types
pub fn valid_relation_types() -> Vec<RelationType> {
    vec![
        RelationType::Trust,
        RelationType::Supports,
        RelationType::Contradicts,
        RelationType::Extends,
        RelationType::Supersedes,
        RelationType::DerivedFrom,
        RelationType::RelatedTo,
        RelationType::ExampleOf,
        RelationType::Specializes,
        RelationType::Clarifies,
        RelationType::Generalizes,
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_relation() {
        let from = Address::fragment("hub:8080", "frag-1");
        let to = Address::fragment("hub:8080", "frag-2");
        let creator = Address::agent("hub:8080", "agent-1");

        let relation = Relation::new(from.clone(), to.clone(), creator.clone(), RelationType::Supports)
            .with_signature("sig");

        assert_eq!(relation.from, from);
        assert_eq!(relation.to, to);
        assert_eq!(relation.relation_type, RelationType::Supports);
    }

    #[test]
    fn test_self_reference() {
        let from = Address::fragment("hub:8080", "frag-1");
        let creator = Address::agent("hub:8080", "agent-1");

        // Self-reference relations now use RELATED_TO to link to TYPE tags
        let relation = Relation::self_reference(from.clone(), creator.clone(), RelationType::RelatedTo)
            .with_signature("sig");

        assert!(relation.is_self_reference());
        assert_eq!(relation.relation_type, RelationType::RelatedTo);
    }

    #[test]
    fn test_relation_type_parsing() {
        assert_eq!(RelationType::from_str("SUPPORTS").unwrap(), RelationType::Supports);
        assert_eq!(RelationType::from_str("derived_from").unwrap(), RelationType::DerivedFrom);
    }
}

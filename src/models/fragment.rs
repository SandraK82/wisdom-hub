//! Fragment model representing knowledge units in the wisdom network

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Address;

/// Evidence type indicating how the fragment's content was derived.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Serialize, Deserialize, Default)]
#[serde(rename_all = "snake_case")]
pub enum EvidenceType {
    /// Observed or tested empirically
    Empirical,
    /// Logically derived from other facts
    Logical,
    /// Agreed upon by multiple sources
    Consensus,
    /// Hypothetical/speculative
    Speculation,
    /// Not specified (default)
    #[default]
    Unknown,
}

impl std::fmt::Display for EvidenceType {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            EvidenceType::Empirical => write!(f, "empirical"),
            EvidenceType::Logical => write!(f, "logical"),
            EvidenceType::Consensus => write!(f, "consensus"),
            EvidenceType::Speculation => write!(f, "speculation"),
            EvidenceType::Unknown => write!(f, "unknown"),
        }
    }
}

impl std::str::FromStr for EvidenceType {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_lowercase().as_str() {
            "empirical" => Ok(EvidenceType::Empirical),
            "logical" => Ok(EvidenceType::Logical),
            "consensus" => Ok(EvidenceType::Consensus),
            "speculation" => Ok(EvidenceType::Speculation),
            "unknown" => Ok(EvidenceType::Unknown),
            _ => Err(format!("Invalid evidence type: {}", s)),
        }
    }
}

/// A knowledge fragment in the wisdom network.
/// Fragments are minimal - typing and state are expressed through Relations.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Fragment {
    /// Unique identifier
    pub uuid: String,
    /// References to Tag entities
    pub tags: Vec<Address>,
    /// Reference to Transform entity (how to interpret content)
    pub transform: Option<Address>,
    /// The actual content
    pub content: String,
    /// SHA-256 hash of content
    pub content_hash: String,
    /// Agent who created this fragment
    pub creator: Address,
    /// Version number
    pub version: u32,
    /// Content timestamp
    pub when: DateTime<Utc>,
    /// Ed25519 signature over the fragment data
    pub signature: String,
    /// When the fragment was created in the system
    pub created_at: DateTime<Utc>,
    /// When the fragment was last updated
    pub updated_at: DateTime<Utc>,
    /// Creator's confidence in this fragment (0.0 to 1.0)
    #[serde(default = "default_confidence")]
    pub confidence: f32,
    /// How the content was derived
    #[serde(default)]
    pub evidence_type: EvidenceType,
}

fn default_confidence() -> f32 {
    0.5
}

impl Fragment {
    /// Create a new fragment
    pub fn new(content: impl Into<String>, creator: Address) -> Self {
        let content_str: String = content.into();
        let content_hash = Self::compute_hash(&content_str);
        let now = Utc::now();
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            tags: Vec::new(),
            transform: None,
            content: content_str,
            content_hash,
            creator,
            version: 1,
            when: now,
            signature: String::new(),
            created_at: now,
            updated_at: now,
            confidence: 0.5,
            evidence_type: EvidenceType::Unknown,
        }
    }

    /// Create with a specific UUID
    pub fn with_uuid(uuid: impl Into<String>, content: impl Into<String>, creator: Address) -> Self {
        let content_str: String = content.into();
        let content_hash = Self::compute_hash(&content_str);
        let now = Utc::now();
        Self {
            uuid: uuid.into(),
            tags: Vec::new(),
            transform: None,
            content: content_str,
            content_hash,
            creator,
            version: 1,
            when: now,
            signature: String::new(),
            created_at: now,
            updated_at: now,
            confidence: 0.5,
            evidence_type: EvidenceType::Unknown,
        }
    }

    /// Compute SHA-256 hash of content
    fn compute_hash(content: &str) -> String {
        use sha2::{Sha256, Digest};
        let mut hasher = Sha256::new();
        hasher.update(content.as_bytes());
        let result = hasher.finalize();
        base64::Engine::encode(&base64::engine::general_purpose::STANDARD, result)
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: Address) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set transform
    pub fn with_transform(mut self, transform: Address) -> Self {
        self.transform = Some(transform);
        self
    }

    /// Set signature
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = signature.into();
        self
    }

    /// Set timestamp
    pub fn with_when(mut self, when: DateTime<Utc>) -> Self {
        self.when = when;
        self
    }

    /// Set confidence level (clamped to 0.0 - 1.0)
    pub fn with_confidence(mut self, confidence: f32) -> Self {
        self.confidence = confidence.clamp(0.0, 1.0);
        self
    }

    /// Set evidence type
    pub fn with_evidence_type(mut self, evidence_type: EvidenceType) -> Self {
        self.evidence_type = evidence_type;
        self
    }

    /// Check if fragment has a tag with the given UUID
    pub fn has_tag(&self, tag_uuid: &str) -> bool {
        self.tags.iter().any(|t| t.entity == tag_uuid)
    }

    /// Validate the fragment data
    pub fn validate(&self) -> Result<(), String> {
        if self.uuid.is_empty() {
            return Err("uuid is required".to_string());
        }
        if self.content.is_empty() {
            return Err("content is required".to_string());
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

/// Request to create a new fragment
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateFragmentRequest {
    pub uuid: Option<String>,
    pub tags: Option<Vec<Address>>,
    pub transform: Option<Address>,
    pub content: String,
    pub creator: Address,
    pub when: Option<DateTime<Utc>>,
    pub signature: String,
    /// Creator's confidence in this fragment (0.0 to 1.0)
    #[serde(default)]
    pub confidence: Option<f32>,
    /// How the content was derived
    #[serde(default)]
    pub evidence_type: Option<EvidenceType>,
}

impl From<CreateFragmentRequest> for Fragment {
    fn from(req: CreateFragmentRequest) -> Self {
        let uuid = req.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut fragment = Fragment::with_uuid(uuid, req.content, req.creator)
            .with_signature(req.signature);

        if let Some(tags) = req.tags {
            for tag in tags {
                fragment = fragment.with_tag(tag);
            }
        }
        if let Some(transform) = req.transform {
            fragment = fragment.with_transform(transform);
        }
        if let Some(when) = req.when {
            fragment = fragment.with_when(when);
        }
        if let Some(confidence) = req.confidence {
            fragment = fragment.with_confidence(confidence);
        }
        if let Some(evidence_type) = req.evidence_type {
            fragment = fragment.with_evidence_type(evidence_type);
        }
        fragment
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_fragment() {
        let creator = Address::agent("hub:8080", "agent-uuid");
        let fragment = Fragment::new("Test content", creator.clone())
            .with_signature("test-sig");

        assert_eq!(fragment.content, "Test content");
        assert_eq!(fragment.creator, creator);
        assert!(!fragment.uuid.is_empty());
    }

    #[test]
    fn test_fragment_with_tags() {
        let creator = Address::agent("hub:8080", "agent-uuid");
        let tag1 = Address::tag("hub:8080", "tag-1");
        let tag2 = Address::tag("hub:8080", "tag-2");

        let fragment = Fragment::new("Content", creator)
            .with_tag(tag1)
            .with_tag(tag2)
            .with_signature("sig");

        assert_eq!(fragment.tags.len(), 2);
        assert!(fragment.has_tag("tag-1"));
        assert!(fragment.has_tag("tag-2"));
    }

    #[test]
    fn test_fragment_with_transform() {
        let creator = Address::agent("hub:8080", "agent-uuid");
        let transform = Address::transformation("hub:8080", "transform-uuid");

        let fragment = Fragment::new("Content", creator)
            .with_transform(transform.clone())
            .with_signature("sig");

        assert_eq!(fragment.transform, Some(transform));
    }
}

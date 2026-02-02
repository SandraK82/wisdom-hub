//! Transform model defining content format transformations

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};

use super::Address;

/// A transform defines how content is transformed between formats.
/// This is used to interpret fragment content consistently.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Transform {
    /// Unique identifier
    pub uuid: String,
    /// Human-readable name
    pub name: String,
    /// What this transform does
    #[serde(default)]
    pub description: String,
    /// Related tags
    #[serde(default)]
    pub tags: Vec<Address>,
    /// Target format (e.g., "text/markdown")
    pub transform_to: String,
    /// Source format (e.g., "text/plain")
    pub transform_from: String,
    /// JSON with extra configuration
    #[serde(default)]
    pub additional_data: String,
    /// Agent who created this transform
    pub agent: Address,
    /// Version number (incremented on updates)
    pub version: u32,
    /// Ed25519 signature over the transform data
    pub signature: String,
    /// When the transform was created
    pub created_at: DateTime<Utc>,
}

impl Transform {
    /// Create a new transform
    pub fn new(
        name: impl Into<String>,
        transform_from: impl Into<String>,
        transform_to: impl Into<String>,
        agent: Address,
    ) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            description: String::new(),
            tags: Vec::new(),
            transform_to: transform_to.into(),
            transform_from: transform_from.into(),
            additional_data: String::new(),
            agent,
            version: 1,
            signature: String::new(),
            created_at: Utc::now(),
        }
    }

    /// Set description
    pub fn with_description(mut self, description: impl Into<String>) -> Self {
        self.description = description.into();
        self
    }

    /// Add a tag
    pub fn with_tag(mut self, tag: Address) -> Self {
        self.tags.push(tag);
        self
    }

    /// Set additional data (JSON)
    pub fn with_additional_data(mut self, data: impl Into<String>) -> Self {
        self.additional_data = data.into();
        self
    }

    /// Set signature
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = signature.into();
        self
    }

    /// Increment version
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Check if transform has a specific tag
    pub fn has_tag(&self, tag_uuid: &str) -> bool {
        self.tags.iter().any(|t| t.entity == tag_uuid)
    }

    /// Validate the transform data
    pub fn validate(&self) -> Result<(), String> {
        if self.uuid.is_empty() {
            return Err("uuid is required".to_string());
        }
        if self.name.is_empty() {
            return Err("name is required".to_string());
        }
        if self.transform_to.is_empty() {
            return Err("transform_to is required".to_string());
        }
        if self.transform_from.is_empty() {
            return Err("transform_from is required".to_string());
        }
        if self.agent.entity.is_empty() {
            return Err("agent is required".to_string());
        }
        if self.signature.is_empty() {
            return Err("signature is required".to_string());
        }
        Ok(())
    }
}

/// Request to create a new transform
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTransformRequest {
    pub uuid: Option<String>,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub tags: Vec<Address>,
    pub transform_to: String,
    pub transform_from: String,
    #[serde(default)]
    pub additional_data: String,
    pub agent: Address,
    pub signature: String,
}

impl From<CreateTransformRequest> for Transform {
    fn from(req: CreateTransformRequest) -> Self {
        let uuid = req.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut transform = Transform::new(req.name, req.transform_from, req.transform_to, req.agent);
        transform.uuid = uuid;
        transform.description = req.description;
        transform.tags = req.tags;
        transform.additional_data = req.additional_data;
        transform.signature = req.signature;
        // created_at is already set in new()
        transform
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_transform() {
        let agent = Address::agent("hub:8080", "agent-uuid");
        let transform = Transform::new(
            "Markdown to HTML",
            "text/markdown",
            "text/html",
            agent.clone(),
        )
        .with_description("Converts markdown to HTML")
        .with_signature("test-sig");

        assert_eq!(transform.name, "Markdown to HTML");
        assert_eq!(transform.transform_from, "text/markdown");
        assert_eq!(transform.transform_to, "text/html");
        assert_eq!(transform.agent, agent);
        assert_eq!(transform.version, 1);
    }

    #[test]
    fn test_transform_with_tags() {
        let agent = Address::agent("hub:8080", "agent-uuid");
        let tag1 = Address::tag("hub:8080", "tag-1");
        let tag2 = Address::tag("hub:8080", "tag-2");

        let transform = Transform::new("Test", "text/plain", "text/markdown", agent)
            .with_tag(tag1)
            .with_tag(tag2)
            .with_signature("sig");

        assert_eq!(transform.tags.len(), 2);
        assert!(transform.has_tag("tag-1"));
        assert!(transform.has_tag("tag-2"));
    }

    #[test]
    fn test_transform_validation() {
        let agent = Address::agent("hub:8080", "agent-uuid");
        let transform = Transform::new("Test", "text/plain", "text/html", agent)
            .with_signature("sig");

        assert!(transform.validate().is_ok());
    }
}

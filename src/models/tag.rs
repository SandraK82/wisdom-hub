//! Tag model for categorizing fragments

use chrono::{DateTime, Utc};
use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;

use super::Address;

/// Tag categories for classification and filtering
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum TagCategory {
    /// Operating system or platform
    Platform,
    /// Programming language
    Language,
    /// Framework
    Framework,
    /// Library or package
    Library,
    /// Version number
    Version,
    /// Domain or topic area
    Domain,
    /// Content type
    Type,
    /// Environment (dev, prod, etc.)
    Environment,
    /// System architecture
    Architecture,
    /// Country or region
    Country,
    /// Field of study or expertise
    Field,
}

impl Default for TagCategory {
    fn default() -> Self {
        TagCategory::Domain
    }
}

impl fmt::Display for TagCategory {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            TagCategory::Platform => write!(f, "PLATFORM"),
            TagCategory::Language => write!(f, "LANGUAGE"),
            TagCategory::Framework => write!(f, "FRAMEWORK"),
            TagCategory::Library => write!(f, "LIBRARY"),
            TagCategory::Version => write!(f, "VERSION"),
            TagCategory::Domain => write!(f, "DOMAIN"),
            TagCategory::Type => write!(f, "TYPE"),
            TagCategory::Environment => write!(f, "ENVIRONMENT"),
            TagCategory::Architecture => write!(f, "ARCHITECTURE"),
            TagCategory::Country => write!(f, "COUNTRY"),
            TagCategory::Field => write!(f, "FIELD"),
        }
    }
}

impl FromStr for TagCategory {
    type Err = String;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "PLATFORM" => Ok(TagCategory::Platform),
            "LANGUAGE" => Ok(TagCategory::Language),
            "FRAMEWORK" => Ok(TagCategory::Framework),
            "LIBRARY" => Ok(TagCategory::Library),
            "VERSION" => Ok(TagCategory::Version),
            "DOMAIN" => Ok(TagCategory::Domain),
            "TYPE" => Ok(TagCategory::Type),
            "ENVIRONMENT" => Ok(TagCategory::Environment),
            "ARCHITECTURE" => Ok(TagCategory::Architecture),
            "COUNTRY" => Ok(TagCategory::Country),
            "FIELD" => Ok(TagCategory::Field),
            _ => Err(format!("Invalid tag category: {}", s)),
        }
    }
}

/// Returns all valid tag categories
pub fn valid_tag_categories() -> Vec<TagCategory> {
    vec![
        TagCategory::Platform,
        TagCategory::Language,
        TagCategory::Framework,
        TagCategory::Library,
        TagCategory::Version,
        TagCategory::Domain,
        TagCategory::Type,
        TagCategory::Environment,
        TagCategory::Architecture,
        TagCategory::Country,
        TagCategory::Field,
    ]
}

/// A tag for categorizing fragments
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Tag {
    /// Unique identifier
    pub uuid: String,
    /// Unique, normalized name
    pub name: String,
    /// Description of the tag
    #[serde(default)]
    pub content: String,
    /// Version number (incremented on updates)
    pub version: u32,
    /// Classification category
    pub category: TagCategory,
    /// Agent who created this tag
    pub creator: Address,
    /// Ed25519 signature over the tag data
    pub signature: String,
    /// When the tag was created
    pub created_at: DateTime<Utc>,
}

impl Tag {
    /// Create a new tag
    pub fn new(name: impl Into<String>, category: TagCategory, creator: Address) -> Self {
        Self {
            uuid: uuid::Uuid::new_v4().to_string(),
            name: name.into(),
            content: String::new(),
            version: 1,
            category,
            creator,
            signature: String::new(),
            created_at: Utc::now(),
        }
    }

    /// Set content/description
    pub fn with_content(mut self, content: impl Into<String>) -> Self {
        self.content = content.into();
        self
    }

    /// Set the signature
    pub fn with_signature(mut self, signature: impl Into<String>) -> Self {
        self.signature = signature.into();
        self
    }

    /// Increment version
    pub fn increment_version(&mut self) {
        self.version += 1;
    }

    /// Validate the tag data
    pub fn validate(&self) -> Result<(), String> {
        if self.uuid.is_empty() {
            return Err("uuid is required".to_string());
        }
        if self.name.is_empty() {
            return Err("name is required".to_string());
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

/// Request to create a new tag
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CreateTagRequest {
    pub uuid: Option<String>,
    pub name: String,
    #[serde(default)]
    pub content: String,
    pub category: TagCategory,
    pub creator: Address,
    pub signature: String,
}

impl From<CreateTagRequest> for Tag {
    fn from(req: CreateTagRequest) -> Self {
        let uuid = req.uuid.unwrap_or_else(|| uuid::Uuid::new_v4().to_string());
        let mut tag = Tag::new(req.name, req.category, req.creator);
        tag.uuid = uuid;
        tag.content = req.content;
        tag.signature = req.signature;
        tag
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_tag() {
        let creator = Address::agent("hub:8080", "agent-uuid");
        let tag = Tag::new("rust", TagCategory::Language, creator.clone())
            .with_content("Rust programming language")
            .with_signature("test-sig");

        assert_eq!(tag.name, "rust");
        assert_eq!(tag.content, "Rust programming language");
        assert_eq!(tag.category, TagCategory::Language);
        assert_eq!(tag.creator, creator);
        assert_eq!(tag.version, 1);
    }

    #[test]
    fn test_tag_category_parsing() {
        assert_eq!(TagCategory::from_str("LANGUAGE").unwrap(), TagCategory::Language);
        assert_eq!(TagCategory::from_str("framework").unwrap(), TagCategory::Framework);
    }

    #[test]
    fn test_tag_validation() {
        let creator = Address::agent("hub:8080", "agent-uuid");
        let tag = Tag::new("rust", TagCategory::Language, creator)
            .with_signature("sig");

        assert!(tag.validate().is_ok());
    }
}

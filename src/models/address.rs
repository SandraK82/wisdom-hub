//! Address model for ATP (Agent Trust Protocol)
//!
//! Addresses identify entities across the distributed network.

use serde::{Deserialize, Serialize};
use std::fmt;
use std::str::FromStr;
use thiserror::Error;

/// Address parsing errors
#[derive(Debug, Error)]
pub enum AddressError {
    #[error("Invalid address format: {0}")]
    InvalidFormat(String),
    #[error("Missing server")]
    MissingServer,
    #[error("Invalid domain: {0}")]
    InvalidDomain(String),
}

/// Domain types for addressing entities
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "SCREAMING_SNAKE_CASE")]
pub enum Domain {
    Agent,
    Tag,
    Fragment,
    Relation,
    Transformation,
    Hub,
}

impl Default for Domain {
    fn default() -> Self {
        Domain::Fragment
    }
}

impl fmt::Display for Domain {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Domain::Agent => write!(f, "AGENT"),
            Domain::Tag => write!(f, "TAG"),
            Domain::Fragment => write!(f, "FRAGMENT"),
            Domain::Relation => write!(f, "RELATION"),
            Domain::Transformation => write!(f, "TRANSFORMATION"),
            Domain::Hub => write!(f, "HUB"),
        }
    }
}

impl FromStr for Domain {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s.to_uppercase().as_str() {
            "AGENT" => Ok(Domain::Agent),
            "TAG" => Ok(Domain::Tag),
            "FRAGMENT" => Ok(Domain::Fragment),
            "RELATION" => Ok(Domain::Relation),
            "TRANSFORMATION" => Ok(Domain::Transformation),
            "HUB" => Ok(Domain::Hub),
            _ => Err(AddressError::InvalidDomain(s.to_string())),
        }
    }
}

/// ATP address representing a resource in the wisdom network
#[derive(Debug, Clone, PartialEq, Eq, Hash, Serialize, Deserialize)]
pub struct Address {
    /// Hub address with port (FQDN or IP)
    pub server_port: String,
    /// Domain type of the entity
    pub domain: Domain,
    /// Entity identifier (typically UUID, empty for HUB domain)
    #[serde(default)]
    pub entity: String,
}

impl Address {
    /// Create a new address
    pub fn new(server_port: impl Into<String>, domain: Domain, entity: impl Into<String>) -> Self {
        Self {
            server_port: server_port.into(),
            domain,
            entity: entity.into(),
        }
    }

    /// Create an address for an agent
    pub fn agent(server_port: impl Into<String>, agent_id: impl Into<String>) -> Self {
        Self::new(server_port, Domain::Agent, agent_id)
    }

    /// Create an address for a fragment
    pub fn fragment(server_port: impl Into<String>, fragment_id: impl Into<String>) -> Self {
        Self::new(server_port, Domain::Fragment, fragment_id)
    }

    /// Create an address for a relation
    pub fn relation(server_port: impl Into<String>, relation_id: impl Into<String>) -> Self {
        Self::new(server_port, Domain::Relation, relation_id)
    }

    /// Create an address for a tag
    pub fn tag(server_port: impl Into<String>, tag_id: impl Into<String>) -> Self {
        Self::new(server_port, Domain::Tag, tag_id)
    }

    /// Create an address for a transform
    pub fn transformation(server_port: impl Into<String>, transform_id: impl Into<String>) -> Self {
        Self::new(server_port, Domain::Transformation, transform_id)
    }

    /// Create an address for a hub
    pub fn hub(server_port: impl Into<String>) -> Self {
        Self::new(server_port, Domain::Hub, "")
    }

    /// Check if this is a local address (same server)
    pub fn is_local(&self, local_server: &str) -> bool {
        self.server_port == local_server
    }

    /// Check if entity is empty (valid for HUB domain)
    pub fn is_hub(&self) -> bool {
        self.domain == Domain::Hub
    }

    /// Parse an address string, returning None on failure
    pub fn parse(s: &str) -> Option<Self> {
        s.parse().ok()
    }
}

impl Default for Address {
    fn default() -> Self {
        Self {
            server_port: String::new(),
            domain: Domain::Hub,
            entity: String::new(),
        }
    }
}

impl fmt::Display for Address {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        if self.entity.is_empty() {
            write!(f, "{}:{}", self.server_port, self.domain)
        } else {
            write!(f, "{}:{}:{}", self.server_port, self.domain, self.entity)
        }
    }
}

impl FromStr for Address {
    type Err = AddressError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let parts: Vec<&str> = s.splitn(3, ':').collect();

        if parts.len() < 2 {
            return Err(AddressError::InvalidFormat(s.to_string()));
        }

        let server_port = if parts.len() >= 2 && parts[1].chars().all(|c| c.is_ascii_digit()) {
            // server:port format
            format!("{}:{}", parts[0], parts[1])
        } else {
            parts[0].to_string()
        };

        let (domain_str, entity) = if parts.len() == 3 {
            (parts[1], parts[2].to_string())
        } else if parts.len() == 2 {
            (parts[1], String::new())
        } else {
            return Err(AddressError::InvalidFormat(s.to_string()));
        };

        let domain = Domain::from_str(domain_str)?;

        Ok(Address {
            server_port,
            domain,
            entity,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_create_address() {
        let addr = Address::agent("hub.wisdom.net:8080", "abc123");
        assert_eq!(addr.server_port, "hub.wisdom.net:8080");
        assert_eq!(addr.domain, Domain::Agent);
        assert_eq!(addr.entity, "abc123");
    }

    #[test]
    fn test_address_display() {
        let addr = Address::fragment("hub.wisdom.net:8080", "xyz789");
        assert_eq!(addr.to_string(), "hub.wisdom.net:8080:FRAGMENT:xyz789");
    }

    #[test]
    fn test_hub_address() {
        let addr = Address::hub("hub.wisdom.net:8080");
        assert!(addr.is_hub());
        assert_eq!(addr.to_string(), "hub.wisdom.net:8080:HUB");
    }

    #[test]
    fn test_domain_parsing() {
        assert_eq!(Domain::from_str("AGENT").unwrap(), Domain::Agent);
        assert_eq!(Domain::from_str("fragment").unwrap(), Domain::Fragment);
    }
}

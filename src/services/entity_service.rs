//! Entity service with signature verification

use std::sync::Arc;

use serde_json::json;

use crate::crypto::{canonical_json, verify_with_key};
use crate::models::{
    Agent, CreateAgentRequest, Fragment, CreateFragmentRequest,
    Relation, CreateRelationRequest, Tag, CreateTagRequest,
    Transform, CreateTransformRequest,
    HubError, HubResult, Domain,
};
use crate::store::{EntityStore, Cursor, ListResult};

/// Entity service handling business logic and validation
#[derive(Clone)]
pub struct EntityService {
    store: Arc<EntityStore>,
    verify_signatures: bool,
}

impl EntityService {
    /// Create a new entity service
    pub fn new(store: Arc<EntityStore>) -> Self {
        Self {
            store,
            verify_signatures: true,
        }
    }

    /// Create service without signature verification (for testing)
    pub fn without_verification(store: Arc<EntityStore>) -> Self {
        Self {
            store,
            verify_signatures: false,
        }
    }

    /// Get the underlying store
    pub fn store(&self) -> &EntityStore {
        &self.store
    }

    // ========================================================================
    // Agent operations
    // ========================================================================

    /// Create a new agent
    pub fn create_agent(&self, req: CreateAgentRequest) -> HubResult<Agent> {
        // Check if public key is valid (basic validation)
        if req.public_key.is_empty() {
            return Err(HubError::InvalidPublicKey("Public key cannot be empty".to_string()));
        }

        // Verify signature if enabled
        if self.verify_signatures {
            self.verify_agent_signature(&req)?;
        }

        let agent = Agent::from(req);
        self.store.put_agent(&agent)?;
        Ok(agent)
    }

    /// Verify agent signature using canonical JSON over all fields
    fn verify_agent_signature(&self, req: &CreateAgentRequest) -> HubResult<()> {
        let uuid = req.uuid.clone().unwrap_or_default();

        let payload = json!({
            "description": req.description.as_deref().unwrap_or(""),
            "primary_hub": req.primary_hub.as_deref().unwrap_or(""),
            "public_key": req.public_key,
            "trust": serde_json::Value::Object(serde_json::Map::new()),
            "uuid": uuid,
        });
        let data = canonical_json(&payload);
        let is_valid = verify_with_key(&req.public_key, data.as_bytes(), &req.signature)?;

        if !is_valid {
            return Err(HubError::InvalidSignature {
                entity_type: "agent".to_string(),
            });
        }

        Ok(())
    }

    /// Get an agent by UUID
    pub fn get_agent(&self, uuid: &str) -> HubResult<Agent> {
        self.store
            .get_agent(uuid)?
            .ok_or_else(|| HubError::NotFound {
                entity_type: "agent".to_string(),
                id: uuid.to_string(),
            })
    }

    /// List agents with pagination
    pub fn list_agents(&self, cursor: Option<&str>, limit: usize) -> HubResult<ListResult<Agent>> {
        let cursor = cursor
            .and_then(|s| Cursor::from_string(s))
            .unwrap_or_else(Cursor::start);

        self.store.list_agents(&cursor, limit.min(100))
    }

    /// Delete an agent
    pub fn delete_agent(&self, uuid: &str) -> HubResult<()> {
        // Check if agent exists
        self.get_agent(uuid)?;
        self.store.delete_agent(uuid)
    }

    // ========================================================================
    // Fragment operations
    // ========================================================================

    /// Create a new fragment with signature verification
    pub fn create_fragment(&self, req: CreateFragmentRequest) -> HubResult<Fragment> {
        // Verify the creating agent exists
        let agent = self.get_agent(&req.creator.entity)?;

        // Verify signature if enabled
        if self.verify_signatures {
            self.verify_fragment_signature(&req, &agent.public_key)?;
        }

        let fragment = Fragment::from(req);
        self.store.put_fragment(&fragment)?;
        Ok(fragment)
    }

    /// Verify fragment signature using canonical JSON over all fields
    fn verify_fragment_signature(&self, req: &CreateFragmentRequest, public_key: &str) -> HubResult<()> {
        let uuid = req.uuid.clone().unwrap_or_default();
        let tags_json: Vec<serde_json::Value> = req.tags.as_ref()
            .map(|t| t.iter().map(|a| serde_json::to_value(a).unwrap()).collect())
            .unwrap_or_default();
        let transform_json = req.transform.as_ref()
            .map(|t| serde_json::to_value(t).unwrap())
            .unwrap_or(serde_json::Value::Null);
        let when_str = req.when.as_ref()
            .map(|w| w.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .unwrap_or_default();

        let payload = json!({
            "confidence": req.confidence.unwrap_or(0.5),
            "content": req.content,
            "creator": serde_json::to_value(&req.creator).unwrap(),
            "evidence_type": req.evidence_type.as_ref().map(|e| e.to_string()).unwrap_or_else(|| "unknown".to_string()),
            "tags": tags_json,
            "transform": transform_json,
            "uuid": uuid,
            "when": when_str,
        });
        let data = canonical_json(&payload);
        let is_valid = verify_with_key(public_key, data.as_bytes(), &req.signature)?;

        if !is_valid {
            return Err(HubError::InvalidSignature {
                entity_type: "fragment".to_string(),
            });
        }

        Ok(())
    }

    /// Get a fragment by UUID
    pub fn get_fragment(&self, uuid: &str) -> HubResult<Fragment> {
        self.store
            .get_fragment(uuid)?
            .ok_or_else(|| HubError::NotFound {
                entity_type: "fragment".to_string(),
                id: uuid.to_string(),
            })
    }

    /// List fragments with pagination
    pub fn list_fragments(&self, cursor: Option<&str>, limit: usize) -> HubResult<ListResult<Fragment>> {
        let cursor = cursor
            .and_then(|s| Cursor::from_string(s))
            .unwrap_or_else(Cursor::start);

        self.store.list_fragments(&cursor, limit.min(100))
    }

    /// Search fragments
    pub fn search_fragments(&self, query: &str, limit: usize) -> HubResult<Vec<Fragment>> {
        self.store.search_fragments(query, limit.min(100))
    }

    /// Delete a fragment
    pub fn delete_fragment(&self, uuid: &str) -> HubResult<()> {
        self.get_fragment(uuid)?;
        self.store.delete_fragment(uuid)
    }

    // ========================================================================
    // Relation operations
    // ========================================================================

    /// Create a new relation with signature verification
    pub fn create_relation(&self, req: CreateRelationRequest) -> HubResult<Relation> {
        // Verify the creating agent exists
        let agent = self.get_agent(&req.creator.entity)?;

        // Verify from and to entities exist
        self.verify_entity_exists(&req.from)?;
        if !req.to.entity.is_empty() {
            self.verify_entity_exists(&req.to)?;
        }

        // Verify signature if enabled
        if self.verify_signatures {
            self.verify_relation_signature(&req, &agent.public_key)?;
        }

        let relation = Relation::from(req);
        self.store.put_relation(&relation)?;
        Ok(relation)
    }

    /// Verify relation signature using canonical JSON over all fields
    fn verify_relation_signature(&self, req: &CreateRelationRequest, public_key: &str) -> HubResult<()> {
        let uuid = req.uuid.clone().unwrap_or_default();
        let when_str = req.when.as_ref()
            .map(|w| w.to_rfc3339_opts(chrono::SecondsFormat::Millis, true))
            .unwrap_or_default();

        let payload = json!({
            "by": serde_json::to_value(&req.by).unwrap(),
            "content": req.content.as_deref().unwrap_or(""),
            "creator": serde_json::to_value(&req.creator).unwrap(),
            "from": serde_json::to_value(&req.from).unwrap(),
            "to": serde_json::to_value(&req.to).unwrap(),
            "type": req.r#type,
            "uuid": uuid,
            "when": when_str,
        });
        let data = canonical_json(&payload);
        let is_valid = verify_with_key(public_key, data.as_bytes(), &req.signature)?;

        if !is_valid {
            return Err(HubError::InvalidSignature {
                entity_type: "relation".to_string(),
            });
        }

        Ok(())
    }

    /// Check if an entity exists based on its address
    fn verify_entity_exists(&self, addr: &crate::models::Address) -> HubResult<()> {
        match addr.domain {
            Domain::Agent => {
                if self.store.get_agent(&addr.entity)?.is_some() {
                    return Ok(());
                }
            }
            Domain::Fragment => {
                if self.store.get_fragment(&addr.entity)?.is_some() {
                    return Ok(());
                }
            }
            Domain::Tag => {
                if self.store.get_tag(&addr.entity)?.is_some() {
                    return Ok(());
                }
            }
            Domain::Transformation => {
                if self.store.get_transform(&addr.entity)?.is_some() {
                    return Ok(());
                }
            }
            Domain::Relation => {
                if self.store.get_relation(&addr.entity)?.is_some() {
                    return Ok(());
                }
            }
            Domain::Hub => {
                // Hub addresses don't need entity validation
                return Ok(());
            }
        }

        Err(HubError::NotFound {
            entity_type: addr.domain.to_string(),
            id: addr.entity.clone(),
        })
    }

    /// Get a relation by UUID
    pub fn get_relation(&self, uuid: &str) -> HubResult<Relation> {
        self.store
            .get_relation(uuid)?
            .ok_or_else(|| HubError::NotFound {
                entity_type: "relation".to_string(),
                id: uuid.to_string(),
            })
    }

    /// List relations with pagination
    pub fn list_relations(&self, cursor: Option<&str>, limit: usize) -> HubResult<ListResult<Relation>> {
        let cursor = cursor
            .and_then(|s| Cursor::from_string(s))
            .unwrap_or_else(Cursor::start);

        self.store.list_relations(&cursor, limit.min(100))
    }

    /// Get relations by source (from)
    pub fn get_relations_by_from(&self, from_entity: &str) -> HubResult<Vec<Relation>> {
        self.store.get_relations_by_from(from_entity)
    }

    /// Get relations by target (to)
    pub fn get_relations_by_to(&self, to_entity: &str) -> HubResult<Vec<Relation>> {
        self.store.get_relations_by_to(to_entity)
    }

    // ========================================================================
    // Tag operations
    // ========================================================================

    /// Create a new tag with signature verification
    pub fn create_tag(&self, req: CreateTagRequest) -> HubResult<Tag> {
        // Verify the creating agent exists
        let agent = self.get_agent(&req.creator.entity)?;

        // Check if tag name already exists
        if self.store.find_tag_by_name(&req.name)?.is_some() {
            return Err(HubError::AlreadyExists {
                entity_type: "tag".to_string(),
                id: req.name.clone(),
            });
        }

        // Verify signature if enabled
        if self.verify_signatures {
            self.verify_tag_signature(&req, &agent.public_key)?;
        }

        let tag = Tag::from(req);
        self.store.put_tag(&tag)?;
        Ok(tag)
    }

    /// Verify tag signature using canonical JSON over all fields
    fn verify_tag_signature(&self, req: &CreateTagRequest, public_key: &str) -> HubResult<()> {
        let uuid = req.uuid.clone().unwrap_or_default();

        let payload = json!({
            "category": req.category.to_string(),
            "content": req.content,
            "creator": serde_json::to_value(&req.creator).unwrap(),
            "name": req.name,
            "uuid": uuid,
        });
        let data = canonical_json(&payload);
        let is_valid = verify_with_key(public_key, data.as_bytes(), &req.signature)?;

        if !is_valid {
            return Err(HubError::InvalidSignature {
                entity_type: "tag".to_string(),
            });
        }

        Ok(())
    }

    /// Get a tag by UUID
    pub fn get_tag(&self, uuid: &str) -> HubResult<Tag> {
        self.store
            .get_tag(uuid)?
            .ok_or_else(|| HubError::NotFound {
                entity_type: "tag".to_string(),
                id: uuid.to_string(),
            })
    }

    /// List tags with pagination
    pub fn list_tags(&self, cursor: Option<&str>, limit: usize) -> HubResult<ListResult<Tag>> {
        let cursor = cursor
            .and_then(|s| Cursor::from_string(s))
            .unwrap_or_else(Cursor::start);

        self.store.list_tags(&cursor, limit.min(100))
    }

    /// Find tag by name
    pub fn find_tag_by_name(&self, name: &str) -> HubResult<Option<Tag>> {
        self.store.find_tag_by_name(name)
    }

    // ========================================================================
    // Transform operations
    // ========================================================================

    /// Create a new transform with signature verification
    pub fn create_transform(&self, req: CreateTransformRequest) -> HubResult<Transform> {
        // Verify the creating agent exists
        let agent = self.get_agent(&req.agent.entity)?;

        // Verify signature if enabled
        if self.verify_signatures {
            self.verify_transform_signature(&req, &agent.public_key)?;
        }

        let transform = Transform::from(req);
        self.store.put_transform(&transform)?;
        Ok(transform)
    }

    /// Verify transform signature using canonical JSON over all fields
    fn verify_transform_signature(&self, req: &CreateTransformRequest, public_key: &str) -> HubResult<()> {
        let uuid = req.uuid.clone().unwrap_or_default();
        let tags_json: Vec<serde_json::Value> = req.tags.iter()
            .map(|a| serde_json::to_value(a).unwrap())
            .collect();

        let payload = json!({
            "additional_data": req.additional_data,
            "agent": serde_json::to_value(&req.agent).unwrap(),
            "description": req.description,
            "name": req.name,
            "tags": tags_json,
            "transform_from": req.transform_from,
            "transform_to": req.transform_to,
            "uuid": uuid,
        });
        let data = canonical_json(&payload);
        let is_valid = verify_with_key(public_key, data.as_bytes(), &req.signature)?;

        if !is_valid {
            return Err(HubError::InvalidSignature {
                entity_type: "transform".to_string(),
            });
        }

        Ok(())
    }

    /// Get a transform by UUID
    pub fn get_transform(&self, uuid: &str) -> HubResult<Transform> {
        self.store
            .get_transform(uuid)?
            .ok_or_else(|| HubError::NotFound {
                entity_type: "transform".to_string(),
                id: uuid.to_string(),
            })
    }

    /// List transforms with pagination
    pub fn list_transforms(&self, cursor: Option<&str>, limit: usize) -> HubResult<ListResult<Transform>> {
        let cursor = cursor
            .and_then(|s| Cursor::from_string(s))
            .unwrap_or_else(Cursor::start);

        self.store.list_transforms(&cursor, limit.min(100))
    }

    // ========================================================================
    // Statistics
    // ========================================================================

    /// Get entity counts
    pub fn get_stats(&self) -> HubResult<EntityStats> {
        Ok(EntityStats {
            agents_count: self.store.count_agents()?,
            fragments_count: self.store.count_fragments()?,
        })
    }
}

/// Entity statistics
#[derive(Debug, Clone)]
pub struct EntityStats {
    pub agents_count: u64,
    pub fragments_count: u64,
}

impl std::fmt::Debug for EntityService {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("EntityService")
            .field("verify_signatures", &self.verify_signatures)
            .finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Address;
    use crate::store::RocksStore;
    use tempfile::TempDir;

    fn create_test_service() -> (EntityService, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let rocks = RocksStore::open(temp_dir.path()).unwrap();
        let store = Arc::new(EntityStore::new(rocks));
        let service = EntityService::without_verification(store);
        (service, temp_dir)
    }

    #[test]
    fn test_create_agent() {
        let (service, _temp) = create_test_service();

        let req = CreateAgentRequest {
            uuid: None,
            public_key: "test-key".to_string(),
            description: Some("Test agent".to_string()),
            primary_hub: None,
            signature: "sig".to_string(),
        };

        let agent = service.create_agent(req).unwrap();
        assert_eq!(agent.public_key, "test-key");

        // Verify we can retrieve it
        let retrieved = service.get_agent(&agent.uuid).unwrap();
        assert_eq!(retrieved.public_key, agent.public_key);
    }

    #[test]
    fn test_create_fragment() {
        let (service, _temp) = create_test_service();

        // First create an agent
        let agent = service.create_agent(CreateAgentRequest {
            uuid: Some("agent-1".to_string()),
            public_key: "test-key".to_string(),
            description: None,
            primary_hub: None,
            signature: "sig".to_string(),
        }).unwrap();

        let creator = Address::agent("hub:8080", &agent.uuid);

        // Create a fragment
        let req = CreateFragmentRequest {
            uuid: None,
            tags: None,
            transform: None,
            content: "Hello, world!".to_string(),
            creator: creator.clone(),
            when: None,
            signature: "sig".to_string(),
        };

        let fragment = service.create_fragment(req).unwrap();
        assert_eq!(fragment.content, "Hello, world!");
        assert_eq!(fragment.creator, creator);
    }

    #[test]
    fn test_list_agents_pagination() {
        let (service, _temp) = create_test_service();

        // Create multiple agents
        for i in 0..5 {
            service.create_agent(CreateAgentRequest {
                uuid: Some(format!("agent-{}", i)),
                public_key: "key".to_string(),
                description: None,
                primary_hub: None,
                signature: "sig".to_string(),
            }).unwrap();
        }

        // List first page
        let result = service.list_agents(None, 3).unwrap();
        assert_eq!(result.items.len(), 3);
        assert!(result.has_more);

        // List second page
        let result2 = service.list_agents(result.next_cursor.as_deref(), 3).unwrap();
        assert_eq!(result2.items.len(), 2);
        assert!(!result2.has_more);
    }

    #[test]
    fn test_get_stats() {
        let (service, _temp) = create_test_service();

        let stats = service.get_stats().unwrap();
        assert_eq!(stats.agents_count, 0);
        assert_eq!(stats.fragments_count, 0);

        // Create some entities
        let agent = service.create_agent(CreateAgentRequest {
            uuid: Some("agent-1".to_string()),
            public_key: "key".to_string(),
            description: None,
            primary_hub: None,
            signature: "sig".to_string(),
        }).unwrap();

        let creator = Address::agent("hub:8080", &agent.uuid);
        service.create_fragment(CreateFragmentRequest {
            uuid: None,
            tags: None,
            transform: None,
            content: "test".to_string(),
            creator,
            when: None,
            signature: "sig".to_string(),
        }).unwrap();

        let stats = service.get_stats().unwrap();
        assert_eq!(stats.agents_count, 1);
        assert_eq!(stats.fragments_count, 1);
    }
}

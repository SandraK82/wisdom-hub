//! Entity storage operations

use super::RocksStore;
use crate::models::{Agent, Fragment, Relation, Tag, Transform, HubResult, HubError};

/// Pagination cursor for list operations
#[derive(Debug, Clone)]
pub struct Cursor {
    pub last_uuid: Option<String>,
}

impl Cursor {
    pub fn start() -> Self {
        Self { last_uuid: None }
    }

    pub fn from_uuid(uuid: String) -> Self {
        Self { last_uuid: Some(uuid) }
    }

    pub fn from_string(s: &str) -> Option<Self> {
        if s.is_empty() {
            return Some(Self::start());
        }
        Some(Self::from_uuid(s.to_string()))
    }

    pub fn to_string(&self) -> String {
        self.last_uuid.clone().unwrap_or_default()
    }
}

/// Paginated list result
#[derive(Debug, Clone)]
pub struct ListResult<T> {
    pub items: Vec<T>,
    pub next_cursor: Option<String>,
    pub has_more: bool,
}

/// Entity store providing CRUD operations for all entity types
#[derive(Clone, Debug)]
pub struct EntityStore {
    rocks: RocksStore,
}

impl EntityStore {
    /// Create a new entity store
    pub fn new(rocks: RocksStore) -> Self {
        Self { rocks }
    }

    /// Get a reference to the underlying RocksStore
    pub fn rocks(&self) -> &RocksStore {
        &self.rocks
    }

    // ========================================================================
    // Agent operations
    // ========================================================================

    /// Store an agent
    pub fn put_agent(&self, agent: &Agent) -> HubResult<()> {
        let cf = self.rocks.cf("agents")?;
        let key = agent.uuid.as_bytes();
        let value = serde_json::to_vec(agent)?;

        self.rocks
            .db()
            .put_cf(cf, key, value)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Get an agent by UUID
    pub fn get_agent(&self, uuid: &str) -> HubResult<Option<Agent>> {
        let cf = self.rocks.cf("agents")?;
        let key = uuid.as_bytes();

        match self.rocks.db().get_cf(cf, key) {
            Ok(Some(value)) => {
                let agent: Agent = serde_json::from_slice(&value)?;
                Ok(Some(agent))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(HubError::DatabaseError(e.to_string())),
        }
    }

    /// List agents with pagination
    pub fn list_agents(&self, cursor: &Cursor, limit: usize) -> HubResult<ListResult<Agent>> {
        self.list_entities("agents", cursor, limit)
    }

    /// Delete an agent
    pub fn delete_agent(&self, uuid: &str) -> HubResult<()> {
        let cf = self.rocks.cf("agents")?;
        let key = uuid.as_bytes();

        self.rocks
            .db()
            .delete_cf(cf, key)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Count all agents
    pub fn count_agents(&self) -> HubResult<u64> {
        self.count_entities("agents")
    }

    // ========================================================================
    // Fragment operations
    // ========================================================================

    /// Store a fragment
    pub fn put_fragment(&self, fragment: &Fragment) -> HubResult<()> {
        let cf = self.rocks.cf("fragments")?;
        let key = fragment.uuid.as_bytes();
        let value = serde_json::to_vec(fragment)?;

        self.rocks
            .db()
            .put_cf(cf, key, value)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Get a fragment by UUID
    pub fn get_fragment(&self, uuid: &str) -> HubResult<Option<Fragment>> {
        let cf = self.rocks.cf("fragments")?;
        let key = uuid.as_bytes();

        match self.rocks.db().get_cf(cf, key) {
            Ok(Some(value)) => {
                let fragment: Fragment = serde_json::from_slice(&value)?;
                Ok(Some(fragment))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(HubError::DatabaseError(e.to_string())),
        }
    }

    /// List fragments with pagination
    pub fn list_fragments(&self, cursor: &Cursor, limit: usize) -> HubResult<ListResult<Fragment>> {
        self.list_entities("fragments", cursor, limit)
    }

    /// Delete a fragment
    pub fn delete_fragment(&self, uuid: &str) -> HubResult<()> {
        let cf = self.rocks.cf("fragments")?;
        let key = uuid.as_bytes();

        self.rocks
            .db()
            .delete_cf(cf, key)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Count all fragments
    pub fn count_fragments(&self) -> HubResult<u64> {
        self.count_entities("fragments")
    }

    /// Search fragments by content (simple substring match)
    pub fn search_fragments(&self, query: &str, limit: usize) -> HubResult<Vec<Fragment>> {
        let cf = self.rocks.cf("fragments")?;
        let iter = self.rocks.db().iterator_cf(cf, rocksdb::IteratorMode::Start);
        let query_lower = query.to_lowercase();

        let mut results = Vec::new();
        for item in iter {
            if results.len() >= limit {
                break;
            }
            let (_, value) = item.map_err(|e| HubError::DatabaseError(e.to_string()))?;
            let fragment: Fragment = serde_json::from_slice(&value)?;

            // Search in content
            if fragment.content.to_lowercase().contains(&query_lower) {
                results.push(fragment);
            }
        }

        Ok(results)
    }

    // ========================================================================
    // Relation operations
    // ========================================================================

    /// Store a relation
    pub fn put_relation(&self, relation: &Relation) -> HubResult<()> {
        let cf = self.rocks.cf("relations")?;
        let key = relation.uuid.as_bytes();
        let value = serde_json::to_vec(relation)?;

        self.rocks
            .db()
            .put_cf(cf, key, value)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Get a relation by UUID
    pub fn get_relation(&self, uuid: &str) -> HubResult<Option<Relation>> {
        let cf = self.rocks.cf("relations")?;
        let key = uuid.as_bytes();

        match self.rocks.db().get_cf(cf, key) {
            Ok(Some(value)) => {
                let relation: Relation = serde_json::from_slice(&value)?;
                Ok(Some(relation))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(HubError::DatabaseError(e.to_string())),
        }
    }

    /// List relations with pagination
    pub fn list_relations(&self, cursor: &Cursor, limit: usize) -> HubResult<ListResult<Relation>> {
        self.list_entities("relations", cursor, limit)
    }

    /// Delete a relation
    pub fn delete_relation(&self, uuid: &str) -> HubResult<()> {
        let cf = self.rocks.cf("relations")?;
        let key = uuid.as_bytes();

        self.rocks
            .db()
            .delete_cf(cf, key)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Get relations by source entity (from address)
    pub fn get_relations_by_from(&self, from_entity: &str) -> HubResult<Vec<Relation>> {
        let cf = self.rocks.cf("relations")?;
        let iter = self.rocks.db().iterator_cf(cf, rocksdb::IteratorMode::Start);

        let mut results = Vec::new();
        for item in iter {
            let (_, value) = item.map_err(|e| HubError::DatabaseError(e.to_string()))?;
            let relation: Relation = serde_json::from_slice(&value)?;
            if relation.from.entity == from_entity {
                results.push(relation);
            }
        }

        Ok(results)
    }

    /// Get relations by target entity (to address)
    pub fn get_relations_by_to(&self, to_entity: &str) -> HubResult<Vec<Relation>> {
        let cf = self.rocks.cf("relations")?;
        let iter = self.rocks.db().iterator_cf(cf, rocksdb::IteratorMode::Start);

        let mut results = Vec::new();
        for item in iter {
            let (_, value) = item.map_err(|e| HubError::DatabaseError(e.to_string()))?;
            let relation: Relation = serde_json::from_slice(&value)?;
            if relation.to.entity == to_entity {
                results.push(relation);
            }
        }

        Ok(results)
    }

    // ========================================================================
    // Tag operations
    // ========================================================================

    /// Store a tag
    pub fn put_tag(&self, tag: &Tag) -> HubResult<()> {
        let cf = self.rocks.cf("tags")?;
        let key = tag.uuid.as_bytes();
        let value = serde_json::to_vec(tag)?;

        self.rocks
            .db()
            .put_cf(cf, key, value)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Get a tag by UUID
    pub fn get_tag(&self, uuid: &str) -> HubResult<Option<Tag>> {
        let cf = self.rocks.cf("tags")?;
        let key = uuid.as_bytes();

        match self.rocks.db().get_cf(cf, key) {
            Ok(Some(value)) => {
                let tag: Tag = serde_json::from_slice(&value)?;
                Ok(Some(tag))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(HubError::DatabaseError(e.to_string())),
        }
    }

    /// List tags with pagination
    pub fn list_tags(&self, cursor: &Cursor, limit: usize) -> HubResult<ListResult<Tag>> {
        self.list_entities("tags", cursor, limit)
    }

    /// Delete a tag
    pub fn delete_tag(&self, uuid: &str) -> HubResult<()> {
        let cf = self.rocks.cf("tags")?;
        let key = uuid.as_bytes();

        self.rocks
            .db()
            .delete_cf(cf, key)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Find tag by name
    pub fn find_tag_by_name(&self, name: &str) -> HubResult<Option<Tag>> {
        let cf = self.rocks.cf("tags")?;
        let iter = self.rocks.db().iterator_cf(cf, rocksdb::IteratorMode::Start);

        for item in iter {
            let (_, value) = item.map_err(|e| HubError::DatabaseError(e.to_string()))?;
            let tag: Tag = serde_json::from_slice(&value)?;
            if tag.name == name {
                return Ok(Some(tag));
            }
        }

        Ok(None)
    }

    // ========================================================================
    // Transform operations
    // ========================================================================

    /// Store a transform
    pub fn put_transform(&self, transform: &Transform) -> HubResult<()> {
        let cf = self.rocks.cf("transforms")?;
        let key = transform.uuid.as_bytes();
        let value = serde_json::to_vec(transform)?;

        self.rocks
            .db()
            .put_cf(cf, key, value)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    /// Get a transform by UUID
    pub fn get_transform(&self, uuid: &str) -> HubResult<Option<Transform>> {
        let cf = self.rocks.cf("transforms")?;
        let key = uuid.as_bytes();

        match self.rocks.db().get_cf(cf, key) {
            Ok(Some(value)) => {
                let transform: Transform = serde_json::from_slice(&value)?;
                Ok(Some(transform))
            }
            Ok(None) => Ok(None),
            Err(e) => Err(HubError::DatabaseError(e.to_string())),
        }
    }

    /// List transforms with pagination
    pub fn list_transforms(&self, cursor: &Cursor, limit: usize) -> HubResult<ListResult<Transform>> {
        self.list_entities("transforms", cursor, limit)
    }

    /// Delete a transform
    pub fn delete_transform(&self, uuid: &str) -> HubResult<()> {
        let cf = self.rocks.cf("transforms")?;
        let key = uuid.as_bytes();

        self.rocks
            .db()
            .delete_cf(cf, key)
            .map_err(|e| HubError::DatabaseError(e.to_string()))
    }

    // ========================================================================
    // Generic helper methods
    // ========================================================================

    /// Generic list operation for any entity type
    fn list_entities<T: serde::de::DeserializeOwned + HasUuid>(
        &self,
        cf_name: &str,
        cursor: &Cursor,
        limit: usize,
    ) -> HubResult<ListResult<T>> {
        let cf = self.rocks.cf(cf_name)?;

        let iter = match &cursor.last_uuid {
            Some(uuid) => {
                // Start after the cursor UUID
                let start_key = uuid.as_bytes().to_vec();
                self.rocks.db().iterator_cf(
                    cf,
                    rocksdb::IteratorMode::From(&start_key, rocksdb::Direction::Forward),
                )
            }
            None => self.rocks.db().iterator_cf(cf, rocksdb::IteratorMode::Start),
        };

        let mut items = Vec::new();
        let mut skipped_first = cursor.last_uuid.is_none();

        for item in iter {
            let (key, value) = item.map_err(|e| HubError::DatabaseError(e.to_string()))?;

            // Skip the cursor item itself
            if !skipped_first {
                if let Some(cursor_uuid) = &cursor.last_uuid {
                    if key.as_ref() == cursor_uuid.as_bytes() {
                        skipped_first = true;
                        continue;
                    }
                }
                skipped_first = true;
            }

            if items.len() >= limit + 1 {
                break;
            }

            let entity: T = serde_json::from_slice(&value)?;
            items.push(entity);
        }

        let has_more = items.len() > limit;
        if has_more {
            items.pop();
        }

        let next_cursor = if has_more {
            items.last().map(|e| e.uuid().to_string())
        } else {
            None
        };

        Ok(ListResult {
            items,
            next_cursor,
            has_more,
        })
    }

    /// Count entities in a column family
    fn count_entities(&self, cf_name: &str) -> HubResult<u64> {
        let cf = self.rocks.cf(cf_name)?;
        let iter = self.rocks.db().iterator_cf(cf, rocksdb::IteratorMode::Start);

        let mut count = 0u64;
        for item in iter {
            let _ = item.map_err(|e| HubError::DatabaseError(e.to_string()))?;
            count += 1;
        }

        Ok(count)
    }
}

/// Trait for entities that have a UUID
pub trait HasUuid {
    fn uuid(&self) -> &str;
}

impl HasUuid for Agent {
    fn uuid(&self) -> &str {
        &self.uuid
    }
}

impl HasUuid for Fragment {
    fn uuid(&self) -> &str {
        &self.uuid
    }
}

impl HasUuid for Relation {
    fn uuid(&self) -> &str {
        &self.uuid
    }
}

impl HasUuid for Tag {
    fn uuid(&self) -> &str {
        &self.uuid
    }
}

impl HasUuid for Transform {
    fn uuid(&self) -> &str {
        &self.uuid
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::models::Address;
    use tempfile::TempDir;

    fn create_test_store() -> (EntityStore, TempDir) {
        let temp_dir = TempDir::new().unwrap();
        let rocks = RocksStore::open(temp_dir.path()).unwrap();
        let store = EntityStore::new(rocks);
        (store, temp_dir)
    }

    #[test]
    fn test_agent_crud() {
        let (store, _temp) = create_test_store();
        let agent = Agent::new("test-uuid", "test-public-key")
            .with_signature("sig");

        // Create
        store.put_agent(&agent).unwrap();

        // Read
        let retrieved = store.get_agent(&agent.uuid).unwrap().unwrap();
        assert_eq!(retrieved.uuid, agent.uuid);

        // Delete
        store.delete_agent(&agent.uuid).unwrap();
        assert!(store.get_agent(&agent.uuid).unwrap().is_none());
    }

    #[test]
    fn test_fragment_crud() {
        let (store, _temp) = create_test_store();
        let creator = Address::agent("hub:8080", "agent-uuid");
        let fragment = Fragment::new("Hello, world!", creator)
            .with_signature("sig");

        // Create
        store.put_fragment(&fragment).unwrap();

        // Read
        let retrieved = store.get_fragment(&fragment.uuid).unwrap().unwrap();
        assert_eq!(retrieved.content, fragment.content);
    }

    #[test]
    fn test_list_agents() {
        let (store, _temp) = create_test_store();

        // Create multiple agents
        for i in 0..5 {
            let agent = Agent::new(format!("uuid-{}", i), "key")
                .with_signature("sig");
            store.put_agent(&agent).unwrap();
        }

        // List with pagination
        let result = store.list_agents(&Cursor::start(), 3).unwrap();
        assert_eq!(result.items.len(), 3);
        assert!(result.has_more);

        // Get next page
        let cursor = Cursor::from_string(&result.next_cursor.unwrap()).unwrap();
        let result2 = store.list_agents(&cursor, 3).unwrap();
        assert_eq!(result2.items.len(), 2);
        assert!(!result2.has_more);
    }

    #[test]
    fn test_search_fragments() {
        let (store, _temp) = create_test_store();
        let creator = Address::agent("hub:8080", "agent-uuid");

        store.put_fragment(&Fragment::new("Rust is awesome", creator.clone()).with_signature("s")).unwrap();
        store.put_fragment(&Fragment::new("Python is great", creator.clone()).with_signature("s")).unwrap();
        store.put_fragment(&Fragment::new("Rust performance", creator).with_signature("s")).unwrap();

        let results = store.search_fragments("rust", 10).unwrap();
        assert_eq!(results.len(), 2);
    }

    #[test]
    fn test_count_entities() {
        let (store, _temp) = create_test_store();

        assert_eq!(store.count_agents().unwrap(), 0);

        for i in 0..3 {
            store.put_agent(&Agent::new(format!("uuid-{}", i), "key").with_signature("s")).unwrap();
        }

        assert_eq!(store.count_agents().unwrap(), 3);
    }
}

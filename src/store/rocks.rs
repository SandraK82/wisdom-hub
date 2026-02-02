//! RocksDB wrapper for entity storage
//!
//! Will be fully implemented in Phase 2.

use std::path::Path;
use std::sync::Arc;

use crate::models::{HubError, HubResult};

/// RocksDB storage backend
pub struct RocksStore {
    #[allow(dead_code)]
    db: Arc<rocksdb::DB>,
}

impl RocksStore {
    /// Open or create a RocksDB database
    pub fn open<P: AsRef<Path>>(path: P) -> HubResult<Self> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Column families for different entity types
        let cfs = vec![
            "agents",
            "fragments",
            "relations",
            "tags",
            "transforms",
            "trust_relations",
            "sync_log",
        ];

        let db = rocksdb::DB::open_cf(&opts, path, cfs)
            .map_err(|e| HubError::DatabaseError(e.to_string()))?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Open with custom options
    pub fn open_with_opts<P: AsRef<Path>>(
        path: P,
        cache_size_mb: usize,
        compression: bool,
    ) -> HubResult<Self> {
        let mut opts = rocksdb::Options::default();
        opts.create_if_missing(true);
        opts.create_missing_column_families(true);

        // Set block cache
        let mut block_opts = rocksdb::BlockBasedOptions::default();
        block_opts.set_block_cache(&rocksdb::Cache::new_lru_cache(cache_size_mb * 1024 * 1024));
        opts.set_block_based_table_factory(&block_opts);

        // Set compression
        if compression {
            opts.set_compression_type(rocksdb::DBCompressionType::Lz4);
        }

        // Column families
        let cfs = vec![
            "agents",
            "fragments",
            "relations",
            "tags",
            "transforms",
            "trust_relations",
            "sync_log",
        ];

        let db = rocksdb::DB::open_cf(&opts, path, cfs)
            .map_err(|e| HubError::DatabaseError(e.to_string()))?;

        Ok(Self { db: Arc::new(db) })
    }

    /// Get a reference to the underlying database
    pub fn db(&self) -> &rocksdb::DB {
        &self.db
    }

    /// Get a column family handle
    pub fn cf(&self, name: &str) -> HubResult<&rocksdb::ColumnFamily> {
        self.db
            .cf_handle(name)
            .ok_or_else(|| HubError::DatabaseError(format!("Column family not found: {}", name)))
    }
}

impl Clone for RocksStore {
    fn clone(&self) -> Self {
        Self {
            db: Arc::clone(&self.db),
        }
    }
}

impl std::fmt::Debug for RocksStore {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("RocksStore").finish()
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use tempfile::TempDir;

    #[test]
    fn test_open_database() {
        let temp_dir = TempDir::new().unwrap();
        let store = RocksStore::open(temp_dir.path()).unwrap();

        // Verify column families exist
        assert!(store.cf("agents").is_ok());
        assert!(store.cf("fragments").is_ok());
        assert!(store.cf("relations").is_ok());
    }
}

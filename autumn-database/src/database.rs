use sqlx::{PgPool, migrate::Migrator};

use crate::cache::CacheService;

/// Compile-time discovered SQLx migrations for the `autumn-database` crate.
pub static MIGRATOR: Migrator = sqlx::migrate!();

/// Shared database handle passed across crates.
#[derive(Clone, Debug)]
pub struct Database {
    pool: PgPool,
    cache: CacheService,
}

impl Database {
    /// Create a database handle from an existing pool.
    pub fn new(pool: PgPool) -> Self {
        Self {
            pool,
            cache: CacheService::disabled("autumn:prod"),
        }
    }

    /// Create a database handle from an existing pool and cache service.
    pub fn with_cache(pool: PgPool, cache: CacheService) -> Self {
        Self { pool, cache }
    }

    /// Expose the underlying pool for query modules.
    pub fn pool(&self) -> &PgPool {
        &self.pool
    }

    /// Expose the cache service for query modules.
    pub fn cache(&self) -> &CacheService {
        &self.cache
    }
}

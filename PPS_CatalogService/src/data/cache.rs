use crate::{
    application::ProductRepository,
    domain::{CreateProduct, Product, ProductId, ProductPage, ProductQuery, UpdateProduct},
    AppError,
};
use async_trait::async_trait;
use bb8::{ManageConnection, Pool};
use redis::{
    aio::{ConnectionLike, MultiplexedConnection},
    cluster::ClusterClient,
    cluster_async::ClusterConnection,
    RedisError,
};
use std::{sync::Arc, time::Duration};
use tokio::time::timeout;
use tracing::warn;

const PRODUCT_CACHE_TTL_SECONDS: u64 = 60 * 60;
const DEFAULT_CACHE_OPERATION_TIMEOUT: Duration = Duration::from_millis(250);

#[derive(Clone)]
pub struct RedisClusterManager {
    client: ClusterClient,
}

impl RedisClusterManager {
    pub fn new(nodes: Vec<String>) -> Result<Self, RedisError> {
        ClusterClient::new(nodes).map(|client| Self { client })
    }
}

impl ManageConnection for RedisClusterManager {
    type Connection = ClusterConnection;
    type Error = RedisError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.client.get_async_connection().await
    }

    async fn is_valid(&self, connection: &mut Self::Connection) -> Result<(), Self::Error> {
        redis::cmd("PING").query_async(connection).await
    }

    fn has_broken(&self, _connection: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Clone)]
pub struct RedisStandaloneManager {
    client: redis::Client,
}

impl RedisStandaloneManager {
    pub fn new(url: &str) -> Result<Self, RedisError> {
        redis::Client::open(url).map(|client| Self { client })
    }
}

impl ManageConnection for RedisStandaloneManager {
    type Connection = MultiplexedConnection;
    type Error = RedisError;

    async fn connect(&self) -> Result<Self::Connection, Self::Error> {
        self.client.get_multiplexed_async_connection().await
    }

    async fn is_valid(&self, connection: &mut Self::Connection) -> Result<(), Self::Error> {
        redis::cmd("PING").query_async(connection).await
    }

    fn has_broken(&self, _connection: &mut Self::Connection) -> bool {
        false
    }
}

#[derive(Clone)]
enum RedisPool {
    Cluster(Pool<RedisClusterManager>),
    Standalone(Pool<RedisStandaloneManager>),
}

/// A bounded bb8 pool of asynchronous Redis Cluster connections.
#[derive(Clone)]
pub struct ProductCache {
    pool: RedisPool,
    operation_timeout: Duration,
}

impl ProductCache {
    pub fn connect(mode: &str, nodes: Vec<String>, max_connections: u32) -> Result<Self, AppError> {
        if nodes.is_empty() {
            return Err(AppError::Configuration(
                "REDIS_CLUSTER_URLS must contain at least one node".into(),
            ));
        }
        let pool = match mode {
            "cluster" => {
                let manager = RedisClusterManager::new(nodes).map_err(invalid_redis_config)?;
                RedisPool::Cluster(redis_pool(manager, max_connections))
            }
            "standalone" => {
                if nodes.len() != 1 {
                    return Err(AppError::Configuration(
                        "standalone Redis mode requires exactly one URL".into(),
                    ));
                }
                let manager =
                    RedisStandaloneManager::new(&nodes[0]).map_err(invalid_redis_config)?;
                RedisPool::Standalone(redis_pool(manager, max_connections))
            }
            value => {
                return Err(AppError::Configuration(format!(
                    "unsupported REDIS_MODE {value:?}; expected standalone or cluster"
                )))
            }
        };
        Ok(Self {
            pool,
            operation_timeout: DEFAULT_CACHE_OPERATION_TIMEOUT,
        })
    }

    fn product_key(id: &ProductId) -> String {
        format!("product:{{{}}}", id.0)
    }

    async fn get_cached_product(&self, id: &ProductId) -> Result<Option<Product>, CacheError> {
        let key = Self::product_key(id);
        let operation = async {
            match &self.pool {
                RedisPool::Cluster(pool) => {
                    let mut connection = pool.get().await?;
                    get_value(&mut *connection, &key).await
                }
                RedisPool::Standalone(pool) => {
                    let mut connection = pool.get().await?;
                    get_value(&mut *connection, &key).await
                }
            }
        };
        let bytes = timeout(self.operation_timeout, operation)
            .await
            .map_err(|_| CacheError::Timeout("read"))??;
        let Some(bytes) = bytes else {
            return Ok(None);
        };
        match serde_json::from_slice(&bytes) {
            Ok(product) => Ok(Some(product)),
            Err(error) => {
                if let Err(eviction_error) = self.evict_product(id).await {
                    warn!(cache_key = %key, error = %eviction_error, "invalid cached product could not be evicted");
                }
                Err(CacheError::Serialization(error))
            }
        }
    }

    async fn put_cached_product(&self, product: &Product) -> Result<(), CacheError> {
        let key = Self::product_key(&product.id);
        let bytes = serde_json::to_vec(product)?;
        let operation = async {
            match &self.pool {
                RedisPool::Cluster(pool) => {
                    let mut connection = pool.get().await?;
                    set_value(&mut *connection, &key, &bytes).await
                }
                RedisPool::Standalone(pool) => {
                    let mut connection = pool.get().await?;
                    set_value(&mut *connection, &key, &bytes).await
                }
            }
        };
        timeout(self.operation_timeout, operation)
            .await
            .map_err(|_| CacheError::Timeout("backfill"))??;
        Ok(())
    }

    /// Evicts a product after the primary database commit. Eviction failures are
    /// logged and fail open so Redis cannot make successful writes appear failed.
    async fn evict_product(&self, id: &ProductId) -> Result<(), CacheError> {
        let key = Self::product_key(id);
        let operation = async {
            match &self.pool {
                RedisPool::Cluster(pool) => {
                    let mut connection = pool.get().await?;
                    delete_value(&mut *connection, &key).await
                }
                RedisPool::Standalone(pool) => {
                    let mut connection = pool.get().await?;
                    delete_value(&mut *connection, &key).await
                }
            }
        };
        timeout(self.operation_timeout, operation)
            .await
            .map_err(|_| CacheError::Timeout("eviction"))??;
        Ok(())
    }
}

fn redis_pool<M: ManageConnection>(manager: M, max_connections: u32) -> Pool<M> {
    Pool::builder()
        .max_size(max_connections.max(1))
        .connection_timeout(Duration::from_secs(2))
        .build_unchecked(manager)
}

fn invalid_redis_config(error: RedisError) -> AppError {
    AppError::Configuration(format!("invalid Redis configuration: {error}"))
}

async fn get_value<C: ConnectionLike + Send>(
    connection: &mut C,
    key: &str,
) -> Result<Option<Vec<u8>>, CacheError> {
    Ok(redis::cmd("GET").arg(key).query_async(connection).await?)
}

async fn set_value<C: ConnectionLike + Send>(
    connection: &mut C,
    key: &str,
    value: &[u8],
) -> Result<(), CacheError> {
    redis::cmd("SET")
        .arg(key)
        .arg(value)
        .arg("EX")
        .arg(PRODUCT_CACHE_TTL_SECONDS)
        .query_async::<()>(connection)
        .await?;
    Ok(())
}

async fn delete_value<C: ConnectionLike + Send>(
    connection: &mut C,
    key: &str,
) -> Result<(), CacheError> {
    redis::cmd("DEL")
        .arg(key)
        .query_async::<u64>(connection)
        .await?;
    Ok(())
}

#[async_trait]
pub trait ProductCacheBackend: Send + Sync {
    async fn get_product(&self, id: &ProductId) -> Result<Option<Product>, CacheError>;
    async fn put_product(&self, product: &Product) -> Result<(), CacheError>;
    async fn delete_product_cache(&self, id: &ProductId) -> Result<(), CacheError>;
}

#[async_trait]
impl ProductCacheBackend for ProductCache {
    async fn get_product(&self, id: &ProductId) -> Result<Option<Product>, CacheError> {
        self.get_cached_product(id).await
    }

    async fn put_product(&self, product: &Product) -> Result<(), CacheError> {
        self.put_cached_product(product).await
    }

    async fn delete_product_cache(&self, id: &ProductId) -> Result<(), CacheError> {
        self.evict_product(id).await
    }
}

#[derive(Debug, thiserror::Error)]
pub enum CacheError {
    #[error("pool error: {0}")]
    Pool(#[from] bb8::RunError<RedisError>),
    #[error("Redis error: {0}")]
    Redis(#[from] RedisError),
    #[error("cache serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Redis cache {0} timed out")]
    Timeout(&'static str),
}

/// Cache-aside decorator for the primary product repository.
pub struct CachedProductRepository {
    primary: Arc<dyn ProductRepository>,
    cache: Option<Arc<dyn ProductCacheBackend>>,
}

impl CachedProductRepository {
    pub fn new(
        primary: Arc<dyn ProductRepository>,
        cache: Option<Arc<dyn ProductCacheBackend>>,
    ) -> Self {
        Self { primary, cache }
    }

    /// Checks Redis first, then loads from the primary database and backfills
    /// Redis with a one-hour TTL on a miss or recoverable cache failure.
    pub async fn get_product(&self, id: ProductId) -> Result<Option<Product>, AppError> {
        if let Some(cache) = &self.cache {
            match cache.get_product(&id).await {
                Ok(Some(product)) => return Ok(Some(product)),
                Ok(None) => {}
                Err(error) => {
                    warn!(product_id = %id.0, error = %error, "Redis product read failed; using database");
                }
            }
        }
        let product = self.primary.get(id).await?;
        if let (Some(cache), Some(product)) = (&self.cache, product.as_ref()) {
            if let Err(error) = cache.put_product(product).await {
                warn!(product_id = %product.id.0, error = %error, "Redis product backfill failed; returning database result");
            }
        }
        Ok(product)
    }

    pub async fn delete_product_cache(&self, id: &ProductId) -> Result<(), CacheError> {
        if let Some(cache) = &self.cache {
            cache.delete_product_cache(id).await?;
        }
        Ok(())
    }
}

#[async_trait]
impl ProductRepository for CachedProductRepository {
    async fn list_active(&self) -> Result<Vec<Product>, AppError> {
        self.primary.list_active().await
    }

    async fn search(&self, query: ProductQuery) -> Result<ProductPage, AppError> {
        self.primary.search(query).await
    }

    async fn get(&self, id: ProductId) -> Result<Option<Product>, AppError> {
        self.get_product(id).await
    }

    async fn create(&self, input: CreateProduct) -> Result<Product, AppError> {
        self.primary.create(input).await
    }

    async fn update(
        &self,
        id: ProductId,
        input: UpdateProduct,
    ) -> Result<Option<Product>, AppError> {
        let product = self.primary.update(id.clone(), input).await?;
        if product.is_some() {
            if let Err(error) = self.delete_product_cache(&id).await {
                warn!(product_id = %id.0, error = %error, "Redis product eviction failed after update");
            }
        }
        Ok(product)
    }

    async fn deactivate(&self, id: ProductId) -> Result<bool, AppError> {
        let deactivated = self.primary.deactivate(id.clone()).await?;
        if deactivated {
            if let Err(error) = self.delete_product_cache(&id).await {
                warn!(product_id = %id.0, error = %error, "Redis product eviction failed after deactivation");
            }
        }
        Ok(deactivated)
    }
}

#[cfg(test)]
mod tests {
    use super::{
        CacheError, CachedProductRepository, ProductCache, ProductCacheBackend,
        PRODUCT_CACHE_TTL_SECONDS,
    };
    use crate::{
        application::ProductRepository,
        domain::{CreateProduct, Product, ProductId, ProductPage, ProductQuery, UpdateProduct},
        AppError,
    };
    use async_trait::async_trait;
    use std::sync::{
        atomic::{AtomicUsize, Ordering},
        Arc,
    };

    #[test]
    fn product_keys_use_cluster_hash_tags() {
        assert_eq!(
            ProductCache::product_key(&ProductId("507f1f77bcf86cd799439011".into())),
            "product:{507f1f77bcf86cd799439011}"
        );
    }

    #[tokio::test]
    async fn cache_hit_skips_primary_database() {
        let product = product();
        let primary = Arc::new(FakeRepository::new(product.clone()));
        let cache = Arc::new(FakeCache::with_hit(product));
        let repository = CachedProductRepository::new(primary.clone(), Some(cache));

        let result = repository
            .get_product(ProductId("product-1".into()))
            .await
            .expect("cache-aside read should succeed");

        assert!(result.is_some());
        assert_eq!(primary.gets.load(Ordering::Relaxed), 0);
    }

    #[tokio::test]
    async fn cache_miss_reads_primary_and_backfills() {
        let primary = Arc::new(FakeRepository::new(product()));
        let cache = Arc::new(FakeCache::miss());
        let repository = CachedProductRepository::new(primary.clone(), Some(cache.clone()));

        let result = repository
            .get_product(ProductId("product-1".into()))
            .await
            .expect("database fallback should succeed");

        assert!(result.is_some());
        assert_eq!(primary.gets.load(Ordering::Relaxed), 1);
        assert_eq!(cache.puts.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn successful_update_evicts_cached_product() {
        let primary = Arc::new(FakeRepository::new(product()));
        let cache = Arc::new(FakeCache::miss());
        let repository = CachedProductRepository::new(primary, Some(cache.clone()));

        let result = repository
            .update(ProductId("product-1".into()), UpdateProduct::default())
            .await
            .expect("database update should succeed");

        assert!(result.is_some());
        assert_eq!(cache.evictions.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    async fn cache_failure_fails_open_to_primary_database() {
        let primary = Arc::new(FakeRepository::new(product()));
        let cache = Arc::new(ErrorCache);
        let repository = CachedProductRepository::new(primary.clone(), Some(cache));

        let result = repository
            .get_product(ProductId("product-1".into()))
            .await
            .expect("database fallback should succeed");

        assert!(result.is_some());
        assert_eq!(primary.gets.load(Ordering::Relaxed), 1);
    }

    #[tokio::test]
    #[ignore = "requires the local Redis service from compose.yaml"]
    async fn standalone_redis_round_trip_ttl_and_eviction() {
        let url = "redis://127.0.0.1:6379";
        let cache = ProductCache::connect("standalone", vec![url.into()], 2)
            .expect("standalone cache configuration should be valid");
        let product = product();
        cache
            .put_product(&product)
            .await
            .expect("product should be cached");

        let cached = cache
            .get_product(&product.id)
            .await
            .expect("cached product should be readable");
        assert!(cached.is_some());

        let client = redis::Client::open(url).expect("test Redis URL should be valid");
        let mut connection = client
            .get_multiplexed_async_connection()
            .await
            .expect("local Redis should accept connections");
        let ttl: i64 = redis::cmd("TTL")
            .arg(ProductCache::product_key(&product.id))
            .query_async(&mut connection)
            .await
            .expect("TTL should be readable");
        assert!(ttl > 0 && ttl <= PRODUCT_CACHE_TTL_SECONDS as i64);

        cache
            .delete_product_cache(&product.id)
            .await
            .expect("cached product should be evicted");
        assert!(cache
            .get_product(&product.id)
            .await
            .expect("cache should remain available")
            .is_none());
    }

    struct FakeCache {
        hit: Option<Product>,
        puts: AtomicUsize,
        evictions: AtomicUsize,
    }

    impl FakeCache {
        fn with_hit(product: Product) -> Self {
            Self {
                hit: Some(product),
                puts: AtomicUsize::new(0),
                evictions: AtomicUsize::new(0),
            }
        }

        fn miss() -> Self {
            Self {
                hit: None,
                puts: AtomicUsize::new(0),
                evictions: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl ProductCacheBackend for FakeCache {
        async fn get_product(&self, _id: &ProductId) -> Result<Option<Product>, CacheError> {
            Ok(self.hit.clone())
        }

        async fn put_product(&self, _product: &Product) -> Result<(), CacheError> {
            self.puts.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }

        async fn delete_product_cache(&self, _id: &ProductId) -> Result<(), CacheError> {
            self.evictions.fetch_add(1, Ordering::Relaxed);
            Ok(())
        }
    }

    struct ErrorCache;

    #[async_trait]
    impl ProductCacheBackend for ErrorCache {
        async fn get_product(&self, _id: &ProductId) -> Result<Option<Product>, CacheError> {
            Err(CacheError::Timeout("test read"))
        }

        async fn put_product(&self, _product: &Product) -> Result<(), CacheError> {
            Err(CacheError::Timeout("test backfill"))
        }

        async fn delete_product_cache(&self, _id: &ProductId) -> Result<(), CacheError> {
            Err(CacheError::Timeout("test eviction"))
        }
    }

    struct FakeRepository {
        product: Product,
        gets: AtomicUsize,
    }

    impl FakeRepository {
        fn new(product: Product) -> Self {
            Self {
                product,
                gets: AtomicUsize::new(0),
            }
        }
    }

    #[async_trait]
    impl ProductRepository for FakeRepository {
        async fn list_active(&self) -> Result<Vec<Product>, AppError> {
            Ok(vec![self.product.clone()])
        }

        async fn search(&self, _query: ProductQuery) -> Result<ProductPage, AppError> {
            Ok(ProductPage {
                items: vec![self.product.clone()],
                next_cursor: None,
            })
        }

        async fn get(&self, _id: ProductId) -> Result<Option<Product>, AppError> {
            self.gets.fetch_add(1, Ordering::Relaxed);
            Ok(Some(self.product.clone()))
        }

        async fn create(&self, _input: CreateProduct) -> Result<Product, AppError> {
            Ok(self.product.clone())
        }

        async fn update(
            &self,
            _id: ProductId,
            _input: UpdateProduct,
        ) -> Result<Option<Product>, AppError> {
            Ok(Some(self.product.clone()))
        }

        async fn deactivate(&self, _id: ProductId) -> Result<bool, AppError> {
            Ok(true)
        }
    }

    fn product() -> Product {
        serde_json::from_value(serde_json::json!({
            "id": "product-1",
            "sku": "PPS-001",
            "modelNumber": "MODEL-001",
            "brandId": "brand-1",
            "price": { "amount": "99.99", "currency": "USD" },
            "name": "Test Product",
            "attributes": [],
            "description": "Test description",
            "imageName": "test.webp",
            "imageUrls": [],
            "saleType": "retail",
            "categoryIds": ["category-1"],
            "metadata": {
                "createdAt": "2026-01-01T00:00:00Z",
                "updatedAt": "2026-01-01T00:00:00Z",
                "isActive": true,
                "version": 1
            }
        }))
        .expect("test product must deserialize")
    }
}

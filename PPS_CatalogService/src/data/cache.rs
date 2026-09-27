use crate::{
    application::ProductRepository,
    domain::{CreateProduct, Product, ProductId, ProductPage, ProductQuery, UpdateProduct},
    AppError,
};
use async_trait::async_trait;
use bb8::{ManageConnection, Pool};
use redis::{cluster::ClusterClient, cluster_async::ClusterConnection, RedisError};
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

/// A bounded bb8 pool of asynchronous Redis Cluster connections.
#[derive(Clone)]
pub struct ProductCache {
    pool: Pool<RedisClusterManager>,
    operation_timeout: Duration,
}

impl ProductCache {
    pub fn connect(nodes: Vec<String>, max_connections: u32) -> Result<Self, AppError> {
        if nodes.is_empty() {
            return Err(AppError::Configuration(
                "REDIS_CLUSTER_URLS must contain at least one node".into(),
            ));
        }
        let manager = RedisClusterManager::new(nodes).map_err(|error| {
            AppError::Configuration(format!("invalid Redis configuration: {error}"))
        })?;
        let pool = Pool::builder()
            .max_size(max_connections.max(1))
            .connection_timeout(Duration::from_secs(2))
            .build_unchecked(manager);
        Ok(Self {
            pool,
            operation_timeout: DEFAULT_CACHE_OPERATION_TIMEOUT,
        })
    }

    fn product_key(id: &ProductId) -> String {
        format!("product:{{{}}}", id.0)
    }

    async fn get_cached_product(&self, id: &ProductId) -> Option<Product> {
        let key = Self::product_key(id);
        let operation = async {
            let mut connection = self.pool.get().await?;
            let cached: Option<Vec<u8>> = redis::cmd("GET")
                .arg(&key)
                .query_async(&mut *connection)
                .await?;
            Ok::<_, CacheOperationError>(cached)
        };
        let bytes = match timeout(self.operation_timeout, operation).await {
            Ok(Ok(value)) => value,
            Ok(Err(error)) => {
                warn!(cache_key = %key, error = %error, "Redis product read failed; using database");
                return None;
            }
            Err(_) => {
                warn!(cache_key = %key, "Redis product read timed out; using database");
                return None;
            }
        }?;
        match serde_json::from_slice(&bytes) {
            Ok(product) => Some(product),
            Err(error) => {
                warn!(cache_key = %key, error = %error, "invalid cached product; using database");
                ProductCacheBackend::delete_product_cache(self, id).await;
                None
            }
        }
    }

    async fn put_cached_product(&self, product: &Product) {
        let key = Self::product_key(&product.id);
        let bytes = match serde_json::to_vec(product) {
            Ok(value) => value,
            Err(error) => {
                warn!(cache_key = %key, error = %error, "product cache serialization failed");
                return;
            }
        };
        let operation = async {
            let mut connection = self.pool.get().await?;
            redis::cmd("SET")
                .arg(&key)
                .arg(bytes)
                .arg("EX")
                .arg(PRODUCT_CACHE_TTL_SECONDS)
                .query_async::<()>(&mut *connection)
                .await?;
            Ok::<_, CacheOperationError>(())
        };
        match timeout(self.operation_timeout, operation).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                warn!(cache_key = %key, error = %error, "Redis product backfill failed")
            }
            Err(_) => warn!(cache_key = %key, "Redis product backfill timed out"),
        }
    }

    /// Evicts a product after the primary database commit. Eviction failures are
    /// logged and fail open so Redis cannot make successful writes appear failed.
    async fn evict_product(&self, id: &ProductId) {
        let key = Self::product_key(id);
        let operation = async {
            let mut connection = self.pool.get().await?;
            redis::cmd("DEL")
                .arg(&key)
                .query_async::<u64>(&mut *connection)
                .await?;
            Ok::<_, CacheOperationError>(())
        };
        match timeout(self.operation_timeout, operation).await {
            Ok(Ok(())) => {}
            Ok(Err(error)) => {
                warn!(cache_key = %key, error = %error, "Redis product eviction failed")
            }
            Err(_) => warn!(cache_key = %key, "Redis product eviction timed out"),
        }
    }
}

#[async_trait]
pub trait ProductCacheBackend: Send + Sync {
    async fn get_product(&self, id: &ProductId) -> Option<Product>;
    async fn put_product(&self, product: &Product);
    async fn delete_product_cache(&self, id: &ProductId);
}

#[async_trait]
impl ProductCacheBackend for ProductCache {
    async fn get_product(&self, id: &ProductId) -> Option<Product> {
        self.get_cached_product(id).await
    }

    async fn put_product(&self, product: &Product) {
        self.put_cached_product(product).await;
    }

    async fn delete_product_cache(&self, id: &ProductId) {
        self.evict_product(id).await;
    }
}

#[derive(Debug, thiserror::Error)]
enum CacheOperationError {
    #[error("pool error: {0}")]
    Pool(#[from] bb8::RunError<RedisError>),
    #[error("Redis error: {0}")]
    Redis(#[from] RedisError),
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
            if let Some(product) = cache.get_product(&id).await {
                return Ok(Some(product));
            }
        }
        let product = self.primary.get(id).await?;
        if let (Some(cache), Some(product)) = (&self.cache, product.as_ref()) {
            cache.put_product(product).await;
        }
        Ok(product)
    }

    pub async fn delete_product_cache(&self, id: &ProductId) {
        if let Some(cache) = &self.cache {
            cache.delete_product_cache(id).await;
        }
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
            self.delete_product_cache(&id).await;
        }
        Ok(product)
    }

    async fn deactivate(&self, id: ProductId) -> Result<bool, AppError> {
        let deactivated = self.primary.deactivate(id.clone()).await?;
        if deactivated {
            self.delete_product_cache(&id).await;
        }
        Ok(deactivated)
    }
}

#[cfg(test)]
mod tests {
    use super::{CachedProductRepository, ProductCache, ProductCacheBackend};
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
        async fn get_product(&self, _id: &ProductId) -> Option<Product> {
            self.hit.clone()
        }

        async fn put_product(&self, _product: &Product) {
            self.puts.fetch_add(1, Ordering::Relaxed);
        }

        async fn delete_product_cache(&self, _id: &ProductId) {
            self.evictions.fetch_add(1, Ordering::Relaxed);
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

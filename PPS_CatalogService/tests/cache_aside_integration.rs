use async_trait::async_trait;
use pps_catalog_service::{
    application::ProductRepository,
    data::cache::{CacheError, CachedProductRepository, ProductCacheBackend},
    domain::{CreateProduct, Product, ProductId, ProductPage, ProductQuery, UpdateProduct},
    AppError,
};
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
};
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;

#[tokio::test]
async fn sequential_cache_aside_product_lifecycle() -> Result<(), Box<dyn std::error::Error>> {
    let product = test_product("123")?;
    let database = Arc::new(MockProductDatabase::new(product.clone()));
    let cache = Arc::new(MockProductCache::available());
    let repository = CachedProductRepository::new(database.clone(), Some(cache.clone()));
    let id = ProductId("123".into());

    // Step A: the first read misses Redis, fetches the database, and backfills.
    let first = repository.get_product(id.clone()).await;
    assert!(first.is_ok(), "initial database-backed read should succeed");
    assert!(matches!(&first, Ok(Some(product)) if product.id == id));
    assert_eq!(database.read_count(), 1);
    assert_eq!(cache.miss_count(), 1);
    assert_eq!(cache.put_count(), 1);
    assert!(cache.contains(&id).await);

    // Step B: the second read is served directly from Redis.
    let second = repository.get_product(id.clone()).await;
    assert!(second.is_ok(), "cached read should succeed");
    assert!(matches!(&second, Ok(Some(product)) if product.id == id));
    assert_eq!(database.read_count(), 1, "cache hit must skip the database");
    assert_eq!(cache.hit_count(), 1);

    // Step C: the explicit post-update eviction removes the Redis key.
    let eviction = repository.delete_product_cache(&id).await;
    assert!(eviction.is_ok(), "cache eviction should succeed");
    assert_eq!(cache.delete_count(), 1);
    assert!(!cache.contains(&id).await);

    // Step D: the third read misses again and returns to the database.
    let third = repository.get_product(id.clone()).await;
    assert!(third.is_ok(), "post-eviction database read should succeed");
    assert!(matches!(&third, Ok(Some(product)) if product.id == id));
    assert_eq!(database.read_count(), 2);
    assert_eq!(cache.miss_count(), 2);
    assert_eq!(cache.put_count(), 2);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn disconnected_cache_logs_warning_and_returns_database_product(
) -> Result<(), Box<dyn std::error::Error>> {
    let product = test_product("123")?;
    let database = Arc::new(MockProductDatabase::new(product));
    let cache = Arc::new(MockProductCache::disconnected());
    let repository = CachedProductRepository::new(database.clone(), Some(cache));
    let logs = CapturedLogs::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .with_ansi(false)
        .without_time()
        .with_writer(logs.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let id = ProductId("123".into());
    let result = repository.get_product(id.clone()).await;

    assert!(
        result.is_ok(),
        "Redis failure must fail open to the database"
    );
    assert!(
        matches!(&result, Ok(Some(product)) if product.id == id),
        "the database product must be returned"
    );
    assert_eq!(database.read_count(), 1);
    let output = logs.contents();
    assert!(
        output.contains("Redis product read failed; using database"),
        "expected fail-open warning, captured logs: {output}"
    );
    Ok(())
}

struct MockProductDatabase {
    product: Product,
    reads: AtomicUsize,
}

impl MockProductDatabase {
    fn new(product: Product) -> Self {
        Self {
            product,
            reads: AtomicUsize::new(0),
        }
    }

    fn read_count(&self) -> usize {
        self.reads.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl ProductRepository for MockProductDatabase {
    async fn get(&self, id: ProductId) -> Result<Option<Product>, AppError> {
        self.reads.fetch_add(1, Ordering::Relaxed);
        Ok((id == self.product.id).then(|| self.product.clone()))
    }

    async fn list_active(&self) -> Result<Vec<Product>, AppError> {
        Ok(vec![self.product.clone()])
    }

    async fn search(&self, _query: ProductQuery) -> Result<ProductPage, AppError> {
        Ok(ProductPage {
            items: vec![self.product.clone()],
            next_cursor: None,
        })
    }

    async fn create(&self, _input: CreateProduct) -> Result<Product, AppError> {
        Err(AppError::InvalidData("create is outside this test".into()))
    }

    async fn update(
        &self,
        _id: ProductId,
        _input: UpdateProduct,
    ) -> Result<Option<Product>, AppError> {
        Err(AppError::InvalidData("update is outside this test".into()))
    }

    async fn deactivate(&self, _id: ProductId) -> Result<bool, AppError> {
        Err(AppError::InvalidData(
            "deactivate is outside this test".into(),
        ))
    }
}

struct MockProductCache {
    values: RwLock<HashMap<ProductId, Product>>,
    disconnected: bool,
    hits: AtomicUsize,
    misses: AtomicUsize,
    puts: AtomicUsize,
    deletes: AtomicUsize,
}

impl MockProductCache {
    fn available() -> Self {
        Self::new(false)
    }

    fn disconnected() -> Self {
        Self::new(true)
    }

    fn new(disconnected: bool) -> Self {
        Self {
            values: RwLock::new(HashMap::new()),
            disconnected,
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            puts: AtomicUsize::new(0),
            deletes: AtomicUsize::new(0),
        }
    }

    async fn contains(&self, id: &ProductId) -> bool {
        self.values.read().await.contains_key(id)
    }

    fn hit_count(&self) -> usize {
        self.hits.load(Ordering::Relaxed)
    }

    fn miss_count(&self) -> usize {
        self.misses.load(Ordering::Relaxed)
    }

    fn put_count(&self) -> usize {
        self.puts.load(Ordering::Relaxed)
    }

    fn delete_count(&self) -> usize {
        self.deletes.load(Ordering::Relaxed)
    }

    fn ensure_connected(&self, operation: &'static str) -> Result<(), CacheError> {
        if self.disconnected {
            return Err(CacheError::Timeout(operation));
        }
        Ok(())
    }
}

#[async_trait]
impl ProductCacheBackend for MockProductCache {
    async fn get_product(&self, id: &ProductId) -> Result<Option<Product>, CacheError> {
        self.ensure_connected("mock read")?;
        let product = self.values.read().await.get(id).cloned();
        if product.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        Ok(product)
    }

    async fn put_product(&self, product: &Product) -> Result<(), CacheError> {
        self.ensure_connected("mock backfill")?;
        self.values
            .write()
            .await
            .insert(product.id.clone(), product.clone());
        self.puts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn delete_product_cache(&self, id: &ProductId) -> Result<(), CacheError> {
        self.ensure_connected("mock eviction")?;
        self.values.write().await.remove(id);
        self.deletes.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }
}

#[derive(Clone, Default)]
struct CapturedLogs {
    bytes: Arc<Mutex<Vec<u8>>>,
}

impl CapturedLogs {
    fn contents(&self) -> String {
        match self.bytes.lock() {
            Ok(bytes) => String::from_utf8_lossy(&bytes).into_owned(),
            Err(error) => String::from_utf8_lossy(&error.into_inner()).into_owned(),
        }
    }
}

impl<'writer> MakeWriter<'writer> for CapturedLogs {
    type Writer = Self;

    fn make_writer(&'writer self) -> Self::Writer {
        self.clone()
    }
}

impl Write for CapturedLogs {
    fn write(&mut self, buffer: &[u8]) -> io::Result<usize> {
        match self.bytes.lock() {
            Ok(mut bytes) => {
                bytes.extend_from_slice(buffer);
                Ok(buffer.len())
            }
            Err(_) => Err(io::Error::other("captured log mutex is poisoned")),
        }
    }

    fn flush(&mut self) -> io::Result<()> {
        Ok(())
    }
}

fn test_product(id: &str) -> Result<Product, serde_json::Error> {
    serde_json::from_value(serde_json::json!({
        "id": id,
        "sku": "PPS-123",
        "modelNumber": "MODEL-123",
        "brandId": "brand-1",
        "price": { "amount": "199.99", "currency": "USD" },
        "name": "Integration Test Product",
        "attributes": [],
        "description": "Cache-aside integration fixture",
        "imageName": "product.webp",
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
}

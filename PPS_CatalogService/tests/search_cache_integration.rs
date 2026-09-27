use async_trait::async_trait;
use pps_catalog_service::{
    application::ProductRepository,
    data::{
        cache::{CacheError, CachedProductRepository, ProductCacheBackend},
        search::{ElasticsearchProductSearch, SearchableProductRepository},
    },
    domain::{
        CategoryId, CreateProduct, Product, ProductId, ProductPage, ProductQuery, ProductSort,
        SortDirection, UpdateProduct,
    },
    AppError,
};
use serde_json::json;
use std::{
    collections::HashMap,
    io::{self, Write},
    sync::{
        atomic::{AtomicUsize, Ordering},
        Arc, Mutex,
    },
    time::Duration,
};
use tokio::sync::RwLock;
use tracing::Level;
use tracing_subscriber::fmt::MakeWriter;
use wiremock::{
    matchers::{body_json, method, path},
    Mock, MockServer, ResponseTemplate,
};

#[tokio::test]
async fn index_search_and_cache_aside_work_as_one_product_flow(
) -> Result<(), Box<dyn std::error::Error>> {
    let product = test_product()?;
    let elasticsearch = MockServer::start().await;
    let expected_document = search_document(&product);
    Mock::given(method("POST"))
        .and(path("/products/_doc/product-123"))
        .and(body_json(expected_document.clone()))
        .respond_with(ResponseTemplate::new(201).set_body_json(json!({ "result": "created" })))
        .expect(1)
        .mount(&elasticsearch)
        .await;
    Mock::given(method("POST"))
        .and(path("/products/_search"))
        .respond_with(ResponseTemplate::new(200).set_body_json(json!({
            "hits": { "hits": [{ "_source": expected_document }] }
        })))
        .expect(1)
        .mount(&elasticsearch)
        .await;
    let search = ElasticsearchProductSearch::new(&elasticsearch.uri(), Duration::from_millis(500))?;

    // Step A: write the new catalog product to the products index.
    search.index_product(&product).await?;

    // Step B: Elasticsearch resolves a misspelled query to Product A.
    let search_results = search.search_products("shos", "category-shoes", 25).await?;
    assert_eq!(search_results.len(), 1);
    assert_eq!(search_results[0].id, product.id);
    let selected_id = search_results[0].id.clone();

    // Step C: the first click misses Redis and backfills it from the database.
    let database = Arc::new(MockProductDatabase::new(product.clone()));
    let cache = Arc::new(MockRedisPool::connected());
    let products = CachedProductRepository::new(database.clone(), Some(cache.clone()));
    let first_click = products.get_product(selected_id.clone()).await?;
    assert!(matches!(first_click, Some(ref value) if value.id == selected_id));
    assert_eq!(cache.miss_count(), 1);
    assert_eq!(cache.put_count(), 1);
    assert_eq!(database.get_count(), 1);

    // A repeated click is a Redis hit and does not execute database logic.
    let second_click = products.get_product(selected_id.clone()).await?;
    assert!(matches!(second_click, Some(ref value) if value.id == selected_id));
    assert_eq!(cache.hit_count(), 1);
    assert_eq!(database.get_count(), 1);
    Ok(())
}

#[tokio::test(flavor = "current_thread")]
async fn elasticsearch_outage_logs_warning_and_falls_back_to_database_search(
) -> Result<(), Box<dyn std::error::Error>> {
    let product = test_product()?;
    let elasticsearch = MockServer::start().await;
    Mock::given(method("POST"))
        .and(path("/products/_search"))
        .respond_with(ResponseTemplate::new(503).set_body_string("cluster unavailable"))
        .expect(1)
        .mount(&elasticsearch)
        .await;
    let database = Arc::new(MockProductDatabase::new(product.clone()));
    let search = Arc::new(ElasticsearchProductSearch::new(
        &elasticsearch.uri(),
        Duration::from_millis(500),
    )?);
    let products = SearchableProductRepository::new(database.clone(), Some(search));
    let logs = CapturedLogs::default();
    let subscriber = tracing_subscriber::fmt()
        .with_max_level(Level::WARN)
        .with_ansi(false)
        .without_time()
        .with_writer(logs.clone())
        .finish();
    let _guard = tracing::subscriber::set_default(subscriber);

    let results = products.search(text_query()).await?;

    assert_eq!(results.items.len(), 1);
    assert_eq!(results.items[0].id, product.id);
    assert_eq!(database.search_count(), 1);
    let output = logs.contents();
    assert!(
        output.contains("Elasticsearch product search failed; using primary database"),
        "expected search fail-open warning, captured logs: {output}"
    );
    Ok(())
}

struct MockProductDatabase {
    product: Product,
    gets: AtomicUsize,
    searches: AtomicUsize,
}

impl MockProductDatabase {
    fn new(product: Product) -> Self {
        Self {
            product,
            gets: AtomicUsize::new(0),
            searches: AtomicUsize::new(0),
        }
    }

    fn get_count(&self) -> usize {
        self.gets.load(Ordering::Relaxed)
    }

    fn search_count(&self) -> usize {
        self.searches.load(Ordering::Relaxed)
    }
}

#[async_trait]
impl ProductRepository for MockProductDatabase {
    async fn list_active(&self) -> Result<Vec<Product>, AppError> {
        Ok(vec![self.product.clone()])
    }

    async fn search(&self, _query: ProductQuery) -> Result<ProductPage, AppError> {
        self.searches.fetch_add(1, Ordering::Relaxed);
        Ok(ProductPage {
            items: vec![self.product.clone()],
            next_cursor: None,
        })
    }

    async fn get(&self, id: ProductId) -> Result<Option<Product>, AppError> {
        self.gets.fetch_add(1, Ordering::Relaxed);
        Ok((id == self.product.id).then(|| self.product.clone()))
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

struct MockRedisPool {
    products: RwLock<HashMap<ProductId, Product>>,
    hits: AtomicUsize,
    misses: AtomicUsize,
    puts: AtomicUsize,
}

impl MockRedisPool {
    fn connected() -> Self {
        Self {
            products: RwLock::new(HashMap::new()),
            hits: AtomicUsize::new(0),
            misses: AtomicUsize::new(0),
            puts: AtomicUsize::new(0),
        }
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
}

#[async_trait]
impl ProductCacheBackend for MockRedisPool {
    async fn get_product(&self, id: &ProductId) -> Result<Option<Product>, CacheError> {
        let product = self.products.read().await.get(id).cloned();
        if product.is_some() {
            self.hits.fetch_add(1, Ordering::Relaxed);
        } else {
            self.misses.fetch_add(1, Ordering::Relaxed);
        }
        Ok(product)
    }

    async fn put_product(&self, product: &Product) -> Result<(), CacheError> {
        self.products
            .write()
            .await
            .insert(product.id.clone(), product.clone());
        self.puts.fetch_add(1, Ordering::Relaxed);
        Ok(())
    }

    async fn delete_product_cache(&self, id: &ProductId) -> Result<(), CacheError> {
        self.products.write().await.remove(id);
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

fn text_query() -> ProductQuery {
    ProductQuery {
        page_size: 25,
        cursor: None,
        brand_id: None,
        category_id: Some(CategoryId("category-shoes".into())),
        sale_type: None,
        minimum_price: None,
        maximum_price: None,
        search: Some("shos".into()),
        sort: ProductSort::Newest,
        direction: SortDirection::Descending,
    }
}

fn search_document(product: &Product) -> serde_json::Value {
    json!({
        "id": product.id.0,
        "name": product.name,
        "price": product.price,
        "description": product.description,
        "category": product.category_ids.iter().map(|id| id.0.clone()).collect::<Vec<_>>(),
        "active": product.metadata.is_active,
        "product": product
    })
}

fn test_product() -> Result<Product, serde_json::Error> {
    serde_json::from_value(json!({
        "id": "product-123",
        "sku": "PPS-SHOE-123",
        "modelNumber": "SHOE-123",
        "brandId": "brand-1",
        "price": { "amount": "129.99", "currency": "USD" },
        "name": "Performance Shoes",
        "attributes": [],
        "description": "Lightweight driving shoes",
        "imageName": "shoes.webp",
        "imageUrls": [],
        "saleType": "retail",
        "categoryIds": ["category-shoes"],
        "metadata": {
            "createdAt": "2026-01-01T00:00:00Z",
            "updatedAt": "2026-01-01T00:00:00Z",
            "isActive": true,
            "version": 1
        }
    }))
}

use crate::{
    application::ProductRepository,
    domain::{CreateProduct, Product, ProductId, ProductPage, ProductQuery, UpdateProduct},
    AppError,
};
use async_trait::async_trait;
use elasticsearch::{
    http::transport::{SingleNodeConnectionPool, TransportBuilder},
    DeleteParts, Elasticsearch, IndexParts, SearchParts,
};
use serde::{Deserialize, Serialize};
use serde_json::{json, Value};
use std::{sync::Arc, time::Duration};
use tracing::warn;
use url::Url;

const PRODUCT_INDEX: &str = "products";

#[derive(Debug, thiserror::Error)]
pub enum SearchError {
    #[error("invalid Elasticsearch URL: {0}")]
    InvalidUrl(#[from] url::ParseError),
    #[error("Elasticsearch transport configuration failed: {0}")]
    TransportBuild(#[from] elasticsearch::http::transport::BuildError),
    #[error("Elasticsearch transport error: {0}")]
    Transport(#[from] elasticsearch::Error),
    #[error("Elasticsearch response serialization failed: {0}")]
    Serialization(#[from] serde_json::Error),
    #[error("Elasticsearch returned HTTP {status}: {body}")]
    Http { status: u16, body: String },
}

#[derive(Clone)]
pub struct ElasticsearchProductSearch {
    client: Elasticsearch,
}

#[derive(Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
struct ProductSearchDocument {
    id: String,
    name: String,
    price: crate::domain::Money,
    description: String,
    category: Vec<String>,
    active: bool,
    product: Product,
}

#[derive(Deserialize)]
struct ElasticsearchSearchResponse {
    hits: ElasticsearchHits,
}

#[derive(Deserialize)]
struct ElasticsearchHits {
    hits: Vec<ElasticsearchHit>,
}

#[derive(Deserialize)]
struct ElasticsearchHit {
    #[serde(rename = "_source")]
    source: ProductSearchDocument,
}

impl ElasticsearchProductSearch {
    pub fn new(endpoint: &str, request_timeout: Duration) -> Result<Self, SearchError> {
        let endpoint = Url::parse(endpoint)?;
        let pool = SingleNodeConnectionPool::new(endpoint);
        let transport = TransportBuilder::new(pool)
            .timeout(request_timeout)
            .build()?;
        Ok(Self {
            client: Elasticsearch::new(transport),
        })
    }

    /// Searches name and description with typo tolerance and an optional exact
    /// category filter. An empty category string means all categories.
    pub async fn search_products(
        &self,
        raw_query: &str,
        category_filter: &str,
        limit: u32,
    ) -> Result<Vec<Product>, SearchError> {
        let mut filters = vec![json!({ "term": { "active": true } })];
        if !category_filter.trim().is_empty() {
            filters.push(json!({ "term": { "category": category_filter } }));
        }
        let body = json!({
            "size": limit.clamp(1, 100),
            "query": {
                "bool": {
                    "must": [{
                        "multi_match": {
                            "query": raw_query.trim(),
                            "fields": ["name^3", "description"],
                            "fuzziness": "AUTO"
                        }
                    }],
                    "filter": filters
                }
            }
        });
        let response = self
            .client
            .search(SearchParts::Index(&[PRODUCT_INDEX]))
            .body(body)
            .send()
            .await?;
        let status = response.status_code();
        let bytes = response.bytes().await?;
        if !status.is_success() {
            return Err(SearchError::Http {
                status: status.as_u16(),
                body: String::from_utf8_lossy(&bytes).into_owned(),
            });
        }
        let results: ElasticsearchSearchResponse = serde_json::from_slice(&bytes)?;
        Ok(results
            .hits
            .hits
            .into_iter()
            .map(|hit| hit.source.product)
            .collect())
    }

    pub async fn index_product(&self, product: &Product) -> Result<(), SearchError> {
        let document = ProductSearchDocument {
            id: product.id.0.clone(),
            name: product.name.clone(),
            price: product.price.clone(),
            description: product.description.clone().unwrap_or_default(),
            category: product
                .category_ids
                .iter()
                .map(|category| category.0.clone())
                .collect(),
            active: product.metadata.is_active,
            product: product.clone(),
        };
        let payload: Value = serde_json::to_value(document)?;
        let response = self
            .client
            .index(IndexParts::IndexId(PRODUCT_INDEX, &product.id.0))
            .body(payload)
            .send()
            .await?;
        ensure_success(response).await
    }

    async fn delete_product(&self, id: &ProductId) -> Result<(), SearchError> {
        let response = self
            .client
            .delete(DeleteParts::IndexId(PRODUCT_INDEX, &id.0))
            .send()
            .await?;
        if response.status_code().as_u16() == 404 {
            return Ok(());
        }
        ensure_success(response).await
    }
}

async fn ensure_success(
    response: elasticsearch::http::response::Response,
) -> Result<(), SearchError> {
    let status = response.status_code();
    if status.is_success() {
        return Ok(());
    }
    let bytes = response.bytes().await?;
    Err(SearchError::Http {
        status: status.as_u16(),
        body: String::from_utf8_lossy(&bytes).into_owned(),
    })
}

/// Routes compatible text searches to Elasticsearch and keeps the primary
/// database authoritative for writes and unsupported filter combinations.
pub struct SearchableProductRepository {
    primary: Arc<dyn ProductRepository>,
    search: Option<Arc<ElasticsearchProductSearch>>,
}

impl SearchableProductRepository {
    pub fn new(
        primary: Arc<dyn ProductRepository>,
        search: Option<Arc<ElasticsearchProductSearch>>,
    ) -> Self {
        Self { primary, search }
    }

    fn supports_search(query: &ProductQuery) -> bool {
        query.search.is_some()
            && query.cursor.is_none()
            && query.brand_id.is_none()
            && query.sale_type.is_none()
            && query.minimum_price.is_none()
            && query.maximum_price.is_none()
    }
}

#[async_trait]
impl ProductRepository for SearchableProductRepository {
    async fn list_active(&self) -> Result<Vec<Product>, AppError> {
        self.primary.list_active().await
    }

    async fn search(&self, query: ProductQuery) -> Result<ProductPage, AppError> {
        if Self::supports_search(&query) {
            if let (Some(search), Some(raw_query)) = (&self.search, query.search.as_deref()) {
                let category = query
                    .category_id
                    .as_ref()
                    .map_or("", |category| category.0.as_str());
                match search
                    .search_products(raw_query, category, query.page_size)
                    .await
                {
                    Ok(items) => {
                        return Ok(ProductPage {
                            items,
                            next_cursor: None,
                        })
                    }
                    Err(error) => {
                        warn!(error = %error, "Elasticsearch product search failed; using primary database")
                    }
                }
            }
        }
        self.primary.search(query).await
    }

    async fn get(&self, id: ProductId) -> Result<Option<Product>, AppError> {
        self.primary.get(id).await
    }

    async fn create(&self, input: CreateProduct) -> Result<Product, AppError> {
        let product = self.primary.create(input).await?;
        if let Some(search) = &self.search {
            if let Err(error) = search.index_product(&product).await {
                warn!(product_id = %product.id.0, error = %error, "Elasticsearch product indexing failed after create")
            }
        }
        Ok(product)
    }

    async fn update(
        &self,
        id: ProductId,
        input: UpdateProduct,
    ) -> Result<Option<Product>, AppError> {
        let product = self.primary.update(id, input).await?;
        if let (Some(search), Some(product)) = (&self.search, product.as_ref()) {
            if let Err(error) = search.index_product(product).await {
                warn!(product_id = %product.id.0, error = %error, "Elasticsearch product indexing failed after update")
            }
        }
        Ok(product)
    }

    async fn deactivate(&self, id: ProductId) -> Result<bool, AppError> {
        let deactivated = self.primary.deactivate(id.clone()).await?;
        if deactivated {
            if let Some(search) = &self.search {
                if let Err(error) = search.delete_product(&id).await {
                    warn!(product_id = %id.0, error = %error, "Elasticsearch product removal failed after deactivation")
                }
            }
        }
        Ok(deactivated)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::domain::{ProductSort, SortDirection};
    use wiremock::{
        matchers::{body_json, method, path},
        Mock, MockServer, ResponseTemplate,
    };

    #[tokio::test]
    async fn search_builds_fuzzy_multi_match_with_category_filter(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        let product = test_product()?;
        let source = ProductSearchDocument {
            id: product.id.0.clone(),
            name: product.name.clone(),
            price: product.price.clone(),
            description: product.description.clone().unwrap_or_default(),
            category: vec!["category-1".into()],
            active: true,
            product: product.clone(),
        };
        Mock::given(method("POST"))
            .and(path("/products/_search"))
            .and(body_json(json!({
                "size": 25,
                "query": {
                    "bool": {
                        "must": [{
                            "multi_match": {
                                "query": "brke rotor",
                                "fields": ["name^3", "description"],
                                "fuzziness": "AUTO"
                            }
                        }],
                        "filter": [
                            { "term": { "active": true } },
                            { "term": { "category": "category-1" } }
                        ]
                    }
                }
            })))
            .respond_with(ResponseTemplate::new(200).set_body_json(json!({
                "hits": { "hits": [{ "_source": source }] }
            })))
            .expect(1)
            .mount(&server)
            .await;
        let search = ElasticsearchProductSearch::new(&server.uri(), Duration::from_millis(500))?;

        let results = search
            .search_products(" brke rotor ", "category-1", 25)
            .await?;

        assert_eq!(results.len(), 1);
        assert_eq!(results[0].id, product.id);
        Ok(())
    }

    #[tokio::test]
    async fn index_product_upserts_serialized_payload() -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        let product = test_product()?;
        Mock::given(path("/products/_doc/product-1"))
            .respond_with(ResponseTemplate::new(201).set_body_json(json!({
                "result": "created"
            })))
            .expect(1)
            .mount(&server)
            .await;
        let search = ElasticsearchProductSearch::new(&server.uri(), Duration::from_millis(500))?;

        let result = search.index_product(&product).await;

        assert!(result.is_ok(), "index request failed: {result:?}");
        Ok(())
    }

    #[tokio::test]
    async fn unavailable_elasticsearch_falls_back_to_primary_search(
    ) -> Result<(), Box<dyn std::error::Error>> {
        let server = MockServer::start().await;
        Mock::given(method("POST"))
            .and(path("/products/_search"))
            .respond_with(ResponseTemplate::new(503).set_body_string("unavailable"))
            .expect(1)
            .mount(&server)
            .await;
        let product = test_product()?;
        let primary = Arc::new(FallbackRepository {
            product: product.clone(),
        });
        let search = Arc::new(ElasticsearchProductSearch::new(
            &server.uri(),
            Duration::from_millis(500),
        )?);
        let repository = SearchableProductRepository::new(primary, Some(search));

        let page = repository.search(text_query()).await?;

        assert_eq!(page.items.len(), 1);
        assert_eq!(page.items[0].id, product.id);
        Ok(())
    }

    struct FallbackRepository {
        product: Product,
    }

    #[async_trait]
    impl ProductRepository for FallbackRepository {
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

    fn text_query() -> ProductQuery {
        ProductQuery {
            page_size: 25,
            cursor: None,
            brand_id: None,
            category_id: Some(crate::domain::CategoryId("category-1".into())),
            sale_type: None,
            minimum_price: None,
            maximum_price: None,
            search: Some("brke rotor".into()),
            sort: ProductSort::Newest,
            direction: SortDirection::Descending,
        }
    }

    fn test_product() -> Result<Product, serde_json::Error> {
        serde_json::from_value(json!({
            "id": "product-1",
            "sku": "PPS-001",
            "modelNumber": "MODEL-001",
            "brandId": "brand-1",
            "price": { "amount": "199.99", "currency": "USD" },
            "name": "Brake Rotor",
            "attributes": [],
            "description": "Slotted performance brake rotor",
            "imageName": "rotor.webp",
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
}

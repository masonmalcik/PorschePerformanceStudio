use crate::{
    application::{
        BrandApplication, CategoryApplication, EngineApplication, GenerationApplication,
        MetricsSink, Permission, ProductApplication, ProductFitmentApplication, RequestAuthorizer,
        RequestIdentity, RequestMetric, TrimApplication, VehicleConfigurationApplication,
        VehicleModelApplication,
    },
    domain::{
        CreateBrand, CreateCategory, CreateEngine, CreateGeneration, CreateProduct,
        CreateProductFitment, CreateTrim, CreateVehicleConfiguration, CreateVehicleModel,
        ProductId, UpdateProduct,
    },
    presentation::{
        openapi,
        product_query::parse_product_query,
        responses::{
            BrandCreatedResponse, CategoryCreatedResponse, EngineCreatedResponse,
            GenerationCreatedResponse, GenerationOptionResponse, NamedOptionResponse,
            ProductFitmentCreatedResponse, PublicProductPageResponse, PublicProductResponse,
            TrimCreatedResponse, TrimOptionResponse, VehicleConfigurationCreatedResponse,
            VehicleModelCreatedResponse, VehicleModelOptionResponse,
        },
    },
    AppError,
};
use lambda_http::{http::Method, Body, Request, Response};
use serde::Serialize;
use serde_json::json;
use sha2::{Digest, Sha256};
use std::{sync::Arc, time::Instant};

const MAX_BODY_BYTES: usize = 64 * 1024;

#[derive(Clone)]
pub struct AppState {
    products: Arc<dyn ProductApplication>,
    vehicle_models: Arc<dyn VehicleModelApplication>,
    generations: Arc<dyn GenerationApplication>,
    brands: Arc<dyn BrandApplication>,
    categories: Arc<dyn CategoryApplication>,
    trims: Arc<dyn TrimApplication>,
    engines: Arc<dyn EngineApplication>,
    configurations: Arc<dyn VehicleConfigurationApplication>,
    fitments: Arc<dyn ProductFitmentApplication>,
    authorizer: Arc<dyn RequestAuthorizer>,
    metrics: Arc<dyn MetricsSink>,
}

pub struct AdminApplications {
    pub generations: Arc<dyn GenerationApplication>,
    pub brands: Arc<dyn BrandApplication>,
    pub categories: Arc<dyn CategoryApplication>,
    pub trims: Arc<dyn TrimApplication>,
    pub engines: Arc<dyn EngineApplication>,
    pub configurations: Arc<dyn VehicleConfigurationApplication>,
    pub fitments: Arc<dyn ProductFitmentApplication>,
}

impl AppState {
    pub fn new(
        products: Arc<dyn ProductApplication>,
        vehicle_models: Arc<dyn VehicleModelApplication>,
        admin: AdminApplications,
        authorizer: Arc<dyn RequestAuthorizer>,
        metrics: Arc<dyn MetricsSink>,
    ) -> Self {
        Self {
            products,
            vehicle_models,
            generations: admin.generations,
            brands: admin.brands,
            categories: admin.categories,
            trims: admin.trims,
            engines: admin.engines,
            configurations: admin.configurations,
            fitments: admin.fitments,
            authorizer,
            metrics,
        }
    }
}

pub async fn handle_request(request: Request, state: AppState) -> Response<Body> {
    let started = Instant::now();
    let method = request.method().to_string();
    let path = normalized_path(request.uri().path()).to_owned();
    let request_id = request
        .headers()
        .get("x-request-id")
        .or_else(|| request.headers().get("x-amzn-trace-id"))
        .and_then(|value| value.to_str().ok())
        .unwrap_or("unavailable")
        .to_owned();
    let allowed_origin = request
        .headers()
        .get("origin")
        .and_then(|value| value.to_str().ok())
        .filter(|origin| is_allowed_origin(origin))
        .map(str::to_owned);

    let mut response = match route(request, state.clone()).await {
        Ok(response) => response,
        Err(error) => error_response(error),
    };
    if let Some(origin) = allowed_origin {
        let headers = response.headers_mut();
        if let Ok(value) = origin.parse() {
            headers.insert("access-control-allow-origin", value);
        }
        headers.insert("vary", "Origin".parse().expect("valid Vary header"));
        headers.insert(
            "access-control-allow-methods",
            "GET,POST,PATCH,DELETE,OPTIONS"
                .parse()
                .expect("valid CORS methods"),
        );
        headers.insert(
            "access-control-allow-headers",
            "Authorization,Content-Type,If-None-Match,X-Request-ID"
                .parse()
                .expect("valid CORS headers"),
        );
    }

    let duration_ms = started.elapsed().as_millis();
    let status = response.status().as_u16();
    state.metrics.record_request(RequestMetric {
        method: &method,
        path: &path,
        status,
        duration_ms,
    });
    tracing::info!(
        event = "http_request",
        request_id = %request_id,
        method = %method,
        path = %path,
        status,
        duration_ms = duration_ms as u64
    );

    response
}

async fn route(request: Request, state: AppState) -> Result<Response<Body>, AppError> {
    let method = request.method().clone();
    let path = normalized_path(request.uri().path());

    match (method, path) {
        (Method::OPTIONS, _) => Response::builder()
            .status(204)
            .header("cache-control", "public, max-age=3600")
            .body(Body::Empty)
            .map_err(|error| AppError::InvalidData(error.to_string())),
        (Method::GET, "") | (Method::GET, "/health") => {
            json_response(200, &json!({ "status": "healthy" }))
        }
        (Method::GET, "/assets/products/product-placeholder.png") => image_response(
            "image/png",
            include_bytes!("../../assets/products/product-placeholder.png"),
        ),
        (Method::GET, "/assets/brands/pps-performance.png") => image_response(
            "image/png",
            include_bytes!("../../assets/brands/pps-performance.png"),
        ),
        (Method::GET, "/assets/Page-Backgrounds/catalog-workshop.png") => image_response(
            "image/png",
            include_bytes!("../../assets/Page-Backgrounds/catalog-workshop.png"),
        ),
        (Method::GET, "/api-docs/openapi.json") => json_response(200, &openapi::document()),
        (Method::GET, "/products") => {
            let query = parse_product_query(request.uri().query())?;
            let page = state.products.search(query).await?;
            cached_json_response(&request, 200, &PublicProductPageResponse::from(page))
        }
        (Method::GET, path) if path.starts_with("/products/") => {
            let id = parse_id(path, "/products/")?;
            let product = state.products.get(id).await?;
            cached_json_response(&request, 200, &PublicProductResponse::from(product))
        }
        (Method::POST, "/admin/products") => {
            authorize_admin(&request, &state).await?;
            let command: CreateProduct = parse_json_body(&request)?;
            let product = state.products.create(command).await?;
            state.metrics.increment("catalog_product_created");
            json_response(201, &PublicProductResponse::from(product))
        }
        (Method::GET, "/admin/products") => {
            authorize_admin(&request, &state).await?;
            let values: Vec<NamedOptionResponse> = state
                .products
                .list_active()
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
            json_response(200, &values)
        }
        (Method::PATCH, path) if path.starts_with("/admin/products/") => {
            authorize_admin(&request, &state).await?;
            let id = parse_id(path, "/admin/products/")?;
            let command: UpdateProduct = parse_json_body(&request)?;
            let product = state.products.update(id, command).await?;
            state.metrics.increment("catalog_product_updated");
            json_response(200, &PublicProductResponse::from(product))
        }
        (Method::DELETE, path) if path.starts_with("/admin/products/") => {
            authorize_admin(&request, &state).await?;
            let id = parse_id(path, "/admin/products/")?;
            state.products.deactivate(id).await?;
            state.metrics.increment("catalog_product_deactivated");
            Response::builder()
                .status(204)
                .header("cache-control", "no-store")
                .body(Body::Empty)
                .map_err(|error| AppError::InvalidData(error.to_string()))
        }
        (Method::POST, "/admin/vehicle-models") => {
            authorize_admin(&request, &state).await?;
            let command: CreateVehicleModel = parse_json_body(&request)?;
            let model = state.vehicle_models.create(command).await?;
            state.metrics.increment("catalog_vehicle_model_created");
            json_response(201, &VehicleModelCreatedResponse::from(model))
        }
        (Method::GET, "/admin/vehicle-models") => {
            authorize_admin(&request, &state).await?;
            let models: Vec<VehicleModelOptionResponse> = state
                .vehicle_models
                .list_active()
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
            json_response(200, &models)
        }
        (Method::POST, "/admin/vehicle-generations") => {
            authorize_admin(&request, &state).await?;
            let command: CreateGeneration = parse_json_body(&request)?;
            let generation = state.generations.create(command).await?;
            state
                .metrics
                .increment("catalog_vehicle_generation_created");
            json_response(201, &GenerationCreatedResponse::from(generation))
        }
        (Method::GET, "/admin/vehicle-generations") => {
            authorize_admin(&request, &state).await?;
            let values: Vec<GenerationOptionResponse> = state
                .generations
                .list_active()
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
            json_response(200, &values)
        }
        (Method::GET, "/admin/brands") => {
            authorize_admin(&request, &state).await?;
            let values: Vec<NamedOptionResponse> = state
                .brands
                .list_active()
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
            json_response(200, &values)
        }
        (Method::POST, "/admin/brands") => {
            authorize_admin(&request, &state).await?;
            let value = state
                .brands
                .create(parse_json_body::<CreateBrand>(&request)?)
                .await?;
            state.metrics.increment("catalog_brand_created");
            json_response(201, &BrandCreatedResponse::from(value))
        }
        (Method::GET, "/admin/categories") => {
            authorize_admin(&request, &state).await?;
            let values: Vec<NamedOptionResponse> = state
                .categories
                .list_active()
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
            json_response(200, &values)
        }
        (Method::POST, "/admin/categories") => {
            authorize_admin(&request, &state).await?;
            let command: CreateCategory = parse_json_body(&request)?;
            let category = state.categories.create(command).await?;
            state.metrics.increment("catalog_category_created");
            json_response(201, &CategoryCreatedResponse::from(category))
        }
        (Method::POST, "/admin/vehicle-trims") => {
            authorize_admin(&request, &state).await?;
            let command: CreateTrim = parse_json_body(&request)?;
            let trim = state.trims.create(command).await?;
            state.metrics.increment("catalog_vehicle_trim_created");
            json_response(201, &TrimCreatedResponse::from(trim))
        }
        (Method::GET, "/admin/vehicle-trims") => {
            authorize_admin(&request, &state).await?;
            let values: Vec<TrimOptionResponse> = state
                .trims
                .list_active()
                .await?
                .into_iter()
                .map(Into::into)
                .collect();
            json_response(200, &values)
        }
        (Method::POST, "/admin/vehicle-configurations") => {
            authorize_admin(&request, &state).await?;
            let value = state
                .configurations
                .create(parse_json_body::<CreateVehicleConfiguration>(&request)?)
                .await?;
            state
                .metrics
                .increment("catalog_vehicle_configuration_created");
            json_response(201, &VehicleConfigurationCreatedResponse::from(value))
        }
        (Method::POST, "/admin/engines") => {
            authorize_admin(&request, &state).await?;
            let value = state
                .engines
                .create(parse_json_body::<CreateEngine>(&request)?)
                .await?;
            state.metrics.increment("catalog_engine_created");
            json_response(201, &EngineCreatedResponse::from(value))
        }
        (Method::POST, "/admin/product-fitments") => {
            authorize_admin(&request, &state).await?;
            let value = state
                .fitments
                .create(parse_json_body::<CreateProductFitment>(&request)?)
                .await?;
            state.metrics.increment("catalog_product_fitment_created");
            json_response(201, &ProductFitmentCreatedResponse::from(value))
        }
        _ => Err(AppError::NotFound),
    }
}

async fn authorize_admin(request: &Request, state: &AppState) -> Result<(), AppError> {
    let identity = RequestIdentity {
        development_token: header(request, "x-pps-admin-key"),
        bearer_token: header(request, "authorization")
            .and_then(|value| value.strip_prefix("Bearer ").map(str::to_owned)),
    };
    let principal = state
        .authorizer
        .authorize(identity, Permission::CatalogAdmin)
        .await?;
    tracing::info!(event = "admin_authorized", subject = %principal.subject);
    Ok(())
}

fn header(request: &Request, name: &str) -> Option<String> {
    request
        .headers()
        .get(name)
        .and_then(|value| value.to_str().ok())
        .map(str::to_owned)
}

fn parse_id(path: &str, prefix: &str) -> Result<ProductId, AppError> {
    let value = path
        .strip_prefix(prefix)
        .filter(|value| !value.is_empty() && !value.contains('/'))
        .ok_or_else(|| AppError::BadRequest("invalid resource path".to_owned()))?;
    if value.len() != 24 || !value.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return Err(AppError::BadRequest(
            "id must be a 24-character hexadecimal value".to_owned(),
        ));
    }
    Ok(ProductId(value.to_owned()))
}

fn parse_json_body<T>(request: &Request) -> Result<T, AppError>
where
    T: serde::de::DeserializeOwned,
{
    let bytes: &[u8] = match request.body() {
        Body::Text(value) => value.as_bytes(),
        Body::Binary(value) => value.as_slice(),
        Body::Empty => return Err(AppError::BadRequest("request body is required".to_owned())),
        _ => {
            return Err(AppError::BadRequest(
                "unsupported request body encoding".to_owned(),
            ))
        }
    };
    if bytes.len() > MAX_BODY_BYTES {
        return Err(AppError::BadRequest(format!(
            "request body exceeds {MAX_BODY_BYTES} bytes"
        )));
    }
    serde_json::from_slice(bytes)
        .map_err(|error| AppError::BadRequest(format!("invalid JSON request: {error}")))
}

fn cached_json_response<T: Serialize>(
    request: &Request,
    status: u16,
    value: &T,
) -> Result<Response<Body>, AppError> {
    let body = serde_json::to_vec(value).map_err(AppError::Serialization)?;
    let etag = format!("\"{:x}\"", Sha256::digest(&body));
    let cache_control = "public, max-age=60, s-maxage=300";

    if request
        .headers()
        .get("if-none-match")
        .and_then(|value| value.to_str().ok())
        == Some(etag.as_str())
    {
        return Response::builder()
            .status(304)
            .header("etag", etag)
            .header("cache-control", cache_control)
            .body(Body::Empty)
            .map_err(|error| AppError::InvalidData(error.to_string()));
    }

    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("cache-control", cache_control)
        .header("etag", etag)
        .body(Body::Binary(body))
        .map_err(|error| AppError::InvalidData(error.to_string()))
}

fn json_response<T: Serialize>(status: u16, value: &T) -> Result<Response<Body>, AppError> {
    let body = serde_json::to_string(value).map_err(AppError::Serialization)?;
    Response::builder()
        .status(status)
        .header("content-type", "application/json")
        .header("cache-control", "no-store")
        .body(Body::Text(body))
        .map_err(|error| AppError::InvalidData(error.to_string()))
}

fn image_response(
    content_type: &'static str,
    bytes: &'static [u8],
) -> Result<Response<Body>, AppError> {
    Response::builder()
        .status(200)
        .header("content-type", content_type)
        .header("cache-control", "public, max-age=86400, s-maxage=604800")
        .body(Body::Binary(bytes.to_vec()))
        .map_err(|error| AppError::InvalidData(error.to_string()))
}

fn error_response(error: AppError) -> Response<Body> {
    tracing::warn!(event = "request_failed", error = %error);
    error.into_response()
}

fn normalized_path(path: &str) -> &str {
    if let Some(rest) = path
        .strip_prefix("/Prod")
        .filter(|rest| rest.starts_with('/'))
    {
        rest
    } else if let Some(rest) = path
        .strip_prefix("/prod")
        .filter(|rest| rest.starts_with('/'))
    {
        rest
    } else {
        path
    }
}

fn is_allowed_origin(origin: &str) -> bool {
    std::env::var("CORS_ALLOWED_ORIGINS")
        .unwrap_or_else(|_| "http://localhost:3000,http://localhost:5173".to_owned())
        .split(',')
        .map(str::trim)
        .any(|allowed| allowed == origin)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{
        application::{
            BrandApplication, CategoryApplication, GenerationApplication, Principal,
            ProductApplication, ProductFitmentApplication, RequestAuthorizer, TrimApplication,
            VehicleConfigurationApplication, VehicleModelApplication,
        },
        domain::{
            Brand, Category, Generation, Product, ProductPage, ProductQuery, Trim, VehicleModel,
        },
    };
    use async_trait::async_trait;

    struct Products;

    #[async_trait]
    impl ProductApplication for Products {
        async fn list_active(&self) -> Result<Vec<Product>, AppError> {
            Ok(vec![])
        }
        async fn search(&self, _query: ProductQuery) -> Result<ProductPage, AppError> {
            Ok(ProductPage {
                items: vec![],
                next_cursor: None,
            })
        }
        async fn get(&self, _id: ProductId) -> Result<Product, AppError> {
            Err(AppError::NotFound)
        }
        async fn create(&self, _command: CreateProduct) -> Result<Product, AppError> {
            Err(AppError::InvalidData("not used".to_owned()))
        }
        async fn update(
            &self,
            _id: ProductId,
            _command: UpdateProduct,
        ) -> Result<Product, AppError> {
            Err(AppError::InvalidData("not used".to_owned()))
        }
        async fn deactivate(&self, _id: ProductId) -> Result<(), AppError> {
            Ok(())
        }
    }

    struct VehicleModels;

    #[async_trait]
    impl VehicleModelApplication for VehicleModels {
        async fn create(&self, command: CreateVehicleModel) -> Result<VehicleModel, AppError> {
            Ok(VehicleModel {
                id: crate::domain::VehicleModelId(mongodb::bson::oid::ObjectId::new().to_hex()),
                name: command.name,
                model_code: command.model_code,
                metadata: crate::domain::EntityMetadata::new(),
            })
        }
        async fn list_active(&self) -> Result<Vec<VehicleModel>, AppError> {
            Ok(vec![])
        }
    }

    struct Allow;

    struct AdminCatalog;
    #[async_trait]
    impl BrandApplication for AdminCatalog {
        async fn create(&self, _: CreateBrand) -> Result<Brand, AppError> {
            Err(AppError::InvalidData("not used".into()))
        }
        async fn list_active(&self) -> Result<Vec<Brand>, AppError> {
            Ok(vec![])
        }
    }
    #[async_trait]
    impl CategoryApplication for AdminCatalog {
        async fn create(&self, _: CreateCategory) -> Result<Category, AppError> {
            Err(AppError::InvalidData("not used".into()))
        }
        async fn list_active(&self) -> Result<Vec<Category>, AppError> {
            Ok(vec![])
        }
    }
    #[async_trait]
    impl TrimApplication for AdminCatalog {
        async fn create(&self, _: CreateTrim) -> Result<Trim, AppError> {
            Err(AppError::InvalidData("not used".into()))
        }
        async fn list_active(&self) -> Result<Vec<Trim>, AppError> {
            Ok(vec![])
        }
    }
    #[async_trait]
    impl VehicleConfigurationApplication for AdminCatalog {
        async fn create(
            &self,
            _: CreateVehicleConfiguration,
        ) -> Result<crate::domain::VehicleConfiguration, AppError> {
            Err(AppError::InvalidData("not used".into()))
        }
    }
    #[async_trait]
    impl EngineApplication for AdminCatalog {
        async fn create(&self, _: CreateEngine) -> Result<crate::domain::Engine, AppError> {
            Err(AppError::InvalidData("not used".into()))
        }
    }
    #[async_trait]
    impl ProductFitmentApplication for AdminCatalog {
        async fn create(
            &self,
            _: CreateProductFitment,
        ) -> Result<crate::domain::ProductFitment, AppError> {
            Err(AppError::InvalidData("not used".into()))
        }
    }

    struct Generations;
    #[async_trait]
    impl GenerationApplication for Generations {
        async fn create(&self, command: CreateGeneration) -> Result<Generation, AppError> {
            Ok(Generation {
                id: crate::domain::GenerationId(mongodb::bson::oid::ObjectId::new().to_hex()),
                vehicle_model_id: command.vehicle_model_id,
                name: command.name,
                generation_code: command.generation_code,
                timeframe: command.timeframe,
                metadata: crate::domain::EntityMetadata::new(),
            })
        }
        async fn list_active(&self) -> Result<Vec<Generation>, AppError> {
            Ok(vec![])
        }
    }

    #[async_trait]
    impl RequestAuthorizer for Allow {
        async fn authorize(
            &self,
            _identity: RequestIdentity,
            _permission: Permission,
        ) -> Result<Principal, AppError> {
            Ok(Principal {
                subject: "test-admin".to_owned(),
            })
        }
    }

    struct Metrics;
    impl MetricsSink for Metrics {
        fn record_request(&self, _metric: RequestMetric<'_>) {}
        fn increment(&self, _name: &'static str) {}
    }

    fn state() -> AppState {
        AppState::new(
            Arc::new(Products),
            Arc::new(VehicleModels),
            AdminApplications {
                generations: Arc::new(Generations),
                brands: Arc::new(AdminCatalog),
                categories: Arc::new(AdminCatalog),
                trims: Arc::new(AdminCatalog),
                engines: Arc::new(AdminCatalog),
                configurations: Arc::new(AdminCatalog),
                fitments: Arc::new(AdminCatalog),
            },
            Arc::new(Allow),
            Arc::new(Metrics),
        )
    }

    #[tokio::test]
    async fn public_product_list_has_cache_headers() {
        let request = lambda_http::http::Request::<()>::builder()
            .uri("/products")
            .body(Body::Empty)
            .unwrap();
        let response = handle_request(request, state()).await;
        assert_eq!(response.status(), 200);
        assert!(response.headers().contains_key("etag"));
        assert!(response.headers()["cache-control"]
            .to_str()
            .unwrap()
            .contains("s-maxage"));
    }

    #[tokio::test]
    async fn vehicle_model_writes_live_only_under_admin_routes() {
        let public_request = lambda_http::http::Request::<()>::builder()
            .method(Method::POST)
            .uri("/vehicle-models")
            .body(Body::Text(r#"{"name":"911","modelCode":"911"}"#.to_owned()))
            .unwrap();
        assert_eq!(handle_request(public_request, state()).await.status(), 404);

        let admin_request = lambda_http::http::Request::<()>::builder()
            .method(Method::POST)
            .uri("/admin/vehicle-models")
            .body(Body::Text(r#"{"name":"911","modelCode":"911"}"#.to_owned()))
            .unwrap();
        assert_eq!(handle_request(admin_request, state()).await.status(), 201);
    }

    #[tokio::test]
    async fn packaged_product_asset_is_public_and_cacheable() {
        let request = lambda_http::http::Request::<()>::builder()
            .uri("/assets/products/product-placeholder.png")
            .body(Body::Empty)
            .expect("asset request should build");
        let response = handle_request(request, state()).await;
        assert_eq!(response.status(), 200);
        assert_eq!(response.headers()["content-type"], "image/png");
        assert!(response.headers()["cache-control"]
            .to_str()
            .expect("cache-control should be valid")
            .contains("s-maxage"));
        assert!(matches!(response.body(), Body::Binary(bytes) if !bytes.is_empty()));
    }

    #[tokio::test]
    async fn preflight_requests_allow_only_configured_origins() {
        let request = lambda_http::http::Request::<()>::builder()
            .method(Method::OPTIONS)
            .uri("/products")
            .header("origin", "http://localhost:3000")
            .body(Body::Empty)
            .expect("preflight request should build");
        let response = handle_request(request, state()).await;
        assert_eq!(response.status(), 204);
        assert_eq!(
            response.headers()["access-control-allow-origin"],
            "http://localhost:3000"
        );

        let denied = lambda_http::http::Request::<()>::builder()
            .method(Method::OPTIONS)
            .uri("/products")
            .header("origin", "https://untrusted.example")
            .body(Body::Empty)
            .expect("preflight request should build");
        let denied_response = handle_request(denied, state()).await;
        assert_eq!(denied_response.status(), 204);
        assert!(!denied_response
            .headers()
            .contains_key("access-control-allow-origin"));
    }
}

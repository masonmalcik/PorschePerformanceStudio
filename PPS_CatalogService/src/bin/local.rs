use axum::{
    body::{to_bytes, Body as AxumBody},
    extract::{Request, State},
    response::Response,
    Router,
};
use lambda_http::Body;
use pps_catalog_service::{
    application::{
        BrandApplication, CatalogBrandService, CatalogCategoryService, CatalogEngineService,
        CatalogGenerationService, CatalogProductFitmentService, CatalogProductService,
        CatalogTrimService, CatalogVehicleConfigurationService, CatalogVehicleModelService,
        CategoryApplication, EngineApplication, GenerationApplication, MetricsSink,
        ProductApplication, ProductFitmentApplication, RequestAuthorizer, TrimApplication,
        VehicleConfigurationApplication, VehicleModelApplication,
    },
    data::{
        assets::FileSystemImageStore,
        cache::{CachedProductRepository, ProductCache, ProductCacheBackend},
        mongodb::{
            connection::connect,
            repositories::{
                MongoBrandRepository, MongoCategoryRepository, MongoEngineRepository,
                MongoGenerationRepository, MongoProductFitmentRepository, MongoProductRepository,
                MongoTrimRepository, MongoVehicleConfigurationRepository,
                MongoVehicleModelRepository,
            },
        },
        observability::TracingMetrics,
        search::{ElasticsearchProductSearch, SearchableProductRepository},
        security::{CognitoAuthorizer, DevelopmentAuthorizer, DisabledAuthorizer},
    },
    presentation::{handle_request, AdminApplications, AppState},
    Config,
};
use std::{net::SocketAddr, sync::Arc, time::Duration};
use tracing::warn;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json().with_target(false))
        .init();
    let state = Arc::new(build_state(Config::from_env()?).await?);
    let app = Router::new().fallback(proxy).with_state(state);
    let address = SocketAddr::from((
        [0, 0, 0, 0],
        std::env::var("PORT")
            .ok()
            .and_then(|value| value.parse().ok())
            .unwrap_or(3003),
    ));
    let listener = tokio::net::TcpListener::bind(address).await?;
    tracing::info!(%address, "Catalog local HTTP server listening");
    axum::serve(listener, app).await?;
    Ok(())
}

async fn proxy(State(state): State<Arc<AppState>>, request: Request) -> Response {
    let (parts, body) = request.into_parts();
    let bytes = match to_bytes(body, 10 * 1024 * 1024).await {
        Ok(bytes) => bytes,
        Err(_) => {
            return Response::builder()
                .status(400)
                .body(AxumBody::from("invalid request body"))
                .expect("static response")
        }
    };
    let request = lambda_http::Request::from_parts(parts, Body::Binary(bytes.to_vec()));
    let response = handle_request(request, (*state).clone()).await;
    let (parts, body) = response.into_parts();
    let body = match body {
        Body::Empty => AxumBody::empty(),
        Body::Text(value) => AxumBody::from(value),
        Body::Binary(value) => AxumBody::from(value),
        _ => AxumBody::empty(),
    };
    Response::from_parts(parts, body)
}

async fn build_state(config: Config) -> Result<AppState, pps_catalog_service::AppError> {
    let database = connect(&config).await?;
    let mongo_products: Arc<dyn pps_catalog_service::application::ProductRepository> =
        Arc::new(MongoProductRepository::new(database.clone()));
    let product_search = config.elasticsearch_url.as_deref().and_then(|endpoint| match ElasticsearchProductSearch::new(endpoint, Duration::from_millis(config.elasticsearch_timeout_ms)) {
        Ok(search) => Some(Arc::new(search)),
        Err(error) => { warn!(error = %error, "Elasticsearch initialization failed; database search remains active"); None }
    });
    let primary_products: Arc<dyn pps_catalog_service::application::ProductRepository> = Arc::new(
        SearchableProductRepository::new(mongo_products, product_search),
    );
    let product_cache = if config.redis_cluster_urls.is_empty() {
        None
    } else {
        ProductCache::connect(
            &config.redis_mode,
            config.redis_cluster_urls.clone(),
            config.redis_pool_size,
        )
        .ok()
        .map(|cache| Arc::new(cache) as Arc<dyn ProductCacheBackend>)
    };
    let products = Arc::new(CachedProductRepository::new(
        primary_products,
        product_cache,
    ));
    let brands = Arc::new(MongoBrandRepository::new(database.clone()));
    let categories = Arc::new(MongoCategoryRepository::new(database.clone()));
    let vehicle_models = Arc::new(MongoVehicleModelRepository::new(database.clone()));
    let generations = Arc::new(MongoGenerationRepository::new(database.clone()));
    let trims = Arc::new(MongoTrimRepository::new(database.clone()));
    let configurations = Arc::new(MongoVehicleConfigurationRepository::new(database.clone()));
    let product_service: Arc<dyn ProductApplication> = Arc::new(CatalogProductService::new(
        products,
        brands.clone(),
        categories.clone(),
        Arc::new(FileSystemImageStore::new(&config.asset_root)),
    ));
    let authorizer: Arc<dyn RequestAuthorizer> = match config.admin_auth_mode.as_str() {
        "development" => Arc::new(DevelopmentAuthorizer::new(
            config.admin_development_token.clone().ok_or_else(|| {
                pps_catalog_service::AppError::Configuration("ADMIN_DEV_TOKEN must be set".into())
            })?,
        )?),
        "cognito" => Arc::new(CognitoAuthorizer::new(
            config.cognito_issuer.clone().ok_or_else(|| {
                pps_catalog_service::AppError::Configuration("COGNITO_ISSUER must be set".into())
            })?,
            config.cognito_client_id.clone().ok_or_else(|| {
                pps_catalog_service::AppError::Configuration("COGNITO_CLIENT_ID must be set".into())
            })?,
            config.cognito_jwks_url.clone().ok_or_else(|| {
                pps_catalog_service::AppError::Configuration("COGNITO_JWKS_URL must be set".into())
            })?,
        )?),
        "disabled" => Arc::new(DisabledAuthorizer),
        value => {
            return Err(pps_catalog_service::AppError::Configuration(format!(
                "unsupported ADMIN_AUTH_MODE: {value}"
            )))
        }
    };
    let metrics: Arc<dyn MetricsSink> = Arc::new(TracingMetrics);
    Ok(AppState::new(
        product_service,
        Arc::new(CatalogVehicleModelService::new(vehicle_models.clone()))
            as Arc<dyn VehicleModelApplication>,
        AdminApplications {
            generations: Arc::new(CatalogGenerationService::new(generations.clone()))
                as Arc<dyn GenerationApplication>,
            brands: Arc::new(CatalogBrandService::new(brands)) as Arc<dyn BrandApplication>,
            categories: Arc::new(CatalogCategoryService::new(categories))
                as Arc<dyn CategoryApplication>,
            trims: Arc::new(CatalogTrimService::new(
                trims.clone(),
                vehicle_models.clone(),
                generations.clone(),
            )) as Arc<dyn TrimApplication>,
            engines: Arc::new(CatalogEngineService::new(
                Arc::new(MongoEngineRepository::new(database.clone())),
                vehicle_models,
                generations,
                trims,
            )) as Arc<dyn EngineApplication>,
            configurations: Arc::new(CatalogVehicleConfigurationService::new(configurations))
                as Arc<dyn VehicleConfigurationApplication>,
            fitments: Arc::new(CatalogProductFitmentService::new(Arc::new(
                MongoProductFitmentRepository::new(database),
            ))) as Arc<dyn ProductFitmentApplication>,
        },
        authorizer,
        metrics,
    ))
}

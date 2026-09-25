use lambda_http::{run, service_fn, Error};
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
        security::{DevelopmentAuthorizer, DisabledAuthorizer},
    },
    presentation::{handle_request, AdminApplications, AppState},
    Config,
};
use std::sync::Arc;
use tracing_subscriber::{layer::SubscriberExt, util::SubscriberInitExt};

#[tokio::main]
async fn main() -> Result<(), Error> {
    dotenvy::dotenv().ok();
    tracing_subscriber::registry()
        .with(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .with(tracing_subscriber::fmt::layer().json().with_target(false))
        .init();

    let config = Config::from_env()?;
    let database = connect(&config).await?;
    let products = Arc::new(MongoProductRepository::new(database.clone()));
    let brands = Arc::new(MongoBrandRepository::new(database.clone()));
    let categories = Arc::new(MongoCategoryRepository::new(database.clone()));
    let vehicle_models = Arc::new(MongoVehicleModelRepository::new(database.clone()));
    let generations = Arc::new(MongoGenerationRepository::new(database.clone()));
    let trims = Arc::new(MongoTrimRepository::new(database.clone()));
    let engines = Arc::new(MongoEngineRepository::new(database.clone()));
    let configurations = Arc::new(MongoVehicleConfigurationRepository::new(database.clone()));
    let fitments = Arc::new(MongoProductFitmentRepository::new(database.clone()));
    let images = Arc::new(FileSystemImageStore::new(&config.asset_root));
    let product_service: Arc<dyn ProductApplication> = Arc::new(CatalogProductService::new(
        products,
        brands.clone(),
        categories.clone(),
        images,
    ));
    let vehicle_model_service: Arc<dyn VehicleModelApplication> =
        Arc::new(CatalogVehicleModelService::new(vehicle_models.clone()));
    let generation_service: Arc<dyn GenerationApplication> =
        Arc::new(CatalogGenerationService::new(generations.clone()));
    let brand_service: Arc<dyn BrandApplication> = Arc::new(CatalogBrandService::new(brands));
    let category_service: Arc<dyn CategoryApplication> =
        Arc::new(CatalogCategoryService::new(categories));
    let trim_service: Arc<dyn TrimApplication> = Arc::new(CatalogTrimService::new(
        trims.clone(),
        vehicle_models.clone(),
        generations.clone(),
    ));
    let engine_service: Arc<dyn EngineApplication> = Arc::new(CatalogEngineService::new(
        engines,
        vehicle_models,
        generations,
        trims,
    ));
    let configuration_service: Arc<dyn VehicleConfigurationApplication> =
        Arc::new(CatalogVehicleConfigurationService::new(configurations));
    let fitment_service: Arc<dyn ProductFitmentApplication> =
        Arc::new(CatalogProductFitmentService::new(fitments));
    let authorizer: Arc<dyn RequestAuthorizer> = match config.admin_auth_mode.as_str() {
        "development" => Arc::new(DevelopmentAuthorizer::new(
            config.admin_development_token.clone().ok_or_else(|| {
                pps_catalog_service::AppError::Configuration(
                    "ADMIN_DEV_TOKEN must be set in development auth mode".into(),
                )
            })?,
        )?),
        "disabled" => Arc::new(DisabledAuthorizer),
        value => {
            return Err(pps_catalog_service::AppError::Configuration(format!(
                "unsupported ADMIN_AUTH_MODE: {value}"
            ))
            .into())
        }
    };
    let metrics: Arc<dyn MetricsSink> = Arc::new(TracingMetrics);
    let state = AppState::new(
        product_service,
        vehicle_model_service,
        AdminApplications {
            generations: generation_service,
            brands: brand_service,
            categories: category_service,
            trims: trim_service,
            engines: engine_service,
            configurations: configuration_service,
            fitments: fitment_service,
        },
        authorizer,
        metrics,
    );

    run(service_fn(move |request| {
        let state = state.clone();
        async move { Ok::<_, Error>(handle_request(request, state).await) }
    }))
    .await
}

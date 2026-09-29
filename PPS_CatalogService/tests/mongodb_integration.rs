use mongodb::{bson::doc, options::IndexOptions, Client, IndexModel};

use pps_catalog_service::{
    application::VehicleModelRepository, data::mongodb::repositories::MongoVehicleModelRepository,
    domain::CreateVehicleModel, AppError,
};

#[tokio::test]
#[ignore = "requires MONGODB_URI and creates a temporary database"]
async fn vehicle_model_repository_maps_duplicate_codes_to_conflict() {
    let uri = std::env::var("MONGODB_URI").expect("MONGODB_URI must be set");
    let client = Client::with_uri_str(uri).await.expect("MongoDB connection");
    let database_name = format!("pps_ct_{}", mongodb::bson::oid::ObjectId::new().to_hex());
    let database = client.database(&database_name);
    database
        .collection::<mongodb::bson::Document>("vehicle_models")
        .create_index(
            IndexModel::builder()
                .keys(doc! { "modelCode": 1 })
                .options(IndexOptions::builder().unique(true).build())
                .build(),
        )
        .await
        .expect("unique index");

    let repository = MongoVehicleModelRepository::new(database.clone());
    let command = || CreateVehicleModel {
        name: "911".into(),
        model_code: "911".into(),
        is_active: None,
    };
    repository.create(command()).await.expect("first insert");
    let duplicate = repository.create(command()).await;

    database.drop().await.expect("temporary database cleanup");
    assert!(matches!(duplicate, Err(AppError::Conflict(_))));
}

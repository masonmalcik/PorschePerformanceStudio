use crate::AppError;
use futures_util::TryStreamExt;
use mongodb::{
    bson::{doc, DateTime, Document},
    options::IndexOptions,
    Database, IndexModel,
};

const COLLECTIONS: &[&str] = &[
    "brands",
    "categories",
    "products",
    "vehicle_models",
    "vehicle_generations",
    "vehicle_trims",
    "vehicle_configurations",
    "engines",
    "product_fitments",
    "_catalog_migrations",
    "_catalog_seed_runs",
];

pub async fn migrate(database: &Database) -> Result<(), AppError> {
    let existing = database.list_collection_names().await?;
    for name in COLLECTIONS {
        if !existing.iter().any(|value| value == name) {
            database.create_collection(*name).await?;
        }
    }

    database
        .collection::<Document>("brands")
        .update_many(
            doc! { "imageName": { "$exists": false } },
            doc! { "$set": { "imageName": "pps-performance.png", "updatedAt": DateTime::now() } },
        )
        .await?;
    database
        .collection::<Document>("products")
        .update_many(
            doc! { "imageName": { "$exists": false } },
            doc! { "$set": { "imageName": "product-placeholder.png", "updatedAt": DateTime::now() } },
        )
        .await?;

    let trims = database.collection::<Document>("vehicle_trims");
    let generations = database.collection::<Document>("vehicle_generations");
    let mut missing_models = trims
        .find(doc! { "vehicleModels": { "$exists": false } })
        .await?;
    while let Some(trim) = missing_models.try_next().await? {
        let trim_id = trim
            .get_object_id("_id")
            .map_err(|_| AppError::InvalidData("vehicle trim has no valid _id".into()))?;
        let generation_id = trim.get_object_id("generationId").map_err(|_| {
            AppError::InvalidData(format!("vehicle trim {trim_id} has no valid generationId"))
        })?;
        let generation = generations
            .find_one(doc! { "_id": generation_id })
            .await?
            .ok_or_else(|| {
                AppError::InvalidData(format!(
                    "vehicle trim {trim_id} refers to a missing generation"
                ))
            })?;
        let vehicle_model_id = generation.get_object_id("vehicleModelId").map_err(|_| {
            AppError::InvalidData(format!(
                "generation {generation_id} has no valid vehicleModelId"
            ))
        })?;
        trims.update_one(
            doc! { "_id": trim_id, "vehicleModels": { "$exists": false } },
            doc! { "$set": { "vehicleModels": [vehicle_model_id], "updatedAt": DateTime::now() }, "$inc": { "version": 1_i64 } },
        ).await?;
    }
    let engines = database.collection::<Document>("engines");
    let mut legacy_engines = engines
        .find(doc! { "vehicleTrims": { "$exists": false } })
        .await?;
    while let Some(engine) = legacy_engines.try_next().await? {
        let id = engine
            .get_object_id("_id")
            .map_err(|_| AppError::InvalidData("engine has no valid _id".into()))?;
        let trim_id = engine
            .get_object_id("vehicleTrim")
            .map_err(|_| AppError::InvalidData(format!("engine {id} has no valid vehicleTrim")))?;
        engines.update_one(
            doc! { "_id": id, "vehicleTrims": { "$exists": false } },
            doc! { "$set": { "vehicleTrims": [trim_id], "updatedAt": DateTime::now() }, "$inc": { "version": 1_i64 } },
        ).await?;
    }
    database
        .collection::<Document>("products")
        .update_many(
            doc! { "saleType": { "$in": ["new", "remanufactured", "used"] } },
            doc! { "$set": { "saleType": "retail", "updatedAt": DateTime::now() } },
        )
        .await?;

    apply_validators(database).await?;

    database
        .collection::<Document>("brands")
        .create_index(unique("brandCode_1", doc! { "brandCode": 1 }))
        .await?;
    database
        .collection::<Document>("categories")
        .create_indexes([
            unique("categoryCode_1", doc! { "categoryCode": 1 }),
            index("parentId_1", doc! { "parentId": 1 }),
        ])
        .await?;
    database
        .collection::<Document>("products")
        .create_indexes([
            unique("sku_1", doc! { "sku": 1 }),
            index(
                "brandId_1_modelNumber_1",
                doc! { "brandId": 1, "modelNumber": 1 },
            ),
            index("categoryIds_1", doc! { "categoryIds": 1 }),
            index("isActive_1_name_1", doc! { "isActive": 1, "name": 1 }),
        ])
        .await?;
    database
        .collection::<Document>("vehicle_models")
        .create_index(unique("modelCode_1", doc! { "modelCode": 1 }))
        .await?;
    database
        .collection::<Document>("vehicle_generations")
        .create_index(unique("generationCode_1", doc! { "generationCode": 1 }))
        .await?;
    database
        .collection::<Document>("vehicle_trims")
        .create_indexes([
            unique(
                "generationId_1_trimCode_1",
                doc! { "generationId": 1, "trimCode": 1 },
            ),
            index("vehicleModels_1", doc! { "vehicleModels": 1 }),
        ])
        .await?;
    database
        .collection::<Document>("vehicle_configurations")
        .create_index(unique(
            "trimId_1_modelYear_1",
            doc! { "trimId": 1, "modelYear": 1 },
        ))
        .await?;
    engines
        .create_indexes([
            unique(
                "vehicleTrims_1_factoryCode_1",
                doc! { "vehicleTrims": 1, "factoryCode": 1 },
            ),
            index(
                "vehicleModel_1_vehicleGeneration_1",
                doc! { "vehicleModel": 1, "vehicleGeneration": 1 },
            ),
        ])
        .await?;
    let existing_indexes: Vec<IndexModel> = engines.list_indexes().await?.try_collect().await?;
    if existing_indexes.iter().any(|index| {
        index
            .options
            .as_ref()
            .and_then(|options| options.name.as_deref())
            == Some("vehicleTrim_1_factoryCode_1")
    }) {
        engines.drop_index("vehicleTrim_1_factoryCode_1").await?;
    }
    engines
        .update_many(
            doc! { "vehicleTrim": { "$exists": true } },
            doc! { "$unset": { "vehicleTrim": "" } },
        )
        .await?;
    database
        .collection::<Document>("product_fitments")
        .create_indexes([
            index(
                "productId_1_isActive_1",
                doc! { "productId": 1, "isActive": 1 },
            ),
            index(
                "vehicleScope.trimId_1_system_1_component_1",
                doc! { "vehicleScope.trimId": 1, "system": 1, "component": 1 },
            ),
            index(
                "vehicleScope.generationId_1_system_1_component_1",
                doc! { "vehicleScope.generationId": 1, "system": 1, "component": 1 },
            ),
        ])
        .await?;

    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 1_i32 }, doc! { "$setOnInsert": { "name": "initial_catalog_schema", "checksum": "catalog-v1", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 2_i32 }, doc! { "$setOnInsert": { "name": "require_brand_image_name", "checksum": "catalog-v2", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 3_i32 }, doc! { "$setOnInsert": { "name": "require_product_image_name", "checksum": "catalog-v3", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 4_i32 }, doc! { "$setOnInsert": { "name": "replace_product_condition_with_sale_type", "checksum": "catalog-v4", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 5_i32 }, doc! { "$setOnInsert": { "name": "add_trim_vehicle_models", "checksum": "catalog-v5", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 6_i32 }, doc! { "$setOnInsert": { "name": "add_engines", "checksum": "catalog-v6", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 7_i32 }, doc! { "$setOnInsert": { "name": "engine_vehicle_trims_array", "checksum": "catalog-v7", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 8_i32 }, doc! { "$setOnInsert": { "name": "engine_horsepower_rpm", "checksum": "catalog-v8", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    database.collection::<Document>("_catalog_migrations").update_one(
        doc! { "version": 9_i32 }, doc! { "$setOnInsert": { "name": "engine_fuel_delivery", "checksum": "catalog-v9", "appliedAt": DateTime::now() } }
    ).upsert(true).await?;
    Ok(())
}

async fn apply_validators(database: &Database) -> Result<(), AppError> {
    let validators = [
        (
            "brands",
            doc! { "$jsonSchema": { "bsonType": "object", "required": ["brandCode", "name", "imageName", "createdAt", "updatedAt", "isActive", "version"], "properties": { "brandCode": { "bsonType": "string" }, "name": { "bsonType": "string" }, "imageName": { "bsonType": "string", "minLength": 1 }, "isActive": { "bsonType": "bool" } } } },
        ),
        (
            "categories",
            doc! { "$jsonSchema": { "bsonType": "object", "required": ["categoryCode", "name", "createdAt", "updatedAt", "isActive", "version"], "properties": { "categoryCode": { "bsonType": "string" }, "name": { "bsonType": "string" }, "attributes": { "bsonType": "array" } } } },
        ),
        (
            "products",
            doc! { "$jsonSchema": { "bsonType": "object", "required": ["sku", "brandId", "price", "currency", "name", "imageName", "saleType", "categoryIds", "createdAt", "updatedAt", "isActive", "version"], "properties": { "sku": { "bsonType": "string" }, "brandId": { "bsonType": "objectId" }, "price": { "bsonType": "decimal" }, "currency": { "bsonType": "string" }, "imageName": { "bsonType": "string", "minLength": 1 }, "saleType": { "enum": ["retail", "sale", "clearance"] }, "categoryIds": { "bsonType": "array" }, "isActive": { "bsonType": "bool" } } } },
        ),
        (
            "vehicle_models",
            entity_validator(
                doc! { "name": { "bsonType": "string" }, "modelCode": { "bsonType": "string" } },
                &["name", "modelCode"],
            ),
        ),
        (
            "vehicle_generations",
            entity_validator(
                doc! { "vehicleModelId": { "bsonType": "objectId" }, "name": { "bsonType": "string" }, "generationCode": { "bsonType": "string" }, "timeframe": { "bsonType": "object" } },
                &["vehicleModelId", "name", "generationCode", "timeframe"],
            ),
        ),
        (
            "vehicle_trims",
            entity_validator(
                doc! { "generationId": { "bsonType": "objectId" }, "vehicleModels": { "bsonType": "array", "minItems": 1, "maxItems": 1, "items": { "bsonType": "objectId" } }, "name": { "bsonType": "string" }, "trimCode": { "bsonType": "string" }, "timeframe": { "bsonType": "object" } },
                &[
                    "generationId",
                    "vehicleModels",
                    "name",
                    "trimCode",
                    "timeframe",
                ],
            ),
        ),
        (
            "vehicle_configurations",
            entity_validator(
                doc! { "trimId": { "bsonType": "objectId" }, "modelYear": { "bsonType": "int", "minimum": 1900, "maximum": 2200 } },
                &["trimId", "modelYear"],
            ),
        ),
        (
            "engines",
            entity_validator(
                doc! {
                    "vehicleModel": { "bsonType": "objectId" },
                    "vehicleGeneration": { "bsonType": "objectId" },
                    "vehicleTrims": { "bsonType": "array", "minItems": 1, "maxItems": 20, "uniqueItems": true, "items": { "bsonType": "objectId" } },
                    "alloyMaterial": { "bsonType": "string", "minLength": 1 },
                    "factoryCode": { "bsonType": "string", "minLength": 1 },
                    "displacement": { "bsonType": "double", "minimum": 0.0, "exclusiveMinimum": true, "maximum": 20.0 },
                    "horsepower": { "bsonType": "int", "minimum": 1 },
                    "rpm": { "bsonType": "int", "minimum": 1 },
                    "layout": { "enum": ["F6", "F4", "I4", "I5", "V6", "V8"] },
                    "aspirationType": { "enum": ["naturally_aspirated", "supercharged", "turbocharged", "twin_turbocharged"] },
                    "fuelDelivery": { "enum": ["fuel_injected", "carbureted"] },
                },
                &[
                    "vehicleModel",
                    "vehicleGeneration",
                    "vehicleTrims",
                    "alloyMaterial",
                    "factoryCode",
                    "displacement",
                    "horsepower",
                    "rpm",
                    "layout",
                    "aspirationType",
                    "fuelDelivery",
                ],
            ),
        ),
        (
            "product_fitments",
            entity_validator(
                doc! { "productId": { "bsonType": "objectId" }, "vehicleScope": { "bsonType": "object" }, "system": { "bsonType": "string" }, "component": { "bsonType": "string" } },
                &["productId", "vehicleScope", "system", "component"],
            ),
        ),
    ];
    for (collection, validator) in validators {
        database.run_command(doc! { "collMod": collection, "validator": validator, "validationLevel": "strict", "validationAction": "error" }).await?;
    }
    Ok(())
}

fn entity_validator(properties: Document, fields: &[&str]) -> Document {
    let mut required = fields.to_vec();
    required.extend(["createdAt", "updatedAt", "isActive", "version"]);
    doc! { "$jsonSchema": { "bsonType": "object", "required": required, "properties": properties } }
}

fn index(name: &str, keys: Document) -> IndexModel {
    IndexModel::builder()
        .keys(keys)
        .options(IndexOptions::builder().name(name.to_owned()).build())
        .build()
}
fn unique(name: &str, keys: Document) -> IndexModel {
    IndexModel::builder()
        .keys(keys)
        .options(
            IndexOptions::builder()
                .name(name.to_owned())
                .unique(true)
                .build(),
        )
        .build()
}

use serde_json::{json, Value};

pub fn document() -> Value {
    json!({
                    "openapi": "3.1.0",
                    "info": { "title": "PPS Catalog Service", "version": env!("CARGO_PKG_VERSION") },
                    "servers": [{ "url": "http://localhost:9000", "description": "Local Cargo Lambda" }],
                    "paths": {
                        "/products": { "get": {
                            "summary": "Search public products", "operationId": "searchProducts",
                            "parameters": [
                                { "name": "pageSize", "in": "query", "schema": { "type": "integer", "minimum": 1, "maximum": 100, "default": 25 } },
                                { "name": "cursor", "in": "query", "schema": { "type": "string" } },
                                { "name": "brandId", "in": "query", "schema": { "type": "string" } },
                                { "name": "categoryId", "in": "query", "schema": { "type": "string" } },
                                { "name": "saleType", "in": "query", "schema": { "type": "string", "enum": ["retail", "sale", "clearance"] } },
                                { "name": "minPrice", "in": "query", "schema": { "type": "string" } },
                                { "name": "maxPrice", "in": "query", "schema": { "type": "string" } },
                                { "name": "q", "in": "query", "schema": { "type": "string", "maxLength": 100 } },
                                { "name": "sort", "in": "query", "schema": { "type": "string", "enum": ["name", "newest", "price"] } },
                                { "name": "direction", "in": "query", "schema": { "type": "string", "enum": ["asc", "desc"] } }
                            ],
                            "responses": { "200": { "description": "A page of public products" }, "304": { "description": "Not modified" }, "400": { "description": "Invalid query" } }
                        }},
                        "/products/{id}": { "get": { "summary": "Get a public product", "operationId": "getProduct", "responses": { "200": { "description": "Product" }, "404": { "description": "Not found" } } } },
                        "/categories": { "get": { "summary": "List active public categories", "operationId": "listCategories", "responses": { "200": { "description": "Active categories" } } } },
                        "/admin/products": { "get": { "summary":"List active product options", "security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Product options"}}}, "post": { "summary": "Create a product", "operationId": "createProduct", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "201": { "description": "Created" }, "401": { "description": "Unauthorized" }, "403": { "description": "Forbidden" } } } },
                        "/admin/products/{id}": {
                            "patch": { "summary": "Update a product", "operationId": "updateProduct", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "200": { "description": "Updated" }, "400": { "description": "Invalid request" } } },
                            "delete": { "summary": "Deactivate a product", "operationId": "deactivateProduct", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "204": { "description": "Deactivated" } } }
                        },
                        "/admin/vehicle-models": {
                            "get": { "summary": "List active vehicle models for administration", "operationId": "listVehicleModels", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "200": { "description": "Vehicle model options" } } },
                            "post": { "summary": "Create a vehicle model", "operationId": "createVehicleModel", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "201": { "description": "Created" }, "409": { "description": "Duplicate modelCode" } } }
                        }
                        ,"/admin/vehicle-models/{id}": {"get":{"summary":"Get a vehicle model","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Vehicle model"},"404":{"description":"Not found"}}},"patch":{"summary":"Update a vehicle model","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"},"409":{"description":"Duplicate modelCode"}}}}
                        ,"/admin/vehicle-generations/{id}": {"get":{"summary":"Get a vehicle generation","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Generation"}}},"patch":{"summary":"Update a vehicle generation","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"}}}}
                        ,"/admin/vehicle-trims/{id}": {"get":{"summary":"Get a vehicle trim","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Trim"}}},"patch":{"summary":"Update a vehicle trim","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"}}}}
                        ,"/admin/brands/{id}": {"get":{"summary":"Get a brand","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Brand"}}},"patch":{"summary":"Update a brand","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"}}}}
                        ,"/admin/categories/{id}": {"get":{"summary":"Get a category","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Category"}}},"patch":{"summary":"Update a category","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"}}}}
                        ,"/admin/vehicle-configurations/{id}": {"get":{"summary":"Get a vehicle configuration","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Configuration"}}},"patch":{"summary":"Update a vehicle configuration","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"}}}}
                        ,"/admin/engines/{id}": {"get":{"summary":"Get an engine","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Engine"}}},"patch":{"summary":"Update an engine","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"}}}}
                        ,"/admin/product-fitments/{id}": {"get":{"summary":"Get a product fitment","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Fitment"}}},"patch":{"summary":"Update a product fitment","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Updated"}}}}
                        ,"/admin/vehicle-generations": { "post": { "summary": "Create a vehicle generation", "operationId": "createVehicleGeneration", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "201": { "description": "Created" }, "409": { "description": "Duplicate generationCode" } } } }
                        ,"/admin/categories": { "get": { "summary": "List active categories", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "200": { "description": "Category options" } } }, "post": { "summary": "Create a category", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "201": { "description": "Created" } } } }
                        ,"/admin/brands": { "get": { "summary": "List active brands", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "responses": { "200": { "description": "Brand options" } } }, "post":{"summary":"Create a brand","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"201":{"description":"Created"}}} }
                        ,"/admin/vehicle-trims": { "get":{"summary":"List active vehicle trims","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"200":{"description":"Trim options"}}}, "post": { "summary": "Create a vehicle trim", "security": [{ "localAdmin": [] }, { "cognito": ["pps-catalog/admin"] }], "requestBody": { "required": true, "content": { "application/json": { "schema": { "type": "object", "required": ["generationId", "vehicleModels", "name", "trimCode", "timeframe"], "properties": { "generationId": { "type": "string" }, "vehicleModels": { "type": "array", "minItems": 1, "maxItems": 1, "items": { "type": "string" } }, "name": { "type": "string" }, "trimCode": { "type": "string" }, "timeframe": { "type": "object" } } } } } }, "responses": { "201": { "description": "Created" }, "400": { "description": "Invalid vehicle model IDs or generation relationship" } } } }
                        ,"/admin/vehicle-configurations":{"post":{"summary":"Create a vehicle configuration","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"201":{"description":"Created"}}}}
    ,"/admin/engines": {"post": {"summary": "Create an engine", "operationId": "createEngine", "security": [{"localAdmin": []}, {"cognito": ["pps-catalog/admin"]}], "requestBody": {"required": true, "content": {"application/json": {"schema": {"type": "object", "additionalProperties": false, "required": ["vehicleModel", "vehicleGeneration", "vehicleTrims", "alloyMaterial", "factoryCode", "displacement", "horsepower", "rpm", "layout", "aspirationType", "fuelDelivery"], "properties": {"vehicleModel": {"type": "string", "pattern": "^[a-fA-F0-9]{24}$"}, "vehicleGeneration": {"type": "string", "pattern": "^[a-fA-F0-9]{24}$"}, "vehicleTrims": {"type": "array", "minItems": 1, "maxItems": 20, "uniqueItems": true, "items": {"type": "string", "pattern": "^[a-fA-F0-9]{24}$"}}, "alloyMaterial": {"type": "string", "minLength": 1, "maxLength": 100}, "factoryCode": {"type": "string", "minLength": 1, "maxLength": 64}, "displacement": {"type": "number", "exclusiveMinimum": 0, "maximum": 20, "description": "Liters"}, "horsepower": {"type": "integer", "minimum": 1, "maximum": 2147483647}, "rpm": {"type": "integer", "minimum": 1, "maximum": 2147483647}, "layout": {"type": "string", "enum": ["F6", "F4", "I4", "I5", "V6", "V8"]}, "aspirationType": {"type": "string", "enum": ["naturally_aspirated", "supercharged", "turbocharged", "twin_turbocharged"]}, "fuelDelivery": {"type": "string", "enum": ["fuel_injected", "carbureted"]}}}}}}, "responses": {"201": {"description": "Created"}, "400": {"description": "Invalid fields or vehicle relationships"}, "409": {"description": "Factory code already exists for a selected trim"}}}}
                        ,"/admin/product-fitments":{"post":{"summary":"Create a product fitment","security":[{"localAdmin":[]},{"cognito":["pps-catalog/admin"]}],"responses":{"201":{"description":"Created"}}}}
                    },
                    "components": { "securitySchemes": {
                        "localAdmin": { "type": "apiKey", "in": "header", "name": "x-pps-admin-key" },
                        "cognito": { "type": "oauth2", "flows": { "authorizationCode": { "authorizationUrl": "https://example.auth.us-east-1.amazoncognito.com/oauth2/authorize", "tokenUrl": "https://example.auth.us-east-1.amazoncognito.com/oauth2/token", "scopes": { "pps-catalog/admin": "Administer catalog" } } } }
                    }}
                })
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn contract_contains_public_and_admin_routes() {
        let paths = document()["paths"].as_object().unwrap().clone();
        assert!(paths.contains_key("/products"));
        assert!(paths.contains_key("/categories"));
        assert!(paths.contains_key("/admin/vehicle-models"));
        assert!(paths.contains_key("/admin/engines"));
        let engine_schema = &paths["/admin/engines"]["post"]["requestBody"]["content"]
            ["application/json"]["schema"];
        assert!(engine_schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("horsepower")));
        assert!(engine_schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("rpm")));
        assert!(engine_schema["required"]
            .as_array()
            .unwrap()
            .contains(&json!("fuelDelivery")));
        assert_eq!(
            engine_schema["properties"]["fuelDelivery"]["enum"],
            json!(["fuel_injected", "carbureted"])
        );
        assert_eq!(
            paths["/admin/engines"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["properties"]["vehicleTrims"]["minItems"],
            json!(1)
        );
        assert_eq!(
            paths["/admin/engines"]["post"]["requestBody"]["content"]["application/json"]["schema"]
                ["properties"]["layout"]["enum"],
            json!(["F6", "F4", "I4", "I5", "V6", "V8"])
        );
        assert!(paths["/admin/vehicle-models"]["post"]["security"].is_array());
        assert!(
            paths["/admin/vehicle-trims"]["post"]["requestBody"]["content"]["application/json"]
                ["schema"]["required"]
                .as_array()
                .unwrap()
                .contains(&json!("vehicleModels"))
        );
    }
}

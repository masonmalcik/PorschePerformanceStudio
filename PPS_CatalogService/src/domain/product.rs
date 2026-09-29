use super::{BrandId, CategoryId, EntityMetadata, Money, ProductId};
use serde::{Deserialize, Serialize};

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum SaleType {
    Retail,
    Sale,
    Clearance,
}

pub type ProductAttribute = String;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(rename_all = "camelCase")]
pub struct Product {
    pub id: ProductId,
    pub sku: String,
    pub model_number: Option<String>,
    pub brand_id: BrandId,
    pub price: Money,
    pub name: String,
    pub attributes: Vec<ProductAttribute>,
    pub description: Option<String>,
    pub image_name: String,
    pub image_urls: Vec<String>,
    pub sale_type: SaleType,
    pub category_ids: Vec<CategoryId>,
    pub metadata: EntityMetadata,
}

#[derive(Clone, Debug, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct CreateProduct {
    pub sku: String,
    pub model_number: Option<String>,
    pub brand_id: BrandId,
    pub price: Money,
    pub name: String,
    #[serde(default)]
    pub attributes: Vec<ProductAttribute>,
    pub description: Option<String>,
    pub image_name: String,
    #[serde(default)]
    pub image_urls: Vec<String>,
    pub sale_type: SaleType,
    #[serde(default)]
    pub category_ids: Vec<CategoryId>,
}

#[derive(Clone, Debug, Default, Deserialize)]
#[serde(rename_all = "camelCase", deny_unknown_fields)]
pub struct UpdateProduct {
    pub sku: Option<String>,
    pub brand_id: Option<BrandId>,
    pub model_number: Option<String>,
    pub price: Option<Money>,
    pub name: Option<String>,
    pub attributes: Option<Vec<ProductAttribute>>,
    pub description: Option<String>,
    pub image_name: Option<String>,
    pub image_urls: Option<Vec<String>>,
    pub sale_type: Option<SaleType>,
    pub category_ids: Option<Vec<CategoryId>>,
    pub is_active: Option<bool>,
}

#[derive(Clone, Copy, Debug)]
pub enum ProductSort {
    Name,
    Newest,
    Price,
}

#[derive(Clone, Copy, Debug)]
pub enum SortDirection {
    Ascending,
    Descending,
}

#[derive(Clone, Debug)]
pub struct ProductQuery {
    pub page_size: u32,
    pub cursor: Option<String>,
    pub brand_id: Option<BrandId>,
    pub category_id: Option<CategoryId>,
    pub sale_type: Option<SaleType>,
    pub minimum_price: Option<rust_decimal::Decimal>,
    pub maximum_price: Option<rust_decimal::Decimal>,
    pub search: Option<String>,
    pub sort: ProductSort,
    pub direction: SortDirection,
}

#[derive(Clone, Debug)]
pub struct ProductPage {
    pub items: Vec<Product>,
    pub next_cursor: Option<String>,
}

impl CreateProduct {
    pub fn validate(&self) -> Result<(), String> {
        if self.sku.trim().is_empty() {
            return Err("sku is required".into());
        }
        if self.name.trim().is_empty() {
            return Err("name is required".into());
        }
        if self
            .model_number
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err("modelNumber is required".into());
        }
        if self
            .description
            .as_ref()
            .is_none_or(|value| value.trim().is_empty())
        {
            return Err("description is required".into());
        }
        if self.category_ids.is_empty() {
            return Err("at least one categoryId is required".into());
        }
        if self.image_name.trim().is_empty() {
            return Err("imageName is required".into());
        }
        if self.sku.len() > 64
            || self.name.len() > 200
            || self.image_name.len() > 255
            || self
                .model_number
                .as_ref()
                .is_some_and(|value| value.len() > 100)
            || self
                .description
                .as_ref()
                .is_some_and(|value| value.len() > 5000)
        {
            return Err("one or more product fields exceed their maximum length".into());
        }
        if self.category_ids.len() > 20 {
            return Err("categoryIds cannot contain more than 20 values".into());
        }
        if self.attributes.len() > 50
            || self
                .attributes
                .iter()
                .any(|attribute| attribute.trim().is_empty() || attribute.len() > 100)
        {
            return Err("attributes must contain no more than 50 non-empty strings of 100 characters or fewer".into());
        }
        self.price.validate()
    }
}

#[cfg(test)]
mod tests {
    use super::CreateProduct;
    use serde_json::{json, Value};

    fn valid_product() -> Value {
        json!({
            "sku": "PPS-001",
            "modelNumber": "MODEL-001",
            "brandId": "68c8b9a713c53c1d8d950e20",
            "price": { "amount": "100.00", "currency": "USD" },
            "name": "Test Product",
            "description": "A test product",
            "imageName": "test.webp",
            "saleType": "retail",
            "categoryIds": ["68c8b9a713c53c1d8d950e21"]
        })
    }

    #[test]
    fn creation_requires_category_model_number_and_description() {
        for (field, empty_value) in [
            ("categoryIds", json!([])),
            ("modelNumber", json!(" ")),
            ("description", json!(" ")),
        ] {
            let mut input = valid_product();
            input[field] = empty_value;
            let command: CreateProduct = serde_json::from_value(input).unwrap();
            assert!(command.validate().is_err(), "{field} should be required");
        }
        let command: CreateProduct = serde_json::from_value(valid_product()).unwrap();
        assert!(command.validate().is_ok());
    }
}

use crate::{
    domain::{BrandId, CategoryId, ProductQuery, ProductSort, SaleType, SortDirection},
    AppError,
};
use rust_decimal::Decimal;
use std::{collections::HashMap, str::FromStr};

const ALLOWED: &[&str] = &[
    "pageSize",
    "cursor",
    "brandId",
    "categoryId",
    "saleType",
    "minPrice",
    "maxPrice",
    "q",
    "sort",
    "direction",
];

pub fn parse_product_query(query: Option<&str>) -> Result<ProductQuery, AppError> {
    let mut values = HashMap::<String, String>::new();
    for (key, value) in url::form_urlencoded::parse(query.unwrap_or_default().as_bytes()) {
        if !ALLOWED.contains(&key.as_ref()) {
            return Err(AppError::BadRequest(format!(
                "unsupported query parameter: {key}"
            )));
        }
        if values
            .insert(key.into_owned(), value.into_owned())
            .is_some()
        {
            return Err(AppError::BadRequest("duplicate query parameter".into()));
        }
    }
    let page_size = parse_optional::<u32>(&values, "pageSize")?.unwrap_or(25);
    if !(1..=100).contains(&page_size) {
        return Err(AppError::BadRequest(
            "pageSize must be between 1 and 100".into(),
        ));
    }
    let brand_id = values
        .get("brandId")
        .map(|value| validated_id(value).map(BrandId))
        .transpose()?;
    let category_id = values
        .get("categoryId")
        .map(|value| validated_id(value).map(CategoryId))
        .transpose()?;
    let sale_type = values
        .get("saleType")
        .map(|value| match value.as_str() {
            "retail" => Ok(SaleType::Retail),
            "sale" => Ok(SaleType::Sale),
            "clearance" => Ok(SaleType::Clearance),
            _ => Err(AppError::BadRequest("invalid saleType".into())),
        })
        .transpose()?;
    let minimum_price = parse_optional::<Decimal>(&values, "minPrice")?;
    let maximum_price = parse_optional::<Decimal>(&values, "maxPrice")?;
    if minimum_price.is_some_and(|value| value.is_sign_negative())
        || maximum_price.is_some_and(|value| value.is_sign_negative())
    {
        return Err(AppError::BadRequest(
            "price filters cannot be negative".into(),
        ));
    }
    if minimum_price
        .zip(maximum_price)
        .is_some_and(|(min, max)| min > max)
    {
        return Err(AppError::BadRequest(
            "minPrice cannot exceed maxPrice".into(),
        ));
    }
    let search = values
        .get("q")
        .map(|value| value.trim().to_owned())
        .filter(|value| !value.is_empty());
    if search.as_ref().is_some_and(|value| value.len() > 100) {
        return Err(AppError::BadRequest(
            "q cannot exceed 100 characters".into(),
        ));
    }
    let sort = match values.get("sort").map(String::as_str).unwrap_or("newest") {
        "name" => ProductSort::Name,
        "newest" => ProductSort::Newest,
        "price" => ProductSort::Price,
        _ => {
            return Err(AppError::BadRequest(
                "sort must be name, newest, or price".into(),
            ))
        }
    };
    let direction = match values
        .get("direction")
        .map(String::as_str)
        .unwrap_or("desc")
    {
        "asc" => SortDirection::Ascending,
        "desc" => SortDirection::Descending,
        _ => return Err(AppError::BadRequest("direction must be asc or desc".into())),
    };
    Ok(ProductQuery {
        page_size,
        cursor: values.get("cursor").cloned(),
        brand_id,
        category_id,
        sale_type,
        minimum_price,
        maximum_price,
        search,
        sort,
        direction,
    })
}

fn parse_optional<T: FromStr>(
    values: &HashMap<String, String>,
    key: &str,
) -> Result<Option<T>, AppError> {
    values
        .get(key)
        .map(|value| {
            value
                .parse()
                .map_err(|_| AppError::BadRequest(format!("invalid {key}")))
        })
        .transpose()
}

fn validated_id(value: &str) -> Result<String, AppError> {
    if value.len() == 24 && value.chars().all(|character| character.is_ascii_hexdigit()) {
        Ok(value.to_owned())
    } else {
        Err(AppError::BadRequest(
            "identifier must be a 24-character hexadecimal value".into(),
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn rejects_excessive_page_size() {
        assert!(parse_product_query(Some("pageSize=101")).is_err());
    }
    #[test]
    fn parses_filters_and_sorting() {
        let query =
            parse_product_query(Some("pageSize=10&saleType=retail&sort=name&direction=asc"))
                .unwrap();
        assert_eq!(query.page_size, 10);
        assert!(matches!(query.sort, ProductSort::Name));
    }
}

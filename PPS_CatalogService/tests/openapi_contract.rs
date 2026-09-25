use pps_catalog_service::presentation::openapi;

#[test]
fn published_contract_keeps_reads_public_and_writes_administrative() {
    let document = openapi::document();
    let paths = document["paths"].as_object().expect("paths object");

    assert!(paths["/products"]["get"].get("security").is_none());
    assert!(paths["/products/{id}"]["get"].get("security").is_none());
    for path in [
        "/admin/products",
        "/admin/products/{id}",
        "/admin/vehicle-models",
    ] {
        let operations = paths[path].as_object().expect("operations object");
        assert!(operations
            .values()
            .all(|operation| operation["security"].is_array()));
    }
    assert!(paths.get("/vehicle-models").is_none());
}

#[test]
fn product_search_contract_documents_bounded_cursor_pagination() {
    let document = openapi::document();
    let parameters = document["paths"]["/products"]["get"]["parameters"]
        .as_array()
        .expect("parameters array");
    let names: Vec<_> = parameters
        .iter()
        .map(|value| value["name"].as_str().unwrap())
        .collect();

    assert!(names.contains(&"cursor"));
    assert!(names.contains(&"pageSize"));
    let page_size = parameters
        .iter()
        .find(|value| value["name"] == "pageSize")
        .unwrap();
    assert_eq!(page_size["schema"]["maximum"], 100);
}

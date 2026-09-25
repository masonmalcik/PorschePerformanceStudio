mod handler;
pub mod openapi;
mod product_query;
mod responses;
pub use handler::{handle_request, AdminApplications, AppState};

//! `/api` Contains the entire API

use axum::{Router, routing::get};

use crate::AppState;

mod v1;
mod versions;

pub fn router(path: &str, app_state: &'static AppState) -> Router<&'static AppState> {
    Router::new()
        .route(&format!("{path}/versions"), get(versions::versions))
        .nest(&format!("{path}/v1"), v1::router(app_state))
}

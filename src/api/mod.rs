//! `/api` Contains the entire API

use std::sync::Arc;

use axum::{Router, routing::get};

use crate::AppState;

mod v1;
mod versions;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/versions", get(versions::versions))
        .nest("/v1", v1::router())
}

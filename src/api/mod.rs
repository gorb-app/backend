//! `/api` Contains the entire API

use std::sync::Arc;

use axum::{routing::get, Router};

use crate::AppState;

mod v1;
mod versions;

pub fn router(path: &str, app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(&format!("{path}/versions"), get(versions::versions))
        .nest(&format!("{path}/v1"), v1::router(app_state))
}

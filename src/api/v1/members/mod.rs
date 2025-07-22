use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{any, delete, get, patch},
};

use crate::{AppState, api::v1::auth::CurrentUser};

mod uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{uuid}", get(uuid::get))
        .route("/{uuid}", delete(uuid::delete))
}

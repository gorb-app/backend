use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get},
};

use crate::AppState;

mod uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{uuid}", get(uuid::get))
        .route("/{uuid}", delete(uuid::delete))
}

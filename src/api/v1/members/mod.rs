use axum::{
    Router,
    routing::{delete, get, post},
};

use crate::AppState;

mod uuid;

pub fn router() -> Router<&'static AppState> {
    Router::new()
        .route("/{uuid}", get(uuid::get))
        .route("/{uuid}", delete(uuid::delete))
        .route("/{uuid}/ban", post(uuid::ban::post))
}

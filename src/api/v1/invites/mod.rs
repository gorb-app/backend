use std::sync::Arc;

use axum::{
    Router,
    routing::{get, post},
};

use crate::AppState;

mod id;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{id}", get(id::get))
        .route("/{id}", post(id::join))
}

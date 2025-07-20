use std::sync::Arc;

use axum::{
    Router,
    routing::{delete, get, patch},
};
//use socketioxide::SocketIo;

use crate::AppState;

mod uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/{uuid}", get(uuid::get))
        .route("/{uuid}", delete(uuid::delete))
        .route("/{uuid}", patch(uuid::patch))
        .route("/{uuid}/messages", get(uuid::messages::get))
}

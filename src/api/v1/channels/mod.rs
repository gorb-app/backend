use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{delete, get, patch},
};
//use socketioxide::SocketIo;

use crate::{AppState, api::v1::auth::CurrentUser};

mod uuid;

pub fn router(app_state: &'static AppState) -> Router<&'static AppState> {
    Router::new()
        .route("/{uuid}", get(uuid::get))
        .route("/{uuid}", delete(uuid::delete))
        .route("/{uuid}", patch(uuid::patch))
        .route("/{uuid}/messages", get(uuid::messages::get))
        .layer(from_fn_with_state(app_state, CurrentUser::check_auth_layer))
}

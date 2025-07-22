use std::sync::Arc;

use axum::{
    Router,
    middleware::from_fn_with_state,
    routing::{any, delete, get, patch},
};
//use socketioxide::SocketIo;

use crate::{AppState, api::v1::auth::CurrentUser};

mod uuid;

pub fn router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    let router_with_auth = Router::new()
        .route("/{uuid}", get(uuid::get))
        .route("/{uuid}", delete(uuid::delete))
        .layer(from_fn_with_state(app_state, CurrentUser::check_auth_layer));

    Router::new()
        .merge(router_with_auth)
}

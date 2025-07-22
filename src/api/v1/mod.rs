//! `/api/v1` Contains version 1 of the api

use std::sync::Arc;

use axum::{Router, middleware::from_fn_with_state, routing::get};

use crate::{AppState, api::v1::auth::CurrentUser};

mod auth;
mod channels;
mod guilds;
mod invites;
mod me;
mod stats;
mod users;
mod members;

pub fn router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    let router_with_auth = Router::new()
        .nest("/users", users::router())
        .nest("/guilds", guilds::router())
        .nest("/invites", invites::router())
        .nest("/members", members::router())
        .nest("/me", me::router())
        .layer(from_fn_with_state(
            app_state.clone(),
            CurrentUser::check_auth_layer,
        ));

    Router::new()
        .route("/stats", get(stats::res))
        .nest("/auth", auth::router(app_state.clone()))
        .nest("/channels", channels::router(app_state))
        .merge(router_with_auth)
}

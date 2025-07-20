//! `/api/v1` Contains version 1 of the api

use std::sync::Arc;

use axum::{middleware::from_fn_with_state, routing::get, Router};

use crate::{api::v1::auth::CurrentUser, AppState};

mod auth;
mod channels;
mod guilds;
mod invites;
mod me;
mod stats;
mod users;

pub fn router(app_state: Arc<AppState>) -> Router<Arc<AppState>> {
    let router_with_auth = Router::new()
        .nest("/users", users::router())
        .nest("/channels", channels::router())
        .nest("/guilds", guilds::router())
        .nest("/invites", invites::router())
        .nest("/me", me::router())
        .layer(from_fn_with_state(app_state.clone(), CurrentUser::check_auth_layer));

    Router::new()
        .route("/stats", get(stats::res))
        .nest("/auth", auth::router(app_state))
        .merge(router_with_auth)
}

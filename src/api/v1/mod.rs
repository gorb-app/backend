//! `/api/v1` Contains version 1 of the api

use axum::{Router, middleware::from_fn_with_state, routing::{any, get}};

use crate::{AppState, api::v1::auth::CurrentUser};

mod auth;
mod channels;
mod guilds;
mod invites;
mod me;
mod members;
mod stats;
mod users;
mod socket;

pub fn router(app_state: &'static AppState) -> Router<&'static AppState> {
    let router_with_auth = Router::new()
        .nest("/users", users::router())
        .nest("/guilds", guilds::router())
        .nest("/invites", invites::router())
        .nest("/members", members::router())
        .nest("/me", me::router())
        .layer(from_fn_with_state(app_state, CurrentUser::check_auth_layer));

    Router::new()
        .route("/stats", get(stats::res))
        .route("/socket", any(socket::ws))
        .nest("/auth", auth::router(app_state))
        .nest("/channels", channels::router(app_state))
        .merge(router_with_auth)
}

//! `/api/v1` Contains version 1 of the api

use std::sync::Arc;

use axum::{routing::get, Router};

use crate::AppState;

mod auth;
mod channels;
mod guilds;
mod invites;
mod me;
mod stats;
mod users;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/stats", get(stats::res))
        .nest("/auth", auth::router())
        .nest("/users", users::router())
        .nest("/channels", channels::router())
        .nest("/guilds", guilds::router())
        .nest("/invites", invites::router())
        .nest("/me", me::router())
}

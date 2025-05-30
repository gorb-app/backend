//! `/api/v1` Contains version 1 of the api

use actix_web::{Scope, web};

mod auth;
mod channels;
mod invites;
mod guilds;
mod stats;
mod users;
mod me;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
        .service(auth::web())
        .service(users::web())
        .service(channels::web())
        .service(guilds::web())
        .service(invites::web())
        .service(me::web())
}

//! `/api/v1` Contains version 1 of the api

use actix_web::{Scope, web};

mod auth;
mod channels;
mod guilds;
mod invites;
mod me;
mod stats;
mod users;
mod federation;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
        .service(auth::web())
        .service(users::web())
        .service(channels::web())
        .service(guilds::web())
        .service(invites::web())
        .service(me::web())
        .service(federation::web())
}

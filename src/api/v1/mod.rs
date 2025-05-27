use actix_web::{Scope, web};

mod auth;
mod invites;
mod servers;
mod stats;
mod users;
mod me;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
        .service(auth::web())
        .service(users::web())
        .service(servers::web())
        .service(invites::web())
        .service(me::web())
}

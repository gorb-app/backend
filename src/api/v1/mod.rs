use actix_web::{Scope, web};

mod auth;
mod stats;
mod users;
mod servers;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
        .service(auth::web())
        .service(users::web())
        .service(servers::web())
}

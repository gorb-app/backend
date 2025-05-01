use actix_web::{Scope, web};

mod stats;
mod auth;
mod user;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
        .service(auth::web())
        .service(user::res)
}

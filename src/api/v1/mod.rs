use actix_web::{Scope, web};

mod stats;
mod register;
mod login;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
        .service(register::res)
        .service(login::res)
}

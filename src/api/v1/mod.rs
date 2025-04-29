use actix_web::{Scope, web};

mod stats;
mod register;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(stats::res)
        .service(register::res)
}

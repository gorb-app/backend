use actix_web::{Scope, web};

mod channel;
mod send;

pub fn web() -> Scope {
    web::scope("/v1")
        .service(channel::res)
        .service(send::res)
}

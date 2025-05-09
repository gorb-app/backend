use actix_web::{web, Scope};

mod id;

pub fn web() -> Scope {
    web::scope("/invites")
        .service(id::get)
        .service(id::join)
}

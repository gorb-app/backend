use actix_web::{Scope, web};

mod id;

pub fn web() -> Scope {
    web::scope("/invites").service(id::get).service(id::join)
}

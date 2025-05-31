use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{Scope, web};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

use crate::{Conn, error::Error, schema::access_tokens::dsl};

mod login;
mod logout;
mod refresh;
mod register;
mod reset_password;
mod revoke;
mod verify_email;

#[derive(Serialize)]
struct Response {
    access_token: String,
}

pub fn web() -> Scope {
    web::scope("/auth")
        .service(register::res)
        .service(login::response)
        .service(logout::res)
        .service(refresh::res)
        .service(revoke::res)
        .service(verify_email::get)
        .service(verify_email::post)
        .service(reset_password::get)
        .service(reset_password::post)
}

pub async fn check_access_token(access_token: &str, conn: &mut Conn) -> Result<Uuid, Error> {
    let (uuid, created_at): (Uuid, i64) = dsl::access_tokens
        .filter(dsl::token.eq(access_token))
        .select((dsl::uuid, dsl::created_at))
        .get_result(conn)
        .await
        .map_err(|error| {
            if error == diesel::result::Error::NotFound {
                Error::Unauthorized("Invalid access token".to_string())
            } else {
                Error::from(error)
            }
        })?;

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH)?.as_secs() as i64;

    let lifetime = current_time - created_at;

    if lifetime > 3600 {
        return Err(Error::Unauthorized("Invalid access token".to_string()));
    }

    Ok(uuid)
}

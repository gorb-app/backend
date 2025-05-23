use std::{
    sync::LazyLock,
    time::{SystemTime, UNIX_EPOCH},
};

use actix_web::{Scope, web};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use regex::Regex;
use serde::Serialize;
use uuid::Uuid;

use crate::{Conn, error::Error, schema::access_tokens::dsl};

mod login;
mod refresh;
mod register;
mod revoke;

#[derive(Serialize)]
struct Response {
    access_token: String,
}

static EMAIL_REGEX: LazyLock<Regex> = LazyLock::new(|| {
    Regex::new(r"[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+(?:\.[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+)*@(?:[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?").unwrap()
});

static USERNAME_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"^[a-z0-9_.-]+$").unwrap());

// Password is expected to be hashed using SHA3-384
static PASSWORD_REGEX: LazyLock<Regex> = LazyLock::new(|| Regex::new(r"[0-9a-f]{96}").unwrap());

pub fn web() -> Scope {
    web::scope("/auth")
        .service(register::res)
        .service(login::response)
        .service(refresh::res)
        .service(revoke::res)
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

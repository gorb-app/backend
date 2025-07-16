use std::{
    sync::Arc,
    time::{SystemTime, UNIX_EPOCH},
};

use axum::{
    Router,
    routing::{delete, get, post},
};
use diesel::{ExpressionMethods, QueryDsl};
use diesel_async::RunQueryDsl;
use uuid::Uuid;

use crate::{AppState, Conn, error::Error, schema::access_tokens::dsl};

mod devices;
mod login;
mod logout;
mod refresh;
mod register;
mod reset_password;
mod revoke;
mod verify_email;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/register", post(register::post))
        .route("/login", post(login::response))
        .route("/logout", delete(logout::res))
        .route("/refresh", post(refresh::post))
        .route("/revoke", post(revoke::post))
        .route("/verify-email", get(verify_email::get))
        .route("/verify-email", post(verify_email::post))
        .route("/reset-password", get(reset_password::get))
        .route("/reset-password", post(reset_password::post))
        .route("/devices", get(devices::get))
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

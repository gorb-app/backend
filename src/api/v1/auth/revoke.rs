use std::sync::Arc;

use argon2::{PasswordHash, PasswordVerifier};
use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::{
    TypedHeader,
    headers::authorization::{Authorization, Bearer},
};
use diesel::{ExpressionMethods, QueryDsl, delete};
use diesel_async::RunQueryDsl;
use serde::Deserialize;

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    schema::refresh_tokens::{self, dsl as rdsl},
    schema::users::dsl as udsl,
};

#[derive(Deserialize)]
pub struct RevokeRequest {
    password: String,
    device_name: String,
}

// TODO: Should maybe be a delete request?
#[axum::debug_handler]
pub async fn post(
    State(app_state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(revoke_request): Json<RevokeRequest>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    let database_password: String = udsl::users
        .filter(udsl::uuid.eq(uuid))
        .select(udsl::password)
        .get_result(&mut conn)
        .await?;

    let hashed_password = PasswordHash::new(&database_password)
        .map_err(|e| Error::PasswordHashError(e.to_string()))?;

    if app_state
        .argon2
        .verify_password(revoke_request.password.as_bytes(), &hashed_password)
        .is_err()
    {
        return Err(Error::Unauthorized(
            "Wrong username or password".to_string(),
        ));
    }

    delete(refresh_tokens::table)
        .filter(rdsl::uuid.eq(uuid))
        .filter(rdsl::device_name.eq(&revoke_request.device_name))
        .execute(&mut conn)
        .await?;

    Ok(StatusCode::OK)
}

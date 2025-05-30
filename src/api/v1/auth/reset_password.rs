//! `/api/v1/auth/reset-password` Endpoints for resetting user password

use actix_web::{HttpResponse, get, post, web};
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::{Data, error::Error, structs::PasswordResetToken};

#[derive(Deserialize)]
struct Query {
    identifier: String,
}

/// `GET /api/v1/auth/reset-password` Sends password reset email to user
///
/// requires auth? no
///
/// ### Query Parameters
/// identifier: Email or username
///
/// ### Responses
/// 200 Email sent
/// 429 Too Many Requests
/// 404 Not found
/// 400 Bad request
///
#[get("/reset-password")]
pub async fn get(query: web::Query<Query>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let mut conn = data.pool.get().await?;

    if let Ok(password_reset_token) =
        PasswordResetToken::get_with_identifier(&mut conn, query.identifier.clone()).await
    {
        if Utc::now().signed_duration_since(password_reset_token.created_at) > Duration::hours(1) {
            password_reset_token.delete(&mut conn).await?;
        } else {
            return Err(Error::TooManyRequests(
                "Please allow 1 hour before sending a new email".to_string(),
            ));
        }
    }

    PasswordResetToken::new(&data, query.identifier.clone()).await?;

    Ok(HttpResponse::Ok().finish())
}

#[derive(Deserialize)]
struct ResetPassword {
    password: String,
    token: String,
}

/// `POST /api/v1/auth/reset-password` Resets user password
///
/// requires auth? no
///
/// ### Request Example:
/// ```
/// json!({
///     "password": "1608c17a27f6ae3891c23d680c73ae91528f20a54dcf4973e2c3126b9734f48b7253047f2395b51bb8a44a6daa188003",
///     "token": "a3f7e29c1b8d0456e2c9f83b7a1d6e4f5028c3b9a7e1f2d5c6b8a0d3e7f4a2b"
/// });
/// ```
///
/// ### Responses
/// 200 Success
/// 410 Token Expired
/// 404 Not Found
/// 400 Bad Request
///
#[post("/reset-password")]
pub async fn post(
    reset_password: web::Json<ResetPassword>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let mut conn = data.pool.get().await?;

    let password_reset_token =
        PasswordResetToken::get(&mut conn, reset_password.token.clone()).await?;

    if Utc::now().signed_duration_since(password_reset_token.created_at) > Duration::hours(24) {
        password_reset_token.delete(&mut conn).await?;
        return Ok(HttpResponse::Gone().finish());
    }

    password_reset_token
        .set_password(&data, reset_password.password.clone())
        .await?;

    Ok(HttpResponse::Ok().finish())
}

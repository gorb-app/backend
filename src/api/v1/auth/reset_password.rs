//! `/api/v1/auth/reset-password` Endpoints for resetting user password

use axum::{
    Json,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::{AppState, error::Error, objects::PasswordResetToken};

#[derive(Deserialize)]
pub struct QueryParams {
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
///
/// 429 Too Many Requests
///
/// 404 Not found
///
/// 400 Bad request
///
pub async fn get(
    State(app_state): State<&'static AppState>,
    query: Query<QueryParams>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    if let Ok(password_reset_token) = PasswordResetToken::get_with_identifier(
        &mut conn,
        &app_state.cache_pool,
        query.identifier.clone(),
    )
    .await
    {
        if Utc::now().signed_duration_since(password_reset_token.created_at) > Duration::hours(1) {
            password_reset_token.delete(&app_state.cache_pool).await?;
        } else {
            return Err(Error::TooManyRequests(
                "Please allow 1 hour before sending a new email".to_string(),
            ));
        }
    }

    PasswordResetToken::new(&mut conn, &app_state, query.identifier.clone()).await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct ResetPassword {
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
///
/// 410 Token Expired
///
/// 404 Not Found
///
/// 400 Bad Request
///
pub async fn post(
    State(app_state): State<&'static AppState>,
    reset_password: Json<ResetPassword>,
) -> Result<impl IntoResponse, Error> {
    let password_reset_token =
        PasswordResetToken::get(&app_state.cache_pool, reset_password.token.clone()).await?;

    password_reset_token
        .set_password(
            &mut app_state.pool.get().await?,
            &app_state,
            reset_password.password.clone(),
        )
        .await?;

    Ok(StatusCode::OK)
}

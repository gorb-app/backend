//! `/api/v1/auth/verify-email` Endpoints for verifying user emails

use std::sync::Arc;

use axum::{
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{EmailToken, Me},
};

#[derive(Deserialize)]
pub struct QueryParams {
    token: String,
}

/// `GET /api/v1/auth/verify-email` Verifies user email address
///
/// requires auth? yes
///
/// ### Query Parameters
/// token
///
/// ### Responses
/// 200 Success
///
/// 204 Already verified
///
/// 410 Token Expired
///
/// 404 Not Found
///
/// 401 Unauthorized
///
pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Query(query): Query<QueryParams>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    let me = Me::get(&mut conn, uuid).await?;

    if me.email_verified {
        return Ok(StatusCode::NO_CONTENT);
    }

    let email_token = EmailToken::get(&app_state, me.uuid).await?;

    if query.token != email_token.token {
        return Ok(StatusCode::UNAUTHORIZED);
    }

    me.verify_email(&mut conn).await?;

    email_token.delete(&app_state).await?;

    Ok(StatusCode::OK)
}

/// `POST /api/v1/auth/verify-email` Sends user verification email
///
/// requires auth? yes
///
/// ### Responses
/// 200 Email sent
///
/// 204 Already verified
///
/// 429 Too Many Requests
///
/// 401 Unauthorized
///
pub async fn post(
    State(app_state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    let me = Me::get(&mut conn, uuid).await?;

    if me.email_verified {
        return Ok(StatusCode::NO_CONTENT);
    }

    if let Ok(email_token) = EmailToken::get(&app_state, me.uuid).await {
        if Utc::now().signed_duration_since(email_token.created_at) > Duration::hours(1) {
            email_token.delete(&app_state).await?;
        } else {
            return Err(Error::TooManyRequests(
                "Please allow 1 hour before sending a new email".to_string(),
            ));
        }
    }

    EmailToken::new(&app_state, me).await?;

    Ok(StatusCode::OK)
}

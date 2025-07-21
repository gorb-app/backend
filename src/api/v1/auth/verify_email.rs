//! `/api/v1/auth/verify-email` Endpoints for verifying user emails

use std::sync::Arc;

use axum::{
    Extension,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use chrono::{Duration, Utc};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
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
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let me = Me::get(&mut conn, uuid).await?;

    if me.email_verified {
        return Ok(StatusCode::NO_CONTENT);
    }

    let email_token = EmailToken::get(&app_state.cache_pool, me.uuid).await?;

    if query.token != email_token.token {
        return Ok(StatusCode::UNAUTHORIZED);
    }

    me.verify_email(&mut conn).await?;

    email_token.delete(&app_state.cache_pool).await?;

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
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let me = Me::get(&mut app_state.pool.get().await?, uuid).await?;

    if me.email_verified {
        return Ok(StatusCode::NO_CONTENT);
    }

    if let Ok(email_token) = EmailToken::get(&app_state.cache_pool, me.uuid).await {
        if Utc::now().signed_duration_since(email_token.created_at) > Duration::hours(1) {
            email_token.delete(&app_state.cache_pool).await?;
        } else {
            return Err(Error::TooManyRequests(
                "Please allow 1 hour before sending a new email".to_string(),
            ));
        }
    }

    EmailToken::new(&app_state, me).await?;

    Ok(StatusCode::OK)
}

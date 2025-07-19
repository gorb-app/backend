use std::sync::Arc;

use axum::{
    extract::State,
    http::{HeaderValue, StatusCode},
    response::IntoResponse,
};
use axum_extra::extract::CookieJar;
use diesel::{ExpressionMethods, delete};
use diesel_async::RunQueryDsl;

use crate::{
    AppState,
    error::Error,
    schema::refresh_tokens::{self, dsl},
};

/// `GET /api/v1/logout`
///
/// requires auth: kinda, needs refresh token set but no access token is technically required
///
/// ### Responses
///
/// 200 Logged out
///
/// 404 Refresh token is invalid
///
/// 401 Unauthorized (no refresh token found)
///
pub async fn res(
    State(app_state): State<Arc<AppState>>,
    jar: CookieJar,
) -> Result<impl IntoResponse, Error> {
    let mut refresh_token_cookie = jar
        .get("refresh_token")
        .ok_or(Error::Unauthorized(
            "request has no refresh token".to_string(),
        ))?
        .to_owned();

    let access_token_cookie = jar.get("access_token");

    let refresh_token = String::from(refresh_token_cookie.value_trimmed());

    let mut conn = app_state.pool.get().await?;

    let deleted = delete(refresh_tokens::table)
        .filter(dsl::token.eq(refresh_token))
        .execute(&mut conn)
        .await?;

    let mut response;

    if deleted == 0 {
        response = StatusCode::NOT_FOUND.into_response();
    } else {
        response = StatusCode::OK.into_response();
    }

    refresh_token_cookie.make_removal();
    response.headers_mut().append(
        "Set-Cookie",
        HeaderValue::from_str(&refresh_token_cookie.to_string())?,
    );

    if let Some(cookie) = access_token_cookie {
        let mut cookie = cookie.clone();
        cookie.make_removal();
        response
            .headers_mut()
            .append("Set-Cookie", HeaderValue::from_str(&cookie.to_string())?);
    }

    Ok(response)
}

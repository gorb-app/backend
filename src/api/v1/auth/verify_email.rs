//! `/api/v1/auth/verify-email` Endpoints for verifying user emails

use actix_web::{HttpRequest, HttpResponse, get, post, web};
use chrono::{Duration, Utc};
use serde::Deserialize;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{EmailToken, Me},
    utils::get_auth_header,
};

#[derive(Deserialize)]
struct Query {
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
#[get("/verify-email")]
pub async fn get(
    req: HttpRequest,
    query: web::Query<Query>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let me = Me::get(&mut conn, uuid).await?;

    if me.email_verified {
        return Ok(HttpResponse::NoContent().finish());
    }

    let email_token = EmailToken::get(&data, me.uuid).await?;

    if query.token != email_token.token {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    me.verify_email(&mut conn).await?;

    email_token.delete(&data).await?;

    Ok(HttpResponse::Ok().finish())
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
#[post("/verify-email")]
pub async fn post(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let me = Me::get(&mut conn, uuid).await?;

    if me.email_verified {
        return Ok(HttpResponse::NoContent().finish());
    }

    if let Ok(email_token) = EmailToken::get(&data, me.uuid).await {
        if Utc::now().signed_duration_since(email_token.created_at) > Duration::hours(1) {
            email_token.delete(&data).await?;
        } else {
            return Err(Error::TooManyRequests(
                "Please allow 1 hour before sending a new email".to_string(),
            ));
        }
    }

    EmailToken::new(&data, me).await?;

    Ok(HttpResponse::Ok().finish())
}

use actix_web::{Error, HttpRequest, HttpResponse, error, post, web};
use argon2::{PasswordHash, PasswordVerifier};
use futures::{StreamExt, future};
use log::error;
use serde::{Deserialize, Serialize};

use crate::{Data, api::v1::auth::check_access_token, utils::get_auth_header};

#[derive(Deserialize)]
struct RevokeRequest {
    password: String,
    device_name: String,
}

#[derive(Serialize)]
struct Response {
    deleted: bool,
}

impl Response {
    fn new(deleted: bool) -> Self {
        Self { deleted }
    }
}

const MAX_SIZE: usize = 262_144;

#[post("/revoke")]
pub async fn res(
    req: HttpRequest,
    mut payload: web::Payload,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    let revoke_request = serde_json::from_slice::<RevokeRequest>(&body)?;

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let database_password_raw = sqlx::query_scalar(&format!(
        "SELECT password FROM users WHERE uuid = '{}'",
        uuid
    ))
    .fetch_one(&data.pool)
    .await;

    if let Err(error) = database_password_raw {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().json(Response::new(false)));
    }

    let database_password: String = database_password_raw.unwrap();

    let hashed_password_raw = PasswordHash::new(&database_password);

    if let Err(error) = hashed_password_raw {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().json(Response::new(false)));
    }

    let hashed_password = hashed_password_raw.unwrap();

    if data
        .argon2
        .verify_password(revoke_request.password.as_bytes(), &hashed_password)
        .is_err()
    {
        return Ok(HttpResponse::Unauthorized().finish());
    }

    let tokens_raw = sqlx::query_scalar(&format!(
        "SELECT token FROM refresh_tokens WHERE uuid = '{}' AND device_name = $1",
        uuid
    ))
    .bind(revoke_request.device_name)
    .fetch_all(&data.pool)
    .await;

    if tokens_raw.is_err() {
        error!("{:?}", tokens_raw);
        return Ok(HttpResponse::InternalServerError().json(Response::new(false)));
    }

    let tokens: Vec<String> = tokens_raw.unwrap();

    let mut refresh_tokens_delete = vec![];

    for token in tokens {
        refresh_tokens_delete.push(
            sqlx::query("DELETE FROM refresh_tokens WHERE token = $1")
                .bind(token.clone())
                .execute(&data.pool),
        );
    }

    let results = future::join_all(refresh_tokens_delete).await;

    let errors: Vec<&Result<sqlx::postgres::PgQueryResult, sqlx::Error>> =
        results
            .iter()
            .filter(|r| r.is_err())
            .collect();

    if !errors.is_empty() {
        error!("{:?}", errors);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(Response::new(true)))
}

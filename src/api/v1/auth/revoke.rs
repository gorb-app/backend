use actix_web::{error, post, web, Error, HttpResponse};
use argon2::{PasswordHash, PasswordVerifier};
use serde::{Deserialize, Serialize};
use futures::{future, StreamExt};

use crate::{api::v1::auth::check_access_token, Data};

#[derive(Deserialize)]
struct RevokeRequest {
    access_token: String,
    password: String,
    device_name: String,
}

#[derive(Serialize)]
struct Response {
    deleted: bool,
}

impl Response {
    fn new(deleted: bool) -> Self {
        Self {
            deleted
        }
    }
}

const MAX_SIZE: usize = 262_144;

#[post("/revoke")]
pub async fn res(mut payload: web::Payload, data: web::Data<Data>) -> Result<HttpResponse, Error> {
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

    let authorized = check_access_token(revoke_request.access_token, &data.pool).await;

    if authorized.is_err() {
        return Ok(authorized.unwrap_err())
    }

    let uuid = authorized.unwrap();

    let database_password_raw = sqlx::query_scalar(&format!("SELECT password FROM users WHERE uuid = '{}'", uuid))
        .fetch_one(&data.pool)
        .await;

    if database_password_raw.is_err() {
        eprintln!("{}", database_password_raw.unwrap_err());
        return Ok(HttpResponse::InternalServerError().json(Response::new(false)));
    }

    let database_password: String = database_password_raw.unwrap();

    let hashed_password_raw = PasswordHash::new(&database_password);

    if hashed_password_raw.is_err() {
        eprintln!("{}", hashed_password_raw.unwrap_err());
        return Ok(HttpResponse::InternalServerError().json(Response::new(false)));
    }

    let hashed_password = hashed_password_raw.unwrap();

    if data.argon2.verify_password(revoke_request.password.as_bytes(), &hashed_password).is_err() {
        return Ok(HttpResponse::Unauthorized().finish())
    }

    let tokens_raw = sqlx::query_scalar(&format!("SELECT token FROM refresh_tokens WHERE uuid = '{}' AND device_name = $1", uuid))
        .bind(revoke_request.device_name)
        .fetch_all(&data.pool)
        .await;

    if tokens_raw.is_err() {
        eprintln!("{:?}", tokens_raw);
        return Ok(HttpResponse::InternalServerError().json(Response::new(false)))
    }

    let tokens: Vec<String> = tokens_raw.unwrap();

    let mut access_tokens_delete = vec![];
    let mut refresh_tokens_delete = vec![];


    for token in tokens {
        access_tokens_delete.push(sqlx::query("DELETE FROM access_tokens WHERE refresh_token = $1")
            .bind(token.clone())
            .execute(&data.pool));

        refresh_tokens_delete.push(sqlx::query("DELETE FROM refresh_tokens WHERE token = $1")
            .bind(token.clone())
            .execute(&data.pool));
    }

    let results_access_tokens = future::join_all(access_tokens_delete).await;
    let results_refresh_tokens = future::join_all(refresh_tokens_delete).await;

    let access_tokens_errors: Vec<&Result<sqlx::postgres::PgQueryResult, sqlx::Error>> = results_access_tokens.iter().filter(|r| r.is_err()).collect();
    let refresh_tokens_errors: Vec<&Result<sqlx::postgres::PgQueryResult, sqlx::Error>> = results_refresh_tokens.iter().filter(|r| r.is_err()).collect();

    if !access_tokens_errors.is_empty() && !refresh_tokens_errors.is_empty() {
        println!("{:?}", access_tokens_errors);
        println!("{:?}", refresh_tokens_errors);
        return Ok(HttpResponse::InternalServerError().finish())
    } else if !access_tokens_errors.is_empty() {
        println!("{:?}", access_tokens_errors);
        return Ok(HttpResponse::InternalServerError().finish())
    } else if !refresh_tokens_errors.is_empty() {
        println!("{:?}", refresh_tokens_errors);
        return Ok(HttpResponse::InternalServerError().finish())
    }
    
    Ok(HttpResponse::Ok().json(Response::new(true)))
}

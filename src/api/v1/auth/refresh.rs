use std::time::{SystemTime, UNIX_EPOCH};
use actix_web::{error, post, web, Error, HttpResponse};
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::Data;

#[derive(Deserialize)]
struct RefreshRequest {
    refresh_token: String,
}

#[derive(Serialize)]
struct Response {
    refresh_token: Option<String>,
    access_token: String,
    expires_in: u64,
}

const MAX_SIZE: usize = 262_144;

#[post("/refresh")]
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

    let refresh_request = serde_json::from_slice::<RefreshRequest>(&body)?;

    let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64;

    let row: (String, i64) = sqlx::query_as("SELECT CAST(uuid as VARCHAR), created FROM refresh_tokens WHERE token = $1")
        .bind(refresh_request.refresh_token)
        .fetch_one(&data.pool)
        .await
        .unwrap();

    let (uuid, created) = row;

    println!("{}, {}", uuid, created);

    Ok(HttpResponse::InternalServerError().finish())
}

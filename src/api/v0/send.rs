use std::time::{SystemTime, UNIX_EPOCH};

use actix_web::{Error, HttpResponse, error, post, web};
use futures::StreamExt;
use log::error;
use serde::Deserialize;

use crate::{Data, api::v1::auth::check_access_token};

#[derive(Deserialize)]
struct Request {
    access_token: String,
    message: String, 
}

const MAX_SIZE: usize = 262_144;

#[post("/channel")]
pub async fn res(
    mut payload: web::Payload,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    let request = serde_json::from_slice::<Request>(&body)?;

    let authorized = check_access_token(request.access_token, &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let current_time = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs() as i64;

    let row = sqlx::query(&format!("INSERT INTO channel (timestamp, uuid, message) VALUES ($1, '{}', $2)", uuid))
        .bind(current_time)
        .bind(request.message)
        .execute(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().finish())
}

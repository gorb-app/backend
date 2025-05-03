use actix_web::{Error, HttpResponse, error, post, web};
use futures::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use std::time::{SystemTime, UNIX_EPOCH};

mod uuid;
mod channels;

use crate::Data;

#[derive(Deserialize)]
struct Request {
    access_token: String,
    name: String
}

#[derive(Serialize)]
struct Response {
    refresh_token: String,
    access_token: String,
}

const MAX_SIZE: usize = 262_144;

pub fn web() -> Scope {
    web::scope("/servers")
        .service(channels::web())
        .service(uuid::res)
}

#[post("")]
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

    let request = serde_json::from_slice::<Request>(&body)?;

    Ok(HttpResponse::Unauthorized().finish())
}


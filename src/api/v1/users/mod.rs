use actix_web::{error, post, web, Error, HttpResponse, Scope};
use futures::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::prelude::FromRow;
use crate::{Data, api::v1::auth::check_access_token};

mod me;
mod uuid;

#[derive(Deserialize)]
struct Request {
    access_token: String,
    start: i32,
    amount: i32,
}

#[derive(Serialize, FromRow)]
struct Response {
    uuid: String,
    username: String,
    display_name: Option<String>,
    email: String,
}

const MAX_SIZE: usize = 262_144;

pub fn web() -> Scope {
    web::scope("/users")
        .service(res)
        .service(me::res)
        .service(uuid::res)
}

#[post("")]
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

    if request.amount > 100 {
        return Ok(HttpResponse::BadRequest().finish())
    }

    let authorized = check_access_token(request.access_token, &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let row = sqlx::query_as("SELECT CAST(uuid AS VARCHAR), username, display_name, email FROM users ORDER BY username LIMIT $1 OFFSET $2")
        .bind(request.amount)
        .bind(request.start)
        .fetch_all(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let accounts: Vec<Response> = row.unwrap();

    Ok(HttpResponse::Ok().json(accounts))
}


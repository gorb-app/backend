use actix_web::{error, post, web, Error, HttpRequest, HttpResponse, Scope};
use futures::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use ::uuid::Uuid;
use std::time::{SystemTime, UNIX_EPOCH};

mod uuid;

use crate::{api::v1::auth::check_access_token, utils::get_auth_header, Data};

#[derive(Deserialize)]
struct Request {
    name: String,
    description: Option<String>,
}

#[derive(Serialize)]
struct Response {
    guild_uuid: Uuid,
}

impl Response {
    fn new(guild_uuid: Uuid) -> Self {
        Self {
            guild_uuid
        }
    }
}

const MAX_SIZE: usize = 262_144;

pub fn web() -> Scope {
    web::scope("/servers")
        .service(res)
        .service(uuid::web())
}

#[post("")]
pub async fn res(req: HttpRequest, mut payload: web::Payload, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    let uuid = authorized.unwrap();

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

    let guild_uuid = Uuid::now_v7();

    let row = sqlx::query(&format!("INSERT INTO guilds (uuid, owner_uuid, name, description) VALUES ('{}', '{}', $1, $2)", guild_uuid, uuid))
        .bind(request.name)
        .bind(request.description)
        .execute(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish())
    }

    let row = sqlx::query(&format!("INSERT INTO guild_members (uuid, guild_uuid, user_uuid) VALUES ('{}', '{}', '{}')", Uuid::now_v7(), guild_uuid, uuid))
        .bind(SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs() as i64)
        .execute(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);

        let row = sqlx::query(&format!("DELETE FROM guilds WHERE uuid = '{}'", guild_uuid))
            .execute(&data.pool)
            .await;

        if let Err(error) = row {
            error!("{}", error);
        }

        return Ok(HttpResponse::InternalServerError().finish())
    }

    Ok(HttpResponse::Ok().json(Response::new(guild_uuid)))
}


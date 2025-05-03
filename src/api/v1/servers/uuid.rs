use actix_web::{error, post, web, Error, HttpResponse};
use futures::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use sqlx::FromRow;
use uuid::Uuid;
use std::str::FromStr;

use crate::{api::v1::auth::check_access_token, Data};

#[derive(Deserialize)]
struct Request {
    access_token: String,
}

#[derive(Serialize)]
struct Response {
    uuid: Uuid,
    name: String,
    description: Option<String>,
    icon: String,
    owner_uuid: Uuid,
    roles: Vec<Role>,
    member_count: i64,
}

#[derive(Serialize, FromRow)]
struct Role {
    uuid: String,
    name: String,
    color: i64,
    position: i32,
    permissions: i64,
}

const MAX_SIZE: usize = 262_144;

#[post("/{uuid}")]
pub async fn res(mut payload: web::Payload, path: web::Path<(Uuid,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    let guild_uuid = path.into_inner().0;

    let request = serde_json::from_slice::<Request>(&body)?;

    let authorized = check_access_token(request.access_token, &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error)
    }

    let uuid = authorized.unwrap();

    let row: Result<String, sqlx::Error> = sqlx::query_scalar(&format!("SELECT CAST(uuid AS VARCHAR) FROM guild_members WHERE guild_uuid = '{}' AND user_uuid = '{}'", guild_uuid, uuid))
        .fetch_one(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);

        return Ok(HttpResponse::InternalServerError().finish())
    }

    let member_uuid = Uuid::from_str(&row.unwrap()).unwrap();

    let row = sqlx::query_as(&format!("SELECT CAST(owner_uuid AS VARCHAR), name, description FROM guilds WHERE uuid = '{}'", guild_uuid))
        .fetch_one(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);

        return Ok(HttpResponse::InternalServerError().finish())
    }

    let (owner_uuid_raw, name, description): (String, String, Option<String>) = row.unwrap();

    let owner_uuid = Uuid::from_str(&owner_uuid_raw).unwrap();

    let row = sqlx::query_scalar(&format!("SELECT COUNT(uuid) FROM guild_members WHERE guild_uuid = '{}'", guild_uuid))
        .fetch_one(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);

        return Ok(HttpResponse::InternalServerError().finish())
    }

    let member_count: i64 = row.unwrap();

    let roles_raw = sqlx::query_as(&format!("SELECT (uuid, name, color, position, permissions) FROM roles WHERE guild_uuid = '{}'", guild_uuid))
        .fetch_all(&data.pool)
        .await;

    if let Err(error) = roles_raw {
        error!("{}", error);

        return Ok(HttpResponse::InternalServerError().finish())
    }

    let roles: Vec<Role> = roles_raw.unwrap();


    Ok(HttpResponse::Ok().json(Response {
        uuid,
        name,
        description,
        icon: "bogus".to_string(),
        owner_uuid,
        roles,
        member_count,
    }))
}


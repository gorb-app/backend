use std::str::FromStr;

use actix_web::{get, web, Error, HttpRequest, HttpResponse};
use serde::Serialize;
use sqlx::{prelude::FromRow, Pool, Postgres};
use crate::{api::v1::auth::check_access_token, utils::get_auth_header, Data};
use ::uuid::Uuid;
use log::error;

mod uuid;

#[derive(Serialize, FromRow)]
struct ChannelPermission {
    role_uuid: String,
    permissions: i32
}

#[derive(Serialize)]
struct Channel {
    uuid: String,
    name: String,
    description: Option<String>,
    permissions: Vec<ChannelPermission>
}

impl Channel {
    async fn fetch_all(pool: &Pool<Postgres>, guild_uuid: Uuid) -> Result<Vec<Self>, HttpResponse> {
        let row = sqlx::query_as(&format!("SELECT CAST(uuid AS VARCHAR), name, description FROM channels WHERE guild_uuid = '{}'", guild_uuid))
        .fetch_all(pool)
        .await;

        if let Err(error) = row {
            error!("{}", error);

            return Err(HttpResponse::InternalServerError().finish())
        }

        let channels: Vec<(String, String, Option<String>)> = row.unwrap();

        let futures = channels.iter().map(async |t| {
            let (uuid, name, description) = t.to_owned();

            let row = sqlx::query_as(&format!("SELECT CAST(role_uuid AS VARCHAR), permissions FROM channel_permissions WHERE channel_uuid = '{}'", uuid))
                .fetch_all(pool)
                .await;

            if let Err(error) = row {
                error!("{}", error);

                return Err(HttpResponse::InternalServerError().finish())
            }

            Ok(Self {
                uuid,
                name,
                description,
                permissions: row.unwrap(),
            })
        });

        let channels = futures::future::join_all(futures).await;

        let channels: Result<Vec<Channel>, HttpResponse> = channels.into_iter().collect();

        Ok(channels?)
    }
}

#[get("{uuid}/channels")]
pub async fn response(req: HttpRequest, path: web::Path<(Uuid,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error)
    }

    let guild_uuid = path.into_inner().0;

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

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


    let channels = Channel::fetch_all(&data.pool, guild_uuid).await;

    if let Err(error) = channels {
        return Ok(error)
    }

    Ok(HttpResponse::Ok().json(channels.unwrap()))
}

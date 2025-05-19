pub mod messages;
pub mod socket;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    structs::{Channel, Member},
    utils::get_auth_header,
};
use ::uuid::Uuid;
use actix_web::{Error, HttpRequest, HttpResponse, delete, get, web};
use log::error;

#[get("{uuid}/channels/{channel_uuid}")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let (guild_uuid, channel_uuid) = path.into_inner();

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    let cache_result = data.get_cache_key(format!("{}", channel_uuid)).await;

    if let Ok(cache_hit) = cache_result {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let channel_result = Channel::fetch_one(&data.pool, guild_uuid, channel_uuid).await;

    if let Err(error) = channel_result {
        return Ok(error);
    }

    let channel = channel_result.unwrap();

    let cache_result = data
        .set_cache_key(format!("{}", channel_uuid), channel.clone(), 60)
        .await;

    if let Err(error) = cache_result {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(channel))
}

#[delete("{uuid}/channels/{channel_uuid}")]
pub async fn delete(
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let (guild_uuid, channel_uuid) = path.into_inner();

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    let cache_result = data.get_cache_key(format!("{}", channel_uuid)).await;

    let channel: Channel;

    if let Ok(cache_hit) = cache_result {
        channel = serde_json::from_str(&cache_hit).unwrap();

        let result = data.del_cache_key(format!("{}", channel_uuid)).await;

        if let Err(error) = result {
            error!("{}", error)
        }
    } else {
        let channel_result = Channel::fetch_one(&data.pool, guild_uuid, channel_uuid).await;

        if let Err(error) = channel_result {
            return Ok(error);
        }

        channel = channel_result.unwrap();
    }

    let delete_result = channel.delete(&data.pool).await;

    if let Err(error) = delete_result {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().finish())
}

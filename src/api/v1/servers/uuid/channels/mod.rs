use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use serde::Deserialize;
use crate::{api::v1::auth::check_access_token, structs::{Channel, Member}, utils::get_auth_header, Data};
use ::uuid::Uuid;
use log::error;

pub mod uuid;

#[derive(Deserialize)]
struct ChannelInfo {
    name: String,
    description: Option<String>
}

#[get("{uuid}/channels")]
pub async fn get(req: HttpRequest, path: web::Path<(Uuid,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
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

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    let cache_result = data.get_cache_key(format!("{}_channels", guild_uuid)).await;

    if let Ok(cache_hit) = cache_result {
        return Ok(HttpResponse::Ok().content_type("application/json").body(cache_hit))
    }

    let channels_result = Channel::fetch_all(&data.pool, guild_uuid).await;

    if let Err(error) = channels_result {
        return Ok(error)
    }

    let channels = channels_result.unwrap();

    let cache_result = data.set_cache_key(format!("{}_channels", guild_uuid), channels.clone(), 1800).await;

    if let Err(error) = cache_result {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(channels))
}

#[post("{uuid}/channels")]
pub async fn create(req: HttpRequest, channel_info: web::Json<ChannelInfo>, path: web::Path<(Uuid,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
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

    let member = Member::fetch_one(&data.pool, uuid, guild_uuid).await;

    if let Err(error) = member {
        return Ok(error);
    }

    // FIXME: Logic to check permissions, should probably be done in utils.rs

    let channel = Channel::new(data.clone(), guild_uuid, channel_info.name.clone(), channel_info.description.clone()).await;

    if let Err(error) = channel {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(channel.unwrap()))
}

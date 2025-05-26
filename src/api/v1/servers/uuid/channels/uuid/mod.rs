pub mod messages;
pub mod socket;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    structs::{Channel, Member},
    utils::get_auth_header,
};
use actix_web::{HttpRequest, HttpResponse, delete, get, web};
use uuid::Uuid;

#[get("{uuid}/channels/{channel_uuid}")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let (guild_uuid, channel_uuid) = path.into_inner();

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = data.get_cache_key(format!("{}", channel_uuid)).await {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let channel = Channel::fetch_one(&mut conn, channel_uuid).await?;

    data.set_cache_key(format!("{}", channel_uuid), channel.clone(), 60)
        .await?;

    Ok(HttpResponse::Ok().json(channel))
}

#[delete("{uuid}/channels/{channel_uuid}")]
pub async fn delete(
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let (guild_uuid, channel_uuid) = path.into_inner();

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    let channel: Channel;

    if let Ok(cache_hit) = data.get_cache_key(format!("{}", channel_uuid)).await {
        channel = serde_json::from_str(&cache_hit)?;

        data.del_cache_key(format!("{}", channel_uuid)).await?;
    } else {
        channel = Channel::fetch_one(&mut conn, channel_uuid).await?;
    }

    channel.delete(&mut conn).await?;

    Ok(HttpResponse::Ok().finish())
}

use crate::{
    api::v1::auth::check_access_token, error::Error, structs::{Channel, Member}, utils::{get_auth_header, global_checks, order_by_is_above}, Data
};
use ::uuid::Uuid;
use actix_web::{HttpRequest, HttpResponse, get, post, web};
use serde::Deserialize;

#[derive(Deserialize)]
struct ChannelInfo {
    name: String,
    description: Option<String>,
}

#[get("{uuid}/channels")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = data.get_cache_key(format!("{}_channels", guild_uuid)).await {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let channels = Channel::fetch_all(&data.pool, guild_uuid).await?;

    let channels_ordered = order_by_is_above(channels).await?;

    data.set_cache_key(
        format!("{}_channels", guild_uuid),
        channels_ordered.clone(),
        1800,
    )
    .await?;

    Ok(HttpResponse::Ok().json(channels_ordered))
}

#[post("{uuid}/channels")]
pub async fn create(
    req: HttpRequest,
    channel_info: web::Json<ChannelInfo>,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    Member::fetch_one(&mut conn, uuid, guild_uuid).await?;

    // FIXME: Logic to check permissions, should probably be done in utils.rs

    let channel = Channel::new(
        data.clone(),
        guild_uuid,
        channel_info.name.clone(),
        channel_info.description.clone(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(channel))
}

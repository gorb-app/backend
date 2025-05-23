use crate::{
    error::Error,
    Data,
    api::v1::auth::check_access_token,
    structs::{Channel, Member},
    utils::get_auth_header,
};
use ::uuid::Uuid;
use actix_web::{HttpRequest, HttpResponse, get, web};
use serde::Deserialize;

#[derive(Deserialize)]
struct MessageRequest {
    amount: i64,
    offset: i64,
}

#[get("{uuid}/channels/{channel_uuid}/messages")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid, Uuid)>,
    message_request: web::Query<MessageRequest>,
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
        channel = serde_json::from_str(&cache_hit)?
    } else {
        channel = Channel::fetch_one(&mut conn, channel_uuid).await?;

        data
            .set_cache_key(format!("{}", channel_uuid), channel.clone(), 60)
            .await?;
    }

    let messages = channel
        .fetch_messages(&mut conn, message_request.amount, message_request.offset)
        .await?;

    Ok(HttpResponse::Ok().json(messages))
}

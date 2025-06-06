//! `/api/v1/channels/{uuid}/messages` Endpoints related to channel messages

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Channel, Member},
    utils::{get_auth_header, global_checks},
};
use ::uuid::Uuid;
use actix_web::{HttpRequest, HttpResponse, get, web};
use serde::Deserialize;

#[derive(Deserialize)]
struct MessageRequest {
    amount: i64,
    offset: i64,
}

/// `GET /api/v1/channels/{uuid}/messages` Returns user with the given UUID
///
/// requires auth: yes
///
/// requires relation: yes
///
/// ### Request Example
/// ```
/// json!({
///     "amount": 100,
///     "offset": 0
/// })
/// ```
///
/// ### Response Example
/// ```
/// json!({
///     "uuid": "01971976-8618-74c0-b040-7ffbc44823f6",
///     "channel_uuid": "0196fcb1-e886-7de3-b685-0ee46def9a7b",
///     "user_uuid": "0196fc96-a822-76b0-b9bf-a9de232f54b7",
///     "message": "test",
///     "user": {
///         "uuid": "0196fc96-a822-76b0-b9bf-a9de232f54b7",
///         "username": "1234",
///         "display_name": null,
///         "avatar": "https://cdn.gorb.app/avatar/0196fc96-a822-76b0-b9bf-a9de232f54b7/avatar.jpg"
///     }
/// });
/// ```
///
#[get("/{uuid}/messages")]
pub async fn get(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    message_request: web::Query<MessageRequest>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let channel_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let channel = Channel::fetch_one(&data, channel_uuid).await?;

    Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    let messages = channel
        .fetch_messages(&data, message_request.amount, message_request.offset)
        .await?;

    Ok(HttpResponse::Ok().json(messages))
}

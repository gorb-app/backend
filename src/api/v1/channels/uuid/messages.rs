//! `/api/v1/channels/{uuid}/messages` Endpoints related to channel messages

use std::sync::Arc;

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Channel, Member},
    utils::global_checks,
};
use ::uuid::Uuid;
use axum::{
    Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::Deserialize;

#[derive(Deserialize)]
pub struct MessageRequest {
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
pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(channel_uuid): Path<Uuid>,
    Query(message_request): Query<MessageRequest>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    global_checks(&app_state, uuid).await?;

    let channel = Channel::fetch_one(&app_state, channel_uuid).await?;

    Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    let messages = channel
        .fetch_messages(&app_state, message_request.amount, message_request.offset)
        .await?;

    Ok((StatusCode::OK, Json(messages)))
}

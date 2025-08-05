//! `/api/v1/channels/{uuid}` Channel specific endpoints

pub mod messages;
pub mod socket;

use std::sync::Arc;

use crate::{
    api::v1::auth::CurrentUser, error::Error, objects::{AuditLog, AuditLogId, Channel, Member, Permissions}, utils::global_checks, AppState
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use serde::Deserialize;
use uuid::Uuid;

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(channel_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let channel = Channel::fetch_one(&mut conn, &app_state.cache_pool, channel_uuid).await?;

    Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    Ok((StatusCode::OK, Json(channel)))
}

pub async fn delete(
    State(app_state): State<Arc<AppState>>,
    Path(channel_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let channel = Channel::fetch_one(&mut conn, &app_state.cache_pool, channel_uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    member
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::ManageChannel)
        .await?;

    let log_entrie = AuditLog::new(channel.guild_uuid, AuditLogId::ChannelDelete as i16, member.uuid, None, None, None, None, Some(channel.name.clone()), None, None).await;
    channel.delete(&mut conn, &app_state.cache_pool).await?;
    log_entrie.push(&mut conn).await?;

    Ok(StatusCode::OK)
}

#[derive(Deserialize)]
pub struct NewInfo {
    name: Option<String>,
    description: Option<String>,
    is_above: Option<String>,
}

/// `PATCH /api/v1/channels/{uuid}` Returns user with the given UUID
///
/// requires auth: yes
///
/// requires relation: yes
///
/// ### Request Example
/// All fields are optional and can be nulled/dropped if only changing 1 value
/// ```
/// json!({
///     "name": "gaming-chat",
///     "description": "Gaming related topics.",
///     "is_above": "398f6d7b-752c-4348-9771-fe6024adbfb1"
/// });
/// ```
///
/// ### Response Example
/// ```
/// json!({
///     uuid: "cdcac171-5add-4f88-9559-3a247c8bba2c",
///     guild_uuid: "383d2afa-082f-4dd3-9050-ca6ed91487b6",
///     name: "gaming-chat",
///     description: "Gaming related topics.",
///     is_above: "398f6d7b-752c-4348-9771-fe6024adbfb1",
///     permissions: {
///         role_uuid: "79cc0806-0f37-4a06-a468-6639c4311a2d",
///         permissions: 0
///     }
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
pub async fn patch(
    State(app_state): State<Arc<AppState>>,
    Path(channel_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(new_info): Json<NewInfo>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let mut channel = Channel::fetch_one(&mut conn, &app_state.cache_pool, channel_uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, channel.guild_uuid).await?;

    member
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::ManageChannel)
        .await?;

    if let Some(new_name) = &new_info.name {
        channel
            .set_name(&mut conn, &app_state.cache_pool, new_name.to_string())
            .await?;
    }

    if let Some(new_description) = &new_info.description {
        channel
            .set_description(
                &mut conn,
                &app_state.cache_pool,
                new_description.to_string(),
            )
            .await?;
    }

    if let Some(new_is_above) = &new_info.is_above {
        channel
            .set_description(&mut conn, &app_state.cache_pool, new_is_above.to_string())
            .await?;
    }

    Ok((StatusCode::OK, Json(channel)))
}

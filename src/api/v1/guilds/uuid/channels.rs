use ::uuid::Uuid;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{Channel, Member, Permissions},
    utils::{CacheFns, global_checks, order_by_is_above},
};

#[derive(Deserialize)]
pub struct ChannelInfo {
    name: String,
    description: Option<String>,
}

pub async fn get(
    State(app_state): State<&'static AppState>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = app_state
        .cache_pool
        .get_cache_key::<Vec<Channel>>(format!("{guild_uuid}_channels"))
        .await
    {
        return Ok((StatusCode::OK, Json(cache_hit)).into_response());
    }

    let channels = Channel::fetch_all(&mut conn, guild_uuid).await?;

    let channels_ordered = order_by_is_above(channels).await?;

    app_state
        .cache_pool
        .set_cache_key(
            format!("{guild_uuid}_channels"),
            channels_ordered.clone(),
            1800,
        )
        .await?;

    Ok((StatusCode::OK, Json(channels_ordered)).into_response())
}

pub async fn create(
    State(app_state): State<&'static AppState>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(channel_info): Json<ChannelInfo>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::ManageChannel)
        .await?;

    let channel = Channel::new(
        &mut conn,
        &app_state.cache_pool,
        guild_uuid,
        channel_info.name.clone(),
        channel_info.description.clone(),
    )
    .await?;

    Ok((StatusCode::OK, Json(channel)))
}

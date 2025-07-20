use std::sync::Arc;

use ::uuid::Uuid;
use axum::{
    extract::{Path, State}, http::StatusCode, response::IntoResponse, Extension, Json
};
use serde::Deserialize;

use crate::{
    api::v1::auth::CurrentUser, error::Error, objects::{Channel, Member, Permissions}, utils::{global_checks, order_by_is_above}, AppState
};

#[derive(Deserialize)]
pub struct ChannelInfo {
    name: String,
    description: Option<String>,
}

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    Member::check_membership(&mut app_state.pool.get().await?, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = app_state
        .get_cache_key(format!("{guild_uuid}_channels"))
        .await
    {
        return Ok((StatusCode::OK, Json(cache_hit)).into_response());
    }

    let channels = Channel::fetch_all(&app_state.pool, guild_uuid).await?;

    let channels_ordered = order_by_is_above(channels).await?;

    app_state
        .set_cache_key(
            format!("{guild_uuid}_channels"),
            channels_ordered.clone(),
            1800,
        )
        .await?;

    Ok((StatusCode::OK, Json(channels_ordered)).into_response())
}

pub async fn create(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(channel_info): Json<ChannelInfo>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let member = Member::check_membership(&mut app_state.pool.get().await?, uuid, guild_uuid).await?;

    member
        .check_permission(&app_state, Permissions::ManageChannel)
        .await?;

    let channel = Channel::new(
        &app_state,
        guild_uuid,
        channel_info.name.clone(),
        channel_info.description.clone(),
    )
    .await?;

    Ok((StatusCode::OK, Json(channel)))
}

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{Guild, Invite, Member},
    utils::global_checks,
};

pub async fn get(
    State(app_state): State<&'static AppState>,
    Path(invite_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let invite = Invite::fetch_one(&mut conn, invite_id).await?;

    let guild = Guild::fetch_one(&mut conn, invite.guild_uuid).await?;

    Ok((StatusCode::OK, Json(guild)))
}

pub async fn join(
    State(app_state): State<&'static AppState>,
    Path(invite_id): Path<String>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let invite = Invite::fetch_one(&mut conn, invite_id).await?;

    let guild = Guild::fetch_one(&mut conn, invite.guild_uuid).await?;

    Member::new(&mut conn, &app_state.cache_pool, uuid, guild.uuid).await?;

    Ok((StatusCode::OK, Json(guild)))
}

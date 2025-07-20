use std::sync::Arc;

use axum::{
    extract::{Path, State}, http::StatusCode, response::IntoResponse, Extension, Json
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
    State(app_state): State<Arc<AppState>>,
    Path(invite_id): Path<String>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let invite = Invite::fetch_one(&mut conn, invite_id).await?;

    let guild = Guild::fetch_one(&mut conn, invite.guild_uuid).await?;

    Ok((StatusCode::OK, Json(guild)))
}

pub async fn join(
    State(app_state): State<Arc<AppState>>,
    Path(invite_id): Path<String>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    let invite = Invite::fetch_one(&mut conn, invite_id).await?;

    let guild = Guild::fetch_one(&mut conn, invite.guild_uuid).await?;

    Member::new(&app_state, uuid, guild.uuid).await?;

    Ok((StatusCode::OK, Json(guild)))
}

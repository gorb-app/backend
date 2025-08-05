use std::sync::Arc;

use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    api::v1::auth::CurrentUser, error::Error, objects::{AuditLog, AuditLogId, Guild, Member, Permissions}, utils::global_checks, AppState
};

#[derive(Deserialize)]
pub struct InviteRequest {
    custom_id: Option<String>,
}

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let invites = guild.get_invites(&mut conn).await?;

    Ok((StatusCode::OK, Json(invites)))
}

pub async fn create(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(invite_request): Json<InviteRequest>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::CreateInvite)
        .await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let invite = guild
        .create_invite(&mut conn, uuid, invite_request.custom_id.clone())
        .await?;

    AuditLog::new(guild_uuid, AuditLogId::InviteCreate as i16, member.uuid, None, None, None, None, Some(invite.id.clone()), None, None).await.push(&mut conn).await?;

    Ok((StatusCode::OK, Json(invite)))
}

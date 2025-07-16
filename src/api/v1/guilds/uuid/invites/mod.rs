use std::sync::Arc;

use axum::{
    Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Guild, Member, Permissions},
    utils::global_checks,
};

#[derive(Deserialize)]
pub struct InviteRequest {
    custom_id: Option<String>,
}

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    global_checks(&app_state, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let invites = guild.get_invites(&mut conn).await?;

    Ok((StatusCode::OK, Json(invites)))
}

pub async fn create(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(invite_request): Json<InviteRequest>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    global_checks(&app_state, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&app_state, Permissions::CreateInvite)
        .await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let invite = guild
        .create_invite(&mut conn, uuid, invite_request.custom_id.clone())
        .await?;

    Ok((StatusCode::OK, Json(invite)))
}

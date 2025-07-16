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

use crate::{
    AppState,
    api::v1::auth::check_access_token,
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
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    global_checks(&app_state, uuid).await?;

    let invite = Invite::fetch_one(&mut conn, invite_id).await?;

    let guild = Guild::fetch_one(&mut conn, invite.guild_uuid).await?;

    Member::new(&app_state, uuid, guild.uuid).await?;

    Ok((StatusCode::OK, Json(guild)))
}

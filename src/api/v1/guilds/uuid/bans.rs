use std::sync::Arc;

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
    objects::{GuildBan, Member, Permissions},
    utils::global_checks,
};

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let caller = Member::check_membership(&mut conn, uuid, guild_uuid).await?;
    caller
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::BanMember)
        .await?;

    let all_guild_bans = GuildBan::fetch_all(&mut conn, guild_uuid).await?;

    Ok((StatusCode::OK, Json(all_guild_bans)))
}

pub async fn unban(
    State(app_state): State<Arc<AppState>>,
    Path((guild_uuid, user_uuid)): Path<(Uuid, Uuid)>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let caller = Member::check_membership(&mut conn, uuid, guild_uuid).await?;
    caller
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::BanMember)
        .await?;

    let ban = GuildBan::fetch_one(&mut conn, guild_uuid, user_uuid).await?;

    ban.unban(&mut conn).await?;

    Ok(StatusCode::OK)
}

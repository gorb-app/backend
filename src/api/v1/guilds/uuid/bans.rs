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
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;
    member
        .check_permission(&app_state, Permissions::BanMember)
        .await?;

    let all_guild_bans = GuildBan::fetch_all(&mut conn, guild_uuid).await?;

    Ok((StatusCode::OK, Json(all_guild_bans)))
}

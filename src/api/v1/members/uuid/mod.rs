//! `/api/v1/members/{uuid}` Member specific endpoints

pub mod ban;

use std::sync::Arc;

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{Me, Member, Permissions},
    utils::global_checks,
};
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use uuid::Uuid;

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(member_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let member =
        Member::fetch_one_with_uuid(&mut conn, &app_state.cache_pool, Some(&me), member_uuid)
            .await?;
    Member::check_membership(&mut conn, uuid, member.guild_uuid).await?;

    Ok((StatusCode::OK, Json(member)))
}

pub async fn delete(
    State(app_state): State<Arc<AppState>>,
    Path(member_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let member =
        Member::fetch_one_with_uuid(&mut conn, &app_state.cache_pool, Some(&me), member_uuid)
            .await?;

    let caller = Member::check_membership(&mut conn, uuid, member.guild_uuid).await?;

    caller
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::KickMember)
        .await?;

    member.delete(&mut conn).await?;

    Ok(StatusCode::OK)
}

//! `/api/v1/members/{uuid}` Member specific endpoints

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
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    let me = Me::get(&mut conn, uuid).await?;

    let member = Member::fetch_one_with_member(&app_state, &me, member_uuid).await?;
    Member::check_membership(&mut conn, uuid, member.guild_uuid).await?;

    Ok((StatusCode::OK, Json(member)))
}

pub async fn delete(
    State(app_state): State<Arc<AppState>>,
    Path(member_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    let me = Me::get(&mut conn, uuid).await?;

    let member = Member::fetch_one_with_member(&app_state, &me, member_uuid).await?;

    let deleter = Member::check_membership(&mut conn, uuid, member.guild_uuid).await?;

    deleter
        .check_permission(&app_state, Permissions::KickMember)
        .await?;

    member.delete(&mut conn).await?;

    Ok(StatusCode::OK)
}

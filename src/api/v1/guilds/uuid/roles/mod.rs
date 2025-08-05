use std::sync::Arc;

use ::uuid::Uuid;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    api::v1::auth::CurrentUser, error::Error, objects::{AuditLog, AuditLogId, Member, Permissions, Role}, utils::{global_checks, order_by_is_above, CacheFns}, AppState
};

pub mod uuid;

#[derive(Deserialize)]
pub struct RoleInfo {
    name: String,
}

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = app_state
        .cache_pool
        .get_cache_key::<Vec<Role>>(format!("{guild_uuid}_roles"))
        .await
    {
        return Ok((StatusCode::OK, Json(cache_hit)).into_response());
    }

    let roles = Role::fetch_all(&mut conn, guild_uuid).await?;

    let roles_ordered = order_by_is_above(roles).await?;

    app_state
        .cache_pool
        .set_cache_key(format!("{guild_uuid}_roles"), roles_ordered.clone(), 1800)
        .await?;

    Ok((StatusCode::OK, Json(roles_ordered)).into_response())
}

pub async fn create(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(role_info): Json<RoleInfo>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::ManageRole)
        .await?;

    // TODO: roles permission
    let role = Role::new(&mut conn, guild_uuid, role_info.name.clone()).await?;

    AuditLog::new(guild_uuid, AuditLogId::RoleCreate as i16, member.uuid, None, None, None, Some(role.uuid), Some(role_info.name.clone()) , None, None).await.push(&mut conn).await?;

    Ok((StatusCode::OK, Json(role)).into_response())
}

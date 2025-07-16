use std::sync::Arc;

use ::uuid::Uuid;
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

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Member, Permissions, Role},
    utils::{global_checks, order_by_is_above},
};

pub mod uuid;

#[derive(Deserialize)]
pub struct RoleInfo {
    name: String,
}

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = app_state.get_cache_key(format!("{guild_uuid}_roles")).await {
        return Ok((StatusCode::OK, Json(cache_hit)).into_response());
    }

    let roles = Role::fetch_all(&mut conn, guild_uuid).await?;

    let roles_ordered = order_by_is_above(roles).await?;

    app_state
        .set_cache_key(format!("{guild_uuid}_roles"), roles_ordered.clone(), 1800)
        .await?;

    Ok((StatusCode::OK, Json(roles_ordered)).into_response())
}

pub async fn create(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(role_info): Json<RoleInfo>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    global_checks(&app_state, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&app_state, Permissions::ManageRole)
        .await?;

    let role = Role::new(&mut conn, guild_uuid, role_info.name.clone()).await?;

    Ok((StatusCode::OK, Json(role)).into_response())
}

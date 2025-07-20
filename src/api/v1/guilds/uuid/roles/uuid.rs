use std::sync::Arc;

use ::uuid::Uuid;
use axum::{
    extract::{Path, State}, http::StatusCode, response::IntoResponse, Extension, Json
};

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{Member, Role},
    utils::global_checks,
};

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path((guild_uuid, role_uuid)): Path<(Uuid, Uuid)>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    if let Ok(cache_hit) = app_state.get_cache_key(format!("{role_uuid}")).await {
        return Ok((StatusCode::OK, Json(cache_hit)).into_response());
    }

    let role = Role::fetch_one(&mut conn, role_uuid).await?;

    app_state
        .set_cache_key(format!("{role_uuid}"), role.clone(), 60)
        .await?;

    Ok((StatusCode::OK, Json(role)).into_response())
}

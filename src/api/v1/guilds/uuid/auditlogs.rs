use std::sync::Arc;

use ::uuid::Uuid;
use axum::{
    Extension, Json,
    extract::{Path, Query, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{AuditLog, Member, PaginationRequest, Permissions},
    utils::global_checks,
};

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Query(pagination): Query<PaginationRequest>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let caller = Member::check_membership(&mut conn, uuid, guild_uuid).await?;
    caller
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::ManageGuild)
        .await?;

    let logs = AuditLog::fetch_page(&mut conn, guild_uuid, pagination).await?;

    Ok((StatusCode::OK, Json(logs)))
}

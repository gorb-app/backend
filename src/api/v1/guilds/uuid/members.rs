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
    objects::{Me, Member, PaginationRequest},
    utils::global_checks,
};

pub async fn get(
    State(app_state): State<&'static AppState>,
    Path(guild_uuid): Path<Uuid>,
    Query(pagination): Query<PaginationRequest>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let members = Member::fetch_page(
        &mut conn,
        &app_state.cache_pool,
        &me,
        guild_uuid,
        pagination,
    )
    .await?;

    Ok((StatusCode::OK, Json(members)))
}

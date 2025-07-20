use std::sync::Arc;

use ::uuid::Uuid;
use axum::{
    Extension, Json,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{Me, Member},
    utils::global_checks,
};

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let members = Member::fetch_all(&app_state, &me, guild_uuid).await?;

    Ok((StatusCode::OK, Json(members)))
}

use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use uuid::Uuid;

use crate::{
    AppState, api::v1::auth::CurrentUser, error::Error, objects::Me, utils::global_checks,
};

pub async fn delete(
    State(app_state): State<Arc<AppState>>,
    Path(friend_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    me.remove_friend(&mut conn, friend_uuid).await?;

    Ok(StatusCode::OK)
}

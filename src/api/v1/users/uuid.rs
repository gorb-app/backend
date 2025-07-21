//! `/api/v1/users/{uuid}` Specific user endpoints

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
    objects::{Me, User},
    utils::global_checks,
};

/// `GET /api/v1/users/{uuid}` Returns user with the given UUID
///
/// requires auth: yes
///
/// requires relation: yes
///
/// ### Response Example
/// ```
/// json!({
///         "uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "username": "user1",
///         "display_name": "Nullable Name",
///         "avatar": "https://nullable-url.com/path/to/image.png"
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(user_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let user =
        User::fetch_one_with_friendship(&mut conn, &app_state.cache_pool, &me, user_uuid).await?;

    Ok((StatusCode::OK, Json(user)))
}

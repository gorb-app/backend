use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use serde::Deserialize;
use ::uuid::Uuid;

pub mod uuid;

use crate::{
    api::v1::auth::CurrentUser, error::Error, objects::Me, utils::{global_checks, user_uuid_from_username}, AppState
};

/// Returns a list of users that are your friends
pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let me = Me::get(&mut app_state.pool.get().await?, uuid).await?;

    let friends = me.get_friends(&app_state).await?;

    Ok((StatusCode::OK, Json(friends)))
}

#[derive(Deserialize)]
pub struct UserReq {
    username: String,
}

/// `POST /api/v1/me/friends` Send friend request
///
/// requires auth? yes
///
/// ### Request Example:
/// ```
/// json!({
///     "uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
///
/// ### Responses
/// 200 Success
///
/// 404 Not Found
///
/// 400 Bad Request (usually means users are already friends)
///
pub async fn post(
    State(app_state): State<Arc<AppState>>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(user_request): Json<UserReq>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    let me = Me::get(&mut conn, uuid).await?;

    let target_uuid = user_uuid_from_username(&mut conn, &user_request.username).await?;
    me.add_friend(&mut conn, target_uuid).await?;

    Ok(StatusCode::OK)
}

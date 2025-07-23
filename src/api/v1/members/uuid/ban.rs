use std::sync::Arc;

use axum::{
    Extension,
    extract::{Json, Path, State},
    http::StatusCode,
    response::IntoResponse,
};
use serde::Deserialize;

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{Member, Permissions},
    utils::global_checks,
};

use uuid::Uuid;

#[derive(Deserialize)]
pub struct RequstBody {
    reason: String,
}

pub async fn post(
    State(app_state): State<Arc<AppState>>,
    Path(member_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(payload): Json<RequstBody>,
) -> Result<impl IntoResponse, Error> {
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;

    let member = Member::fetch_one_with_member(&app_state, None, member_uuid).await?;

    let caller = Member::check_membership(&mut conn, uuid, member.guild_uuid).await?;

    caller
        .check_permission(&app_state, Permissions::BanMember)
        .await?;

    member.ban(&mut conn, &payload.reason).await?;

    Ok(StatusCode::OK)
}

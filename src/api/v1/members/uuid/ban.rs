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
    State(app_state): State<&'static AppState>,
    Path(member_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(payload): Json<RequstBody>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let member =
        Member::fetch_one_with_uuid(&mut conn, &app_state.cache_pool, None, member_uuid).await?;

    let caller = Member::check_membership(&mut conn, uuid, member.guild_uuid).await?;

    caller
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::BanMember)
        .await?;

    member.ban(&mut conn, &payload.reason).await?;

    Ok(StatusCode::OK)
}

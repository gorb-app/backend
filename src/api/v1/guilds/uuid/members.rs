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

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Me, Member},
    utils::global_checks,
};

pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    global_checks(&app_state, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    let members = Member::fetch_all(&app_state, &me, guild_uuid).await?;

    Ok((StatusCode::OK, Json(members)))
}

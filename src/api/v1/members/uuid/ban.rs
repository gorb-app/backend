use std::sync::Arc;

use axum::{
    Extension,
    extract::{Path, State, Json},
    http::StatusCode,
    response::IntoResponse,
};
use diesel::{insert_into, RunQueryDsl};
use serde::Deserialize;

use crate::{
    api::v1::auth::CurrentUser, error::Error, objects::{Me, Member, Permissions}, schema::guild_bans::{self, dsl}, utils::global_checks, AppState
};

use uuid::Uuid;

#[derive(Deserialize)]
pub struct RequstBody {
    reason: String
}


pub async fn post(
    State(app_state): State<Arc<AppState>>,
    Path(member_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    Json(payload): Json<RequstBody>,
) -> Result<impl IntoResponse, Error>{
    global_checks(&app_state, uuid).await?;

    let mut conn = app_state.pool.get().await?;
    
    let member = Member::fetch_one_with_member(&app_state, None, member_uuid).await?;

    if member.is_owner {
        return Err(Error::Forbidden("Not allowed".to_string()));
    }
    
    let baner = Member::check_membership(&mut conn, uuid, member.guild_uuid).await?;
    baner.check_permission(&app_state, Permissions::ManageMember).await?;

    member.ban(&mut conn, &payload.reason).await?;


    Ok(StatusCode::OK)
}

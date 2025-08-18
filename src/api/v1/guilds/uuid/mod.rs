//! `/api/v1/guilds/{uuid}` Specific server endpoints

use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{Multipart, Path, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use bytes::Bytes;
use uuid::Uuid;

mod audit_logs;
mod bans;
mod channels;
mod invites;
mod members;
mod roles;

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{Guild, Member, Permissions},
    utils::global_checks,
};

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        // Servers
        .route("/", get(get_guild))
        .route("/", patch(edit))
        // Channels
        .route("/channels", get(channels::get))
        .route("/channels", post(channels::create))
        // Roles
        .route("/roles", get(roles::get))
        .route("/roles", post(roles::create))
        .route("/roles/{role_uuid}", get(roles::uuid::get))
        // Invites
        .route("/invites", get(invites::get))
        .route("/invites", post(invites::create))
        // Members
        .route("/members", get(members::get))
        // Bans
        .route("/bans", get(bans::get))
        .route("/bans/{uuid}", delete(bans::unban))
        // Audit Logs
        .route("/audit-logs", get(audit_logs::get))
}

/// `GET /api/v1/guilds/{uuid}` DESCRIPTION
///
/// requires auth: yes
///
/// ### Response Example
/// ```
/// json!({
///         "uuid": "5ba61ec7-5f97-43e1-89a5-d4693c155612",
///         "name": "My first server!",
///         "description": "This is a cool and nullable description!",
///         "icon": "https://nullable-url/path/to/icon.png",
///         "owner_uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "roles": [
///             {
///                 "uuid": "be0e4da4-cf73-4f45-98f8-bb1c73d1ab8b",
///                 "guild_uuid": "5ba61ec7-5f97-43e1-89a5-d4693c155612",
///                 "name": "Cool people",
///                 "color": 15650773,
///                 "is_above": c7432f1c-f4ad-4ad3-8216-51388b6abb5b,
///                 "permissions": 0
///             }
///             {
///                 "uuid": "c7432f1c-f4ad-4ad3-8216-51388b6abb5b",
///                 "guild_uuid": "5ba61ec7-5f97-43e1-89a5-d4693c155612",
///                 "name": "Equally cool people",
///                 "color": 16777215,
///                 "is_above": null,
///                 "permissions": 0
///             }
///         ],
///         "member_count": 20
/// });
/// ```
pub async fn get_guild(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    let guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    Ok((StatusCode::OK, Json(guild)))
}

/// `PATCH /api/v1/guilds/{uuid}` change guild settings
///
/// requires auth: yes
pub async fn edit(
    State(app_state): State<Arc<AppState>>,
    Path(guild_uuid): Path<Uuid>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    global_checks(&mut conn, &app_state.config, uuid).await?;

    let member = Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    member
        .check_permission(&mut conn, &app_state.cache_pool, Permissions::ManageGuild)
        .await?;

    let mut guild = Guild::fetch_one(&mut conn, guild_uuid).await?;

    let mut icon: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field
            .name()
            .ok_or(Error::BadRequest("Field has no name".to_string()))?;

        if name == "icon" {
            icon = Some(field.bytes().await?);
        }
    }

    if let Some(icon) = icon {
        guild.set_icon(&mut conn, &app_state, icon).await?;
    }

    Ok(StatusCode::OK)
}

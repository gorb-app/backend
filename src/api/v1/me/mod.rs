use std::sync::Arc;

use axum::{
    Extension, Json, Router,
    extract::{DefaultBodyLimit, Multipart, State},
    http::StatusCode,
    response::IntoResponse,
    routing::{delete, get, patch, post},
};
use bytes::Bytes;
use serde::Deserialize;
use uuid::Uuid;

use crate::{
    AppState, api::v1::auth::CurrentUser, error::Error, objects::Me, utils::global_checks,
};

mod friends;
mod guilds;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(get_me))
        .route(
            "/",
            patch(update).layer(DefaultBodyLimit::max(
                100 * 1024 * 1024, /* limit is in bytes */
            )),
        )
        .route("/guilds", get(guilds::get))
        .route("/friends", get(friends::get))
        .route("/friends", post(friends::post))
        .route("/friends/{uuid}", delete(friends::uuid::delete))
}

pub async fn get_me(
    State(app_state): State<Arc<AppState>>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let me = Me::get(&mut app_state.pool.get().await?, uuid).await?;

    Ok((StatusCode::OK, Json(me)))
}

#[derive(Default, Debug, Deserialize, Clone)]
struct NewInfo {
    username: Option<String>,
    display_name: Option<String>,
    email: Option<String>,
    pronouns: Option<String>,
    about: Option<String>,
    online_status: Option<i16>,
}

pub async fn update(
    State(app_state): State<Arc<AppState>>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
    mut multipart: Multipart,
) -> Result<impl IntoResponse, Error> {
    let mut json_raw: Option<NewInfo> = None;
    let mut avatar: Option<Bytes> = None;

    while let Some(field) = multipart.next_field().await.unwrap() {
        let name = field
            .name()
            .ok_or(Error::BadRequest("Field has no name".to_string()))?;

        if name == "avatar" {
            avatar = Some(field.bytes().await?);
        } else if name == "json" {
            json_raw = Some(serde_json::from_str(&field.text().await?)?)
        }
    }

    let json = json_raw.unwrap_or_default();

    let mut conn = app_state.pool.get().await?;

    if avatar.is_some() || json.username.is_some() || json.display_name.is_some() {
        global_checks(&mut conn, &app_state.config, uuid).await?;
    }

    let mut me = Me::get(&mut conn, uuid).await?;

    if let Some(avatar) = avatar {
        me.set_avatar(&mut conn, &app_state, avatar).await?;
    }

    if let Some(username) = &json.username {
        me.set_username(&mut conn, &app_state.cache_pool, username.clone())
            .await?;
    }

    if let Some(display_name) = &json.display_name {
        me.set_display_name(&mut conn, &app_state.cache_pool, display_name.clone())
            .await?;
    }

    if let Some(email) = &json.email {
        me.set_email(&mut conn, &app_state.cache_pool, email.clone())
            .await?;
    }

    if let Some(pronouns) = &json.pronouns {
        me.set_pronouns(&mut conn, &app_state.cache_pool, pronouns.clone())
            .await?;
    }

    if let Some(about) = &json.about {
        me.set_about(&mut conn, &app_state.cache_pool, about.clone())
            .await?;
    }

    if let Some(online_status) = &json.online_status {
        me.set_online_status(&mut conn, &app_state.cache_pool, *online_status)
            .await?;
    }

    Ok(StatusCode::OK)
}

//! `/api/v1/guilds` Guild related endpoints

use std::sync::Arc;

use axum::{
    Json, Router,
    extract::State,
    http::StatusCode,
    response::IntoResponse,
    routing::{get, post},
};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use serde::Deserialize;

mod uuid;

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    objects::{Guild, StartAmountQuery},
    utils::global_checks,
};

#[derive(Deserialize)]
pub struct GuildInfo {
    name: String,
}

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", post(new))
        .route("/", get(get_guilds))
        .nest("/{uuid}", uuid::router())
}

/// `POST /api/v1/guilds` Creates a new guild
///
/// requires auth: yes
///
/// ### Request Example
/// ```
/// json!({
///     "name": "My new server!"
/// });
/// ```
///
/// ### Response Example
/// ```
/// json!({
///     "uuid": "383d2afa-082f-4dd3-9050-ca6ed91487b6",
///     "name": "My new server!",
///     "description": null,
///     "icon": null,
///     "owner_uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///     "roles": [],
///     "member_count": 1
/// });
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
pub async fn new(
    State(app_state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(guild_info): Json<GuildInfo>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    let guild = Guild::new(&mut conn, guild_info.name.clone(), uuid).await?;

    Ok((StatusCode::OK, Json(guild)))
}

/// `GET /api/v1/servers` Fetches all guilds
///
/// requires auth: yes
///
/// requires admin: yes
///
/// ### Response Example
/// ```
/// json!([
///     {
///         "uuid": "383d2afa-082f-4dd3-9050-ca6ed91487b6",
///         "name": "My new server!",
///         "description": null,
///         "icon": null,
///         "owner_uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "roles": [],
///         "member_count": 1
///     },
///     {
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
///     }
/// ]);
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
pub async fn get_guilds(
    State(app_state): State<Arc<AppState>>,
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
    Json(request_query): Json<StartAmountQuery>,
) -> Result<impl IntoResponse, Error> {
    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    let uuid = check_access_token(auth.token(), &mut app_state.pool.get().await?).await?;

    global_checks(&app_state, uuid).await?;

    let guilds = Guild::fetch_amount(&app_state.pool, start, amount).await?;

    Ok((StatusCode::OK, Json(guilds)))
}

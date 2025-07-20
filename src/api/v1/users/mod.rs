//! `/api/v1/users` Contains endpoints related to all users

use std::sync::Arc;

use ::uuid::Uuid;
use axum::{
    Extension, Json, Router,
    extract::{Query, State},
    http::StatusCode,
    response::IntoResponse,
    routing::get,
};

use crate::{
    AppState,
    api::v1::auth::CurrentUser,
    error::Error,
    objects::{StartAmountQuery, User},
    utils::global_checks,
};

mod uuid;

pub fn router() -> Router<Arc<AppState>> {
    Router::new()
        .route("/", get(users))
        .route("/{uuid}", get(uuid::get))
}

/// `GET /api/v1/users` Returns all users on this instance
///
/// requires auth: yes
///
/// requires admin: yes
///
/// ### Response Example
/// ```
/// json!([
///     {
///         "uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "username": "user1",
///         "display_name": "Nullable Name",
///         "avatar": "https://nullable-url.com/path/to/image.png"
///     },
///     {
///         "uuid": "d48a3317-7b4d-443f-a250-ea9ab2bb8661",
///         "username": "user2",
///         "display_name": "John User 2",
///         "avatar": "https://also-supports-jpg.com/path/to/image.jpg"
///     },
///     {
///         "uuid": "12c4b3f8-a25b-4b9b-8136-b275c855ed4a",
///         "username": "user3",
///         "display_name": null,
///         "avatar": null
///     }
/// ]);
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
pub async fn users(
    State(app_state): State<Arc<AppState>>,
    Query(request_query): Query<StartAmountQuery>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    if amount > 100 {
        return Ok(StatusCode::BAD_REQUEST.into_response());
    }

    global_checks(&app_state, uuid).await?;

    let users = User::fetch_amount(&mut app_state.pool.get().await?, start, amount).await?;

    Ok((StatusCode::OK, Json(users)).into_response())
}

//! `/api/v1/auth/devices` Returns list of logged in devices

use std::sync::Arc;

use axum::{extract::State, http::StatusCode, response::IntoResponse, Extension, Json};
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use uuid::Uuid;

use crate::{
    api::v1::auth::CurrentUser, error::Error, schema::refresh_tokens::{self, dsl}, AppState
};

#[derive(Serialize, Selectable, Queryable)]
#[diesel(table_name = refresh_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct Device {
    device_name: String,
    created_at: i64,
}

/// `GET /api/v1/auth/devices` Returns list of logged in devices
///
/// requires auth: no
///
/// ### Response Example
/// ```
/// json!([
///     {
///         "device_name": "My Device!"
///         "created_at": "1752418856"
///     }
///     
/// ]);
/// ```
pub async fn get(
    State(app_state): State<Arc<AppState>>,
    Extension(CurrentUser(uuid)): Extension<CurrentUser<Uuid>>,
) -> Result<impl IntoResponse, Error> {
    let devices: Vec<Device> = dsl::refresh_tokens
        .filter(dsl::uuid.eq(uuid))
        .select(Device::as_select())
        .get_results(&mut app_state.pool.get().await?)
        .await?;

    Ok((StatusCode::OK, Json(devices)))
}

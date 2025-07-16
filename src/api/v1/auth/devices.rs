//! `/api/v1/auth/devices` Returns list of logged in devices

use std::sync::Arc;

use axum::{Json, extract::State, http::StatusCode, response::IntoResponse};
use axum_extra::{
    TypedHeader,
    headers::{Authorization, authorization::Bearer},
};
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::Serialize;

use crate::{
    AppState,
    api::v1::auth::check_access_token,
    error::Error,
    schema::refresh_tokens::{self, dsl},
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
    TypedHeader(auth): TypedHeader<Authorization<Bearer>>,
) -> Result<impl IntoResponse, Error> {
    let mut conn = app_state.pool.get().await?;

    let uuid = check_access_token(auth.token(), &mut conn).await?;

    let devices: Vec<Device> = dsl::refresh_tokens
        .filter(dsl::uuid.eq(uuid))
        .select(Device::as_select())
        .get_results(&mut conn)
        .await?;

    Ok((StatusCode::OK, Json(devices)))
}

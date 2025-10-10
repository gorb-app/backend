//! `/api/v1/stats` Returns stats about the server

use std::time::SystemTime;

use axum::Json;
use axum::extract::State;
use axum::http::StatusCode;
use axum::response::IntoResponse;
use diesel::QueryDsl;
use diesel_async::RunQueryDsl;
use serde::Serialize;

use crate::AppState;
use crate::error::Error;
use crate::schema::users::dsl::{users, uuid};

const VERSION: Option<&'static str> = option_env!("CARGO_PKG_VERSION");
const GIT_SHORT_HASH: &str = env!("GIT_SHORT_HASH");

#[derive(Serialize)]
struct Response {
    accounts: i64,
    uptime: u64,
    version: String,
    registration_enabled: bool,
    email_verification_required: bool,
    build_number: String,
}

/// `GET /api/v1/stats` Returns stats about the server
///
/// requires auth: no
///
/// ### Response Example
/// ```
/// json!({
///     "accounts": 3,
///     "uptime": 50000,
///     "version": "0.1.0",
///     "registration_enabled": true,
///     "email_verification_required": true,
///     "build_number": "39d01bb"
/// });
/// ```
pub async fn res(State(app_state): State<&'static AppState>) -> Result<impl IntoResponse, Error> {
    let accounts: i64 = users
        .select(uuid)
        .count()
        .get_result(&mut app_state.pool.get().await?)
        .await?;

    let response = Response {
        // TODO: Get number of accounts from db
        accounts,
        uptime: SystemTime::now()
            .duration_since(app_state.start_time)
            .expect("Seriously why dont you have time??")
            .as_secs(),
        version: String::from(VERSION.unwrap_or("UNKNOWN")),
        registration_enabled: app_state.config.instance.registration,
        email_verification_required: app_state.config.instance.require_email_verification,
        // TODO: Get build number from git hash or remove this from the spec
        build_number: String::from(GIT_SHORT_HASH),
    };

    Ok((StatusCode::OK, Json(response)))
}

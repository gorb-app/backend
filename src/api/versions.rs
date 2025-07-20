//! `/api/v1/versions` Returns info about api versions
use axum::{Json, http::StatusCode, response::IntoResponse};
use serde::Serialize;

#[derive(Serialize)]
struct Response {
    unstable_features: UnstableFeatures,
    versions: Vec<String>,
}

#[derive(Serialize)]
struct UnstableFeatures;

/// `GET /api/versions` Returns info about api versions.
///
/// requires auth: no
///
/// ### Response Example
/// ```
/// json!({
///     "unstable_features": {},
///     "versions": [
///         "1"
///     ]
/// });
/// ```
pub async fn versions() -> impl IntoResponse {
    let response = Response {
        unstable_features: UnstableFeatures,
        // TODO: Find a way to dynamically update this possibly?
        versions: vec![String::from("1")],
    };

    (StatusCode::OK, Json(response))
}

use actix_web::{Error, HttpResponse, error, post, web};
use futures::StreamExt;
use log::error;
use serde::{Deserialize, Serialize};
use uuid::Uuid;

use crate::{Data, api::v1::auth::check_access_token};

#[derive(Deserialize)]
struct AuthenticationRequest {
    access_token: String,
}

#[derive(Serialize)]
struct Response {
    uuid: String,
    username: String,
    display_name: String,
}

const MAX_SIZE: usize = 262_144;

#[post("/user/{uuid}")]
pub async fn res(
    mut payload: web::Payload,
    path: web::Path<(String,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    let request = path.into_inner().0;

    let authentication_request = serde_json::from_slice::<AuthenticationRequest>(&body)?;

    let authorized = check_access_token(authentication_request.access_token, &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let mut uuid = authorized.unwrap();

    if request != "me" {
        let requested_uuid = Uuid::parse_str(&request);

        if requested_uuid.is_err() {
            return Ok(HttpResponse::BadRequest().json(r#"{ "error": "UUID is invalid!" }"#));
        }

        uuid = requested_uuid.unwrap()
    }

    let row = sqlx::query_as(&format!(
        "SELECT username, display_name FROM users WHERE uuid = '{}'",
        uuid
    ))
    .fetch_one(&data.pool)
    .await;

    if let Err(error) = row {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let (username, display_name): (String, Option<String>) = row.unwrap();

    Ok(HttpResponse::Ok().json(Response {
        uuid: uuid.to_string(),
        username,
        display_name: display_name.unwrap_or_default(),
    }))
}

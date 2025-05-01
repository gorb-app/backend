use actix_web::{error, post, web, Error, HttpResponse};
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::{api::v1::auth::check_access_token, Data};

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
pub async fn res(mut payload: web::Payload, path: web::Path<(String,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
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

    if authorized.is_err() {
        return Ok(authorized.unwrap_err())
    }

    let uuid = authorized.unwrap();

    if request == "me" {
        let row = sqlx::query_as(&format!("SELECT username, display_name FROM users WHERE uuid = '{}'", uuid))
            .fetch_one(&data.pool)
            .await
            .unwrap();

        let (username, display_name): (String, Option<String>) = row;

        return Ok(HttpResponse::Ok().json(Response { uuid: uuid.to_string(), username, display_name: display_name.unwrap_or_default() }))
    } else {
        println!("{}", request);
        if let Ok(row) = sqlx::query_as(&format!("SELECT CAST(uuid as VARCHAR), username, display_name FROM users WHERE uuid = '{}'", request))
            .fetch_one(&data.pool)
            .await {
            let (uuid, username, display_name): (String, String, Option<String>) = row;

            return Ok(HttpResponse::Ok().json(Response { uuid, username, display_name: display_name.unwrap_or_default() }))
        }

        Ok(HttpResponse::NotFound().finish())
    }
}

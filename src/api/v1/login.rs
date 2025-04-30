use actix_web::{error, post, web, Error, HttpResponse};
use regex::Regex;
use serde::{Deserialize, Serialize};
use futures::StreamExt;

use crate::Data;

#[derive(Deserialize)]
struct LoginInformation {
    username: String,
    password: String,
    device_name: String,
}

#[derive(Serialize)]
struct Response {
    access_token: String,
    expires_in: u64,
    refresh_token: String,
}

const MAX_SIZE: usize = 262_144;

#[post("/login")]
pub async fn res(mut payload: web::Payload, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let mut body = web::BytesMut::new();
    while let Some(chunk) = payload.next().await {
        let chunk = chunk?;
        // limit max size of in-memory payload
        if (body.len() + chunk.len()) > MAX_SIZE {
            return Err(error::ErrorBadRequest("overflow"));
        }
        body.extend_from_slice(&chunk);
    }

    let login_information = serde_json::from_slice::<LoginInformation>(&body)?;

    let email_regex = Regex::new(r"[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+(?:\.[-A-Za-z0-9!#$%&'*+/=?^_`{|}~]+)*@(?:[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?\.)+[A-Za-z0-9](?:[-A-Za-z0-9]*[A-Za-z0-9])?").unwrap();

    // FIXME: This regex doesnt seem to be working
    let username_regex = Regex::new(r"[a-zA-Z0-9.-_]").unwrap();

    if email_regex.is_match(&login_information.username) {
        if let Ok(password) = sqlx::query_scalar("SELECT password FROM users WHERE email = $1").bind(login_information.username).fetch_one(&data.pool).await {
            return Ok(login(login_information.password, password))
        }

        return Ok(HttpResponse::Unauthorized().finish())
    } else if username_regex.is_match(&login_information.username) {
        if let Ok(password) = sqlx::query_scalar("SELECT password FROM users WHERE username = $1").bind(login_information.username).fetch_one(&data.pool).await {
            return Ok(login(login_information.password, password))
        }

        return Ok(HttpResponse::Unauthorized().finish())
    }

    Ok(HttpResponse::Unauthorized().finish())
}

fn login(request_password: String, database_password: String) -> HttpResponse {
    if request_password == database_password {
        return HttpResponse::Ok().json(Response {
            access_token: "bogus".to_string(),
            expires_in: 0,
            refresh_token: "bogus".to_string(),
        })
    }
    
    HttpResponse::Unauthorized().finish()
}

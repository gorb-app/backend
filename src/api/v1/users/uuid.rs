use actix_web::{Error, HttpRequest, HttpResponse, get, web};
use log::error;
use serde::Serialize;
use tokio::sync::Mutex;
use uuid::Uuid;

use crate::{Data, api::v1::auth::check_access_token, utils::get_auth_header};

#[derive(Serialize, Clone)]
struct Response {
    uuid: String,
    username: String,
    display_name: String,
}

#[get("/{uuid}")]
pub async fn res(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let uuid = path.into_inner().0;

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let cache_result = data.get_cache_key(uuid.to_string()).await;

    if let Ok(cache_hit) = cache_result {
        return Ok(HttpResponse::Ok().json(cache_hit))
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

    let user = Response {
        uuid: uuid.to_string(),
        username,
        display_name: display_name.unwrap_or_default(),
    };

    let cache_result = data.set_cache_key(uuid.to_string(), user.clone(), 1800).await;

    if let Err(error) = cache_result {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(user))
}

use actix_web::{Error, HttpRequest, HttpResponse, get, web};
use log::error;
use uuid::Uuid;

use crate::{api::v1::auth::check_access_token, structs::User, utils::get_auth_header, Data};

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
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let user_result = User::fetch_one(&data.pool, uuid).await;

    if let Err(error) = user_result {
        return Ok(error);
    }

    let user = user_result.unwrap();

    let cache_result = data
        .set_cache_key(uuid.to_string(), user.clone(), 1800)
        .await;

    if let Err(error) = cache_result {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(user))
}

use actix_web::{get, post, web, Error, HttpRequest, HttpResponse};
use serde::Deserialize;

use crate::{api::v1::auth::check_access_token, structs::Me, utils::get_auth_header, Data};

#[get("/me")]
pub async fn res(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let me = Me::get(&data.pool, uuid).await;

    if let Err(error) = me {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(me.unwrap()))
}

#[derive(Deserialize)]
struct NewInfo {
    username: Option<String>,
    display_name: Option<String>,
    password: Option<String>,
    email: Option<String>,
}

#[post("/me")]
pub async fn update(req: HttpRequest, new_info: web::Json<NewInfo>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    if let Err(error) = auth_header {
        return Ok(error);
    }

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let uuid = authorized.unwrap();

    let me_result = Me::get(&data.pool, uuid).await;

    if let Err(error) = me_result {
        return Ok(error);
    }

    let me = me_result;

    if let Some(username) = &new_info.username {
        todo!();
    }

    if let Some(display_name) = &new_info.display_name {
        todo!();
    }

    if let Some(password) = &new_info.password {
        todo!();
    }

    if let Some(email) = &new_info.email {
        todo!();
    }

    Ok(HttpResponse::Ok().finish())
}

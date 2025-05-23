use actix_web::{get, patch, web, HttpRequest, HttpResponse};
use actix_multipart::form::{json::Json as MpJson, tempfile::TempFile, MultipartForm};
use serde::Deserialize;

use crate::{error::Error, structs::Me, api::v1::auth::check_access_token, utils::get_auth_header, Data};

#[get("/me")]
pub async fn res(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let me = Me::get(&mut conn, uuid).await?;

    Ok(HttpResponse::Ok().json(me))
}

#[derive(Debug, Deserialize)]
struct NewInfo {
    username: Option<String>,
    display_name: Option<String>,
    password: Option<String>,
    email: Option<String>,
}

#[derive(Debug, MultipartForm)]
struct UploadForm {
    #[multipart(limit = "100MB")]
    avatar: Option<TempFile>,
    json: Option<MpJson<NewInfo>>,
}

#[patch("/me")]
pub async fn update(req: HttpRequest, MultipartForm(form): MultipartForm<UploadForm>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let mut me = Me::get(&mut conn, uuid).await?;

    if let Some(avatar) = form.avatar {
        let bytes = tokio::fs::read(avatar.file).await?;

        let byte_slice: &[u8] = &bytes;

        me.set_avatar(&data.bunny_cdn, &mut conn, data.config.bunny.cdn_url.clone(), byte_slice.into()).await?;
    }

    if let Some(new_info) = form.json {
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
    }

    Ok(HttpResponse::Ok().finish())
}

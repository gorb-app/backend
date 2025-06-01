use actix_multipart::form::{MultipartForm, json::Json as MpJson, tempfile::TempFile};
use actix_web::{HttpRequest, HttpResponse, Scope, get, patch, web};
use serde::Deserialize;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    structs::Me,
    utils::{get_auth_header, global_checks},
};

mod guilds;

pub fn web() -> Scope {
    web::scope("/me")
        .service(get)
        .service(update)
        .service(guilds::get)
}

#[get("")]
pub async fn get(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let me = Me::get(&mut conn, uuid).await?;

    Ok(HttpResponse::Ok().json(me))
}

#[derive(Debug, Deserialize, Clone)]
struct NewInfo {
    username: Option<String>,
    display_name: Option<String>,
    //password: Option<String>, will probably be handled through a reset password link
    email: Option<String>,
    pronouns: Option<String>,
    about: Option<String>,
}

#[derive(Debug, MultipartForm)]
struct UploadForm {
    #[multipart(limit = "100MB")]
    avatar: Option<TempFile>,
    json: MpJson<NewInfo>,
}

#[patch("")]
pub async fn update(
    req: HttpRequest,
    MultipartForm(form): MultipartForm<UploadForm>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    if form.avatar.is_some()
    || form.json.username.is_some()
    || form.json.display_name.is_some()
    {
        global_checks(&data, uuid).await?;
    }

    let mut me = Me::get(&mut conn, uuid).await?;

    if let Some(avatar) = form.avatar {
        let bytes = tokio::fs::read(avatar.file).await?;

        let byte_slice: &[u8] = &bytes;

        me.set_avatar(
            &data,
            data.config.bunny.cdn_url.clone(),
            byte_slice.into(),
        )
        .await?;
    }

    if let Some(username) = &form.json.username {
        me.set_username(&data, username.clone()).await?;
    }

    if let Some(display_name) = &form.json.display_name {
        me.set_display_name(&data, display_name.clone()).await?;
    }

    if let Some(email) = &form.json.email {
        me.set_email(&data, email.clone()).await?;
    }

    if let Some(pronouns) = &form.json.pronouns {
        me.set_pronouns(&data, pronouns.clone()).await?;
    }

    if let Some(about) = &form.json.about {
        me.set_about(&data, about.clone()).await?;
    }

    Ok(HttpResponse::Ok().finish())
}

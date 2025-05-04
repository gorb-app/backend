use actix_web::{cookie::{time::Duration, Cookie, SameSite}, http::header::HeaderMap, HttpResponse};

pub fn get_auth_header(headers: &HeaderMap) -> Result<&str, HttpResponse> {
    let auth_token = headers.get(actix_web::http::header::AUTHORIZATION);

    if let None = auth_token {
        return Err(HttpResponse::Unauthorized().finish());
    }

    let auth = auth_token.unwrap().to_str();

    if let Err(error) = auth {
        return Err(HttpResponse::Unauthorized().json(format!(r#" {{ "error": "{}" }} "#, error)));
    }

    let auth_value = auth.unwrap().split_whitespace().nth(1);

    if let None = auth_value {
        return Err(HttpResponse::BadRequest().finish());
    }

    Ok(auth_value.unwrap())
}

pub fn refresh_token_cookie(refresh_token: String) -> Cookie<'static> {
    Cookie::build("refresh_token", refresh_token)
        .http_only(true)
        .secure(true)
        .same_site(SameSite::None)
        .path("/api")
        .max_age(Duration::days(30))
        .finish()
} 

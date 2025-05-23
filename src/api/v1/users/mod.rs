use actix_web::{HttpRequest, HttpResponse, Scope, get, web};

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    structs::{StartAmountQuery, User},
    utils::get_auth_header,
};

mod me;
mod uuid;

pub fn web() -> Scope {
    web::scope("/users")
        .service(res)
        .service(me::res)
        .service(me::update)
        .service(uuid::res)
}

#[get("")]
pub async fn res(
    req: HttpRequest,
    request_query: web::Query<StartAmountQuery>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    if amount > 100 {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let mut conn = data.pool.get().await?;

    check_access_token(auth_header, &mut conn).await?;

    let users = User::fetch_amount(&mut conn, start, amount).await?;

    Ok(HttpResponse::Ok().json(users))
}

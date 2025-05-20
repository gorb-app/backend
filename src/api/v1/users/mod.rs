use crate::{api::v1::auth::check_access_token, structs::User, utils::get_auth_header, Data};
use actix_web::{Error, HttpRequest, HttpResponse, Scope, get, web};
use serde::Deserialize;

mod me;
mod uuid;

#[derive(Deserialize)]
struct RequestQuery {
    start: Option<i32>,
    amount: Option<i32>,
}

pub fn web() -> Scope {
    web::scope("/users")
        .service(res)
        .service(me::res)
        .service(uuid::res)
}

#[get("")]
pub async fn res(
    req: HttpRequest,
    request_query: web::Query<RequestQuery>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers);

    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    if amount > 100 {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let authorized = check_access_token(auth_header.unwrap(), &data.pool).await;

    if let Err(error) = authorized {
        return Ok(error);
    }

    let accounts = User::fetch_amount(&data.pool, start, amount).await;

    if let Err(error) = accounts {
        return Ok(error);
    }

    Ok(HttpResponse::Ok().json(accounts.unwrap()))
}

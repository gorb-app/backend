use crate::{api::v1::auth::check_access_token, structs::StartAmountQuery, utils::get_auth_header, Data};
use actix_web::{Error, HttpRequest, HttpResponse, Scope, get, web};
use log::error;
use serde::Serialize;
use sqlx::prelude::FromRow;

mod me;
mod uuid;

#[derive(Serialize, FromRow)]
struct Response {
    uuid: String,
    username: String,
    display_name: Option<String>,
    email: String,
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
    request_query: web::Query<StartAmountQuery>,
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

    let row = sqlx::query_as("SELECT CAST(uuid AS VARCHAR), username, display_name, email FROM users ORDER BY username LIMIT $1 OFFSET $2")
        .bind(amount)
        .bind(start)
        .fetch_all(&data.pool)
        .await;

    if let Err(error) = row {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    let accounts: Vec<Response> = row.unwrap();

    Ok(HttpResponse::Ok().json(accounts))
}

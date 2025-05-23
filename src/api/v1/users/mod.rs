use actix_web::{HttpRequest, HttpResponse, Scope, get, web};
use diesel::{prelude::Queryable, QueryDsl, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::Serialize;
use ::uuid::Uuid;

use crate::{error::Error,api::v1::auth::check_access_token, schema::users::{self, dsl}, structs::StartAmountQuery, utils::get_auth_header, Data};

mod me;
mod uuid;

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct Response {
    uuid: Uuid,
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

    let auth_header = get_auth_header(headers)?;

    let start = request_query.start.unwrap_or(0);

    let amount = request_query.amount.unwrap_or(10);

    if amount > 100 {
        return Ok(HttpResponse::BadRequest().finish());
    }

    let mut conn = data.pool.get().await?;

    check_access_token(auth_header, &mut conn).await?;

    let users: Vec<Response> = dsl::users
        .order_by(dsl::username)
        .offset(start)
        .limit(amount)
        .select(Response::as_select())
        .load(&mut conn)
        .await?;

    Ok(HttpResponse::Ok().json(users))
}

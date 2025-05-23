use actix_web::{HttpRequest, HttpResponse, get, web};
use diesel::{prelude::Queryable, ExpressionMethods, QueryDsl, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use log::error;
use serde::Serialize;
use uuid::Uuid;

use crate::{error::Error, api::v1::auth::check_access_token, schema::users::{self, dsl}, utils::get_auth_header, Data};

#[derive(Serialize, Queryable, Selectable)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct Response {
    uuid: Uuid,
    username: String,
    display_name: Option<String>,
}

#[get("/me")]
pub async fn res(req: HttpRequest, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let user: Result<Response, diesel::result::Error> = dsl::users
        .filter(dsl::uuid.eq(uuid))
        .select(Response::as_select())
        .get_result(&mut conn)
        .await;

    if let Err(error) = user {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish())
    }

    Ok(HttpResponse::Ok().json(user.unwrap()))
}

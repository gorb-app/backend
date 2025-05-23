use actix_web::{HttpRequest, HttpResponse, get, web};
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use log::error;
use serde::Serialize;
use uuid::Uuid;

use crate::{error::Error, api::v1::auth::check_access_token, schema::users::{self, dsl}, utils::get_auth_header, Data};

#[derive(Serialize, Queryable, Selectable, Clone)]
#[diesel(table_name = users)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct Response {
    uuid: Uuid,
    username: String,
    display_name: Option<String>,
}

#[get("/{uuid}")]
pub async fn res(
    req: HttpRequest,
    path: web::Path<(Uuid,)>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let uuid = path.into_inner().0;

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    check_access_token(auth_header, &mut conn).await?;

    let cache_result = data.get_cache_key(uuid.to_string()).await;

    if let Ok(cache_hit) = cache_result {
        return Ok(HttpResponse::Ok()
            .content_type("application/json")
            .body(cache_hit));
    }

    let user: Response = dsl::users
        .filter(dsl::uuid.eq(uuid))
        .select(Response::as_select())
        .get_result(&mut conn)
        .await?;

    let cache_result = data
        .set_cache_key(uuid.to_string(), user.clone(), 1800)
        .await;

    if let Err(error) = cache_result {
        error!("{}", error);
        return Ok(HttpResponse::InternalServerError().finish());
    }

    Ok(HttpResponse::Ok().json(user))
}

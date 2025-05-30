//! `/api/v1/users` Contains endpoints related to all users

use actix_web::{HttpRequest, HttpResponse, Scope, get, web};

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    structs::{StartAmountQuery, User},
    utils::{get_auth_header, global_checks},
};

mod uuid;

pub fn web() -> Scope {
    web::scope("/users").service(get).service(uuid::get)
}

/// `GET /api/v1/users` Returns all users on this instance
///
/// requires auth: yes
///
/// requires admin: yes
///
/// ### Response Example
/// ```
/// json!([
///     {
///         "uuid": "155d2291-fb23-46bd-a656-ae7c5d8218e6",
///         "username": "user1",
///         "display_name": "Nullable Name",
///         "avatar": "https://nullable-url.com/path/to/image.png"
///     },
///     {
///         "uuid": "d48a3317-7b4d-443f-a250-ea9ab2bb8661",
///         "username": "user2",
///         "display_name": "John User 2",
///         "avatar": "https://also-supports-jpg.com/path/to/image.jpg"
///     },
///     {
///         "uuid": "12c4b3f8-a25b-4b9b-8136-b275c855ed4a",
///         "username": "user3",
///         "display_name": null,
///         "avatar": null
///     }
/// ]);
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
#[get("")]
pub async fn get(
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

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let users = User::fetch_amount(&mut conn, start, amount).await?;

    Ok(HttpResponse::Ok().json(users))
}

//! `/api/v1/auth/devices` Returns list of logged in devices

use actix_web::{HttpRequest, HttpResponse, get, web};
use diesel::{ExpressionMethods, QueryDsl, Queryable, Selectable, SelectableHelper};
use diesel_async::RunQueryDsl;
use serde::Serialize;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    schema::refresh_tokens::{self, dsl},
    utils::get_auth_header,
};

#[derive(Serialize, Selectable, Queryable)]
#[diesel(table_name = refresh_tokens)]
#[diesel(check_for_backend(diesel::pg::Pg))]
struct Device {
    device_name: String,
    created_at: i64
}

/// `GET /api/v1/auth/devices` Returns list of logged in devices
///
/// requires auth: no
///
/// ### Response Example
/// ```
/// json!([
///     {
///         "device_name": "My Device!"
///         "created_at": "1752418856"
///     }
///     
/// ]);
/// ```
#[get("/devices")]
pub async fn get(
    req: HttpRequest,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    let devices: Vec<Device> = dsl::refresh_tokens
        .filter(dsl::uuid.eq(uuid))
        .select(Device::as_select())
        .get_results(&mut conn)
        .await?;

    Ok(HttpResponse::Ok().json(devices))
}

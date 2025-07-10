use actix_web::{HttpRequest, HttpResponse, delete, web};
use uuid::Uuid;

use crate::{
    Data,
    api::v1::auth::check_access_token,
    error::Error,
    objects::Me,
    utils::{get_auth_header, global_checks},
};

#[delete("/friends/{uuid}")]
pub async fn delete(req: HttpRequest, path: web::Path<(Uuid,)>, data: web::Data<Data>) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let auth_header = get_auth_header(headers)?;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    let me = Me::get(&mut conn, uuid).await?;

    me.remove_friend(&mut conn, path.0).await?;

    Ok(HttpResponse::Ok().finish())
}

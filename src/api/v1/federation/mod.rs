use actix_web::{post, web, HttpRequest, Scope};

use crate::{objects::Signature, Data};

mod pubkey;

pub fn web() -> Scope {
    web::scope("/federation")
        .service(pubkey::get)
        .service(post)
}

#[post("")]
pub async fn post(
    req: HttpRequest,
    channel_info: web::Json<>,
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let headers = req.headers();

    let signature = Signature::from_signature_header(headers)?;

    let guild_uuid = path.into_inner().0;

    let mut conn = data.pool.get().await?;

    let uuid = check_access_token(auth_header, &mut conn).await?;

    global_checks(&data, uuid).await?;

    Member::check_membership(&mut conn, uuid, guild_uuid).await?;

    // FIXME: Logic to check permissions, should probably be done in utils.rs

    let channel = Channel::new(
        data.clone(),
        guild_uuid,
        channel_info.name.clone(),
        channel_info.description.clone(),
    )
    .await?;

    Ok(HttpResponse::Ok().json(channel))
}

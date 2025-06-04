//! `/api/v1/users/{uuid}` Specific user endpoints

use actix_web::{HttpResponse, get, web};
use ed25519_dalek::pkcs8::{spki::der::pem::LineEnding, EncodePublicKey};

use crate::{
    Data,
    error::Error,
};

/// `GET /api/v1/users/{uuid}` Returns user with the given UUID
///
/// requires auth: yes
///
/// requires relation: yes
///
/// ### Response Example
/// ```
/// ""
/// ```
/// NOTE: UUIDs in this response are made using `uuidgen`, UUIDs made by the actual backend will be UUIDv7 and have extractable timestamps
#[get("/pubkey")]
pub async fn get(
    data: web::Data<Data>,
) -> Result<HttpResponse, Error> {
    let pubkey = data.signing_key.verifying_key().to_public_key_pem(LineEnding::LF)?;

    Ok(HttpResponse::Ok().body(pubkey))
}

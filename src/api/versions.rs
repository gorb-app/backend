use actix_web::{HttpResponse, Responder, get};
use serde::Serialize;

#[derive(Serialize)]
struct Response {
    unstable_features: UnstableFeatures,
    versions: Vec<String>,
}

#[derive(Serialize)]
struct UnstableFeatures;

#[get("/versions")]
pub async fn res() -> impl Responder {
    let response = Response {
        unstable_features: UnstableFeatures,
        // TODO: Find a way to dynamically update this possibly?
        versions: vec![String::from("1")],
    };

    HttpResponse::Ok().json(response)
}

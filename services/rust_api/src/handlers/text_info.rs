// Stub for missing text_api_info
use actix_web::{get, Responder, HttpResponse};

#[get("/text_info")]
pub async fn text_api_info() -> impl Responder {
    HttpResponse::Ok().body("Text API Info")
}

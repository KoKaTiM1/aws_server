// Stub for missing get_token
use actix_web::{get, Responder, HttpResponse};

#[get("/get_token")]
pub async fn get_token() -> impl Responder {
    HttpResponse::Ok().body("Token")
}

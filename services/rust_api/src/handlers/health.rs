use actix_web::{HttpResponse, Responder};
#[actix_web::get("/health")]
pub async fn health_check() -> impl Responder {
    HttpResponse::Ok().body("✅ Server is healthy")
}

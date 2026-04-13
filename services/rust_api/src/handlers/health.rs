use actix_web::{HttpResponse, Responder};

#[actix_web::get("/health")]
pub async fn health_check() -> impl Responder {
    // Minimal health check - just respond with 200 OK
    // No database checks, no dependencies - pure response
    println!("📡 Health check called");
    HttpResponse::Ok().finish()
}

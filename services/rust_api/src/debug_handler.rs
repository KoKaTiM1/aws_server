use actix_web::{HttpRequest, HttpResponse, Responder};
use tracing::info;

pub async fn debug_unmatched_route(req: HttpRequest) -> impl Responder {
    info!("[DEBUG] Unmatched route: {} {}", req.method(), req.path());
    HttpResponse::NotFound().body("Route not found (debug)")
}

use actix_web::{get, HttpResponse, Responder};
use serde_json::json;

#[get("/public_info")]
pub async fn public_api_info() -> impl Responder {
    HttpResponse::Ok().json(json!({
        "api_name": "Rust API Server",
        "version": "1.0.0",
        "status": "operational",
        "endpoints": [
            "GET /api/v1/health",
            "GET /api/v1/public_info", 
            "GET /api/v1/hardware",
            "POST /api/v1/hardware",
            "POST /api/v1/data/sensor"
        ]
    }))
}
use actix_web::{post, web, HttpResponse, Error};
use serde::{Deserialize, Serialize};
use sqlx::PgPool;

#[derive(Debug, Serialize, Deserialize)]
pub struct FeedbackEntry {
    pub device_id: String,
    pub timestamp: String,
    pub feedback: String,
}

#[post("/feedback")]
pub async fn post_feedback(
    pool: web::Data<PgPool>,
    entry: web::Json<FeedbackEntry>
) -> Result<HttpResponse, Error> {

    use chrono::NaiveDateTime;
    let timestamp = match NaiveDateTime::parse_from_str(&entry.timestamp, "%Y-%m-%dT%H:%M:%SZ") {
        Ok(dt) => dt,
        Err(_) => {
            return Err(actix_web::error::ErrorBadRequest("Invalid timestamp format. Use YYYY-MM-DD HH:MM:SS"));
        }
    };

    sqlx::query(
        "INSERT INTO feedback (device_id, timestamp, feedback) VALUES ($1, $2, $3)"
    )
    .bind(&entry.device_id)
    .bind(timestamp)
    .bind(&entry.feedback)
    .execute(pool.get_ref())
    .await
    .map_err(|e| {
        eprintln!("DB insert error: {e:?}");
        actix_web::error::ErrorInternalServerError("Failed to save feedback")
    })?;

    Ok(HttpResponse::Accepted().json(serde_json::json!({
        "status": "feedback received",
        "device_id": entry.device_id,
        "timestamp": entry.timestamp
    })))
}

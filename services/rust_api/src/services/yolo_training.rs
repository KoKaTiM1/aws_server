use chrono;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::fs::OpenOptions;
use std::io::Write;
use std::process::Command;
use std::sync::{Arc, Mutex};
use uuid::Uuid;

use actix_web::{get, post, web, HttpResponse, Responder};

#[derive(Clone)]
pub struct YoloTrainingService {
    pub log_file_path: String,
    pub python_script_path: String,
    pub job_status: Arc<Mutex<HashMap<String, String>>>,
}

impl YoloTrainingService {
    pub fn new(log_file_path: &str, python_script_path: &str) -> Self {
        Self {
            log_file_path: log_file_path.to_string(),
            python_script_path: python_script_path.to_string(),
            job_status: Arc::new(Mutex::new(HashMap::new())),
        }
    }

    pub fn train(
        &self,
        tag: &str,
        epochs: u32,
        batch_size: u32,
        detection_tagging: bool,
    ) -> Result<String, Box<dyn std::error::Error>> {
        println!("🚀 Training YOLO with tag: {tag}, epochs: {epochs}, batch: {batch_size}, detection_tagging: {detection_tagging}");

        let job_id = Uuid::new_v4().to_string();

        {
            let mut status_map = self.job_status.lock().unwrap();
            status_map.insert(job_id.clone(), "Running".to_string());
        }

        if let Some(parent) = std::path::Path::new(&self.log_file_path).parent() {
            std::fs::create_dir_all(parent)?;
        }

        // Build Python command
        let mut cmd = Command::new("python");
        cmd.arg(&self.python_script_path)
            .arg("--tag")
            .arg(tag)
            .arg("--job-id")
            .arg(&job_id)
            .arg("--device")
            .arg("cpu")
            .arg("--epochs")
            .arg(epochs.to_string())
            .arg("--batch")
            .arg(batch_size.to_string());

        if detection_tagging {
            cmd.arg("--detection-tagging");
        }

        let output = cmd.output()?;

        let mut log_file = OpenOptions::new()
            .create(true)
            .append(true)
            .open(&self.log_file_path)?;

        if output.status.success() {
            writeln!(
                log_file,
                "[{}] ✅ Training completed for tag: {tag}, job_id: {job_id}",
                chrono::Utc::now()
            )?;
            let mut status_map = self.job_status.lock().unwrap();
            status_map.insert(job_id.clone(), "Success".to_string());
            Ok(job_id)
        } else {
            let stderr = String::from_utf8_lossy(&output.stderr);
            writeln!(
                log_file,
                "[{}] ❌ Training failed for tag: {tag}, job_id: {job_id}\nError: {stderr}",
                chrono::Utc::now()
            )?;
            let mut status_map = self.job_status.lock().unwrap();
            status_map.insert(job_id.clone(), format!("Failed: {stderr}"));
            Err(format!("Training process failed: {stderr}").into())
        }
    }

    pub fn get_job_status(&self, job_id: &str) -> Option<String> {
        let status_map = self.job_status.lock().unwrap();
        status_map.get(job_id).cloned()
    }
}

#[derive(Debug, Deserialize)]
pub struct TrainingRequest {
    pub run_name: Option<String>,
    pub epochs: Option<u32>,
    pub batch_size: Option<u32>,
    pub detection_tagging: Option<bool>,
}

#[derive(Debug, Serialize)]
pub struct TrainingResponse {
    pub message: String,
    pub job_id: String,
}

#[derive(Debug, Serialize)]
pub struct ErrorResponse {
    pub error: String,
}

// === Actix Handlers ===

#[post("/trigger_training")]
pub async fn trigger_training_handler(
    service: web::Data<YoloTrainingService>,
    payload: web::Json<TrainingRequest>,
) -> impl Responder {
    let tag = payload
        .run_name
        .clone()
        .unwrap_or_else(|| "default".to_string());
    let epochs = payload.epochs.unwrap_or(128);
    let batch = payload.batch_size.unwrap_or(8);
    let detection = payload.detection_tagging.unwrap_or(false);

    match service.train(&tag, epochs, batch, detection) {
        Ok(job_id) => HttpResponse::Ok().json(TrainingResponse {
            message: "Training started".to_string(),
            job_id,
        }),
        Err(e) => HttpResponse::InternalServerError().json(ErrorResponse {
            error: e.to_string(),
        }),
    }
}

#[get("/job_status/{job_id}")]
pub async fn get_job_status_handler(
    service: web::Data<YoloTrainingService>,
    path: web::Path<String>,
) -> impl Responder {
    let job_id = path.into_inner();
    if let Some(status) = service.get_job_status(&job_id) {
        HttpResponse::Ok().body(status)
    } else {
        HttpResponse::NotFound().body("Job ID not found")
    }
}

#[get("/test")]
pub async fn test_handler() -> impl Responder {
    HttpResponse::Ok().body("✅ YOLO training service is alive.")
}

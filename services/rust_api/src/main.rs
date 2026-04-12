use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use actix_web::http::header;
use actix_cors::Cors;
use actix_files;
use futures_util::future;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use rust_api::handlers::{
    hardware::{register_hardware, sensor_data},
    health::health_check,
    public_info::public_api_info,
    text_info::text_api_info,
    token::get_token,
    ws::hardware_ws,
    yolo_api::create_yolo_scope,
    camera_stream::camera_websocket,
};
use rust_api::models::hardware::HardwarePayload;
use rust_api::routes::dashboard::*;
use rust_api::routes::{alerts::{post_alert, post_multipart_alert, post_device_health}, hardware::*};
mod debug_handler;
use debug_handler::debug_unmatched_route;
use rust_api::middleware::{
rate_limit::RateLimiter, security::SecurityHeadersMiddleware,
};
// use rust_api::services::heartbeat::{HeartbeatRegistry, spawn_watchdog}; // for non-MQTT watchdog 
use std::time::Duration;
use rust_api::services::mqtt_bus::{spawn_mqtt_bus, create_hivemq_config};
use rust_api::services::heartbeat::{spawn_watchdog_mqtt, HeartbeatRegistry};

use rust_api::{
    services::review_queue::ReviewQueueService, services::yolo_training::YoloTrainingService,
    MinioClient,
};
use rustls::{
    pki_types::{CertificateDer, PrivateKeyDer},
    ServerConfig,
};
use std::{env, fs::File, io::BufReader};
use tracing::{info, Level};

#[tokio::main]
async fn main() -> std::io::Result<()> {
    // Handler for /api/v1/nonexistent
    async fn api_v1_nonexistent_handler() -> actix_web::HttpResponse {
        actix_web::HttpResponse::Ok().body("This is a custom response for /api/v1/nonexistent")
    }

    // Handler for /nonexistent
    async fn root_nonexistent_handler() -> actix_web::HttpResponse {
        actix_web::HttpResponse::Ok().body("This is a custom response for /nonexistent")
    }
    
    // Wrapper for POST /hardware/ (trailing slash)
    async fn register_hardware_trailing(payload: actix_web::web::Json<HardwarePayload>) -> actix_web::HttpResponse {
        tracing::info!(
            "Hardware registration request (trailing slash) - ID: {}, Name: {}, Type: {:?}",
            payload.id,
            payload.name,
            payload.sensor_type
        );
        actix_web::HttpResponse::Ok().json(serde_json::json!({
            "status": "registered",
            "hardware_id": payload.id,
            "name": payload.name
        }))
    }

    // Wrapper for POST /hardware/test
    async fn hardware_test_handler(_payload: actix_web::web::Json<HardwarePayload>) -> actix_web::HttpResponse {
        actix_web::HttpResponse::Ok().body("Dummy hardware test endpoint")
    }

    // === Setup Crypto Provider ===
    let provider = rustls::crypto::ring::default_provider();
    provider.install_default().expect("failed to install crypto provider");
    
    // === 🔧 CHECK IF TLS IS ENABLED - THIS IS THE KEY FIX ===
    let tls_enabled = env::var("TLS_ENABLED")
        .unwrap_or_else(|_| "false".to_string())
        .to_lowercase() == "true";
    
    println!("🔒 TLS Enabled: {}", tls_enabled);
    
    // === TLS Certificate and Key Paths (only used if TLS is enabled) ===
    let tls_key_path = env::var("TLS_KEY_PATH").unwrap_or_else(|_| "certs/key.pem".to_string());
    let tls_cert_path = env::var("TLS_CERT_PATH").unwrap_or_else(|_| "certs/cert.pem".to_string());
    
    // === Setup Logging ===
    std::env::set_var("RUST_LOG", "actix_web=debug");
    tracing_subscriber::fmt()
        .with_max_level(Level::DEBUG)
        .init();

    // Print JWT secret for debug
    {
        use rust_api::auth::SECRET;
        println!("[main.rs] JWT secret value: {secret}, length: {len}", secret=String::from_utf8_lossy(SECRET), len=SECRET.len());
    }

    // Print rate limit value for debug
    match std::env::var("MAX_REQUESTS_PER_MINUTE") {
        Ok(val) => println!("[main.rs] MAX_REQUESTS_PER_MINUTE: {val}"),
        Err(_) => println!("[main.rs] MAX_REQUESTS_PER_MINUTE not set!"),
    }

    // === Get host and port ===
    let host = env::var("HOST").unwrap_or_else(|_| "0.0.0.0".to_string());
    let _port = env::var("PORT").unwrap_or_else(|_| "8080".to_string());

    // === AWS S3 Client Setup ===
    if let Ok(aws_key) = env::var("AWS_ACCESS_KEY_ID") {
        println!(
            "AWS_ACCESS_KEY_ID found: {}...",
            aws_key.chars().take(4).collect::<String>()
        );
    } else {
        println!("WARNING: AWS_ACCESS_KEY_ID environment variable not found!");
    }

    if let Ok(aws_secret) = env::var("AWS_SECRET_ACCESS_KEY") {
        println!("AWS_SECRET_ACCESS_KEY found: length={}", aws_secret.len());
    } else {
        println!("WARNING: AWS_SECRET_ACCESS_KEY environment variable not found!");
    }

    let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
    let s3_endpoint = env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://minio:9000".to_string());
    println!("S3 Endpoint: {s3_endpoint}");

    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
        .endpoint_url(&s3_endpoint)
        .force_path_style(true)
        .build();

    let s3_client = S3Client::from_conf(s3_config);

    // Test S3 connectivity at startup
    let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".to_string());
    match s3_client.list_objects_v2().bucket(&bucket).send().await {
        Ok(_) => println!("✅ S3/MinIO connection successful! Bucket '{bucket}' is accessible."),
        Err(e) => {
            println!("❌ S3/MinIO connection error: {e:?}");
            println!("Attempting to create bucket '{bucket}'...");
            match s3_client.create_bucket().bucket(&bucket).send().await {
                Ok(_) => println!("✅ Bucket '{bucket}' created successfully"),
                Err(e) => println!("❌ Failed to create bucket: {e:?}"),
            }
        }
    };

    // === YOLO Service ===
    let yolo_service = YoloTrainingService::new(
        "logs/training.log",
        r"serengeti/train_yolo.py",
    );

    // === Initialize Postgres Connection Pool ===
    use sqlx::postgres::PgPoolOptions;
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");
    let pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&database_url)
        .await
        .expect("Failed to create Postgres pool");

    // === Load persistent data from database on startup ===
    println!("📊 Loading persistent data from database...");
    rust_api::routes::dashboard::load_from_database(&pool).await;

    // === Initialize MinIO client and Review Queue Service ===
    let review_bucket = "review-queue";
    match s3_client.list_objects_v2().bucket(review_bucket).send().await {
    Ok(_) => println!("✅ Review queue bucket '{}' is accessible.", review_bucket),
    Err(e) => {
        println!("❌ Review queue bucket error: {e:?}");
        println!("Attempting to create review queue bucket '{}'...", review_bucket);
        match s3_client.create_bucket().bucket(review_bucket).send().await {
            Ok(_) => println!("✅ Review queue bucket '{}' created successfully", review_bucket),
            Err(e) => println!("❌ Failed to create review queue bucket: {e:?}"),
        }
    }
};
    let review_minio = MinioClient::new(&s3_endpoint, review_bucket)
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))?;
    let review_queue = ReviewQueueService::new(review_minio)
        .await
        .map_err(|e| std::io::Error::other(e.to_string()))?;

    // === 🔧 SSL Configuration (ONLY if TLS is enabled) ===
    let server_config = if tls_enabled {
        println!("🔒 Loading TLS certificates...");
        
        // Debug: List files in certs directory
        match std::fs::read_dir("certs") {
            Ok(entries) => {
                println!("Files in certs directory:");
                for entry in entries.flatten() {
                    println!("- {}", entry.path().display());
                }
            }
            Err(e) => println!("Failed to list certs directory: {e}"),
        }
        
        let key_file = File::open(&tls_key_path)
            .unwrap_or_else(|_| panic!("Failed to open TLS private key file: {tls_key_path}"));
        let cert_file = File::open(&tls_cert_path)
            .unwrap_or_else(|_| panic!("Failed to open TLS certificate file: {tls_cert_path}"));

        let mut key_reader = BufReader::new(key_file);
        let mut cert_reader = BufReader::new(cert_file);

        let key: PrivateKeyDer<'static> = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .next()
            .unwrap()
            .unwrap()
            .into();

        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .unwrap();

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .unwrap();
        
        println!("✅ TLS certificates loaded successfully");
        Some(config)
    } else {
        println!("🔓 TLS disabled - running HTTP only");
        None
    };

    // Log server startup information
    if tls_enabled {
        info!("🚀 Server running at https://{}:8443 and http://{}:8080", host, host);
    } else {
        info!("🚀 Server running at http://{}:8080 (TLS disabled)", host);
    }
    info!("Registering routes: '/', '/health', '/api/v1/...'");

    // // heartbeat registry + single watchdog 
    // let hb_registry = HeartbeatRegistry::new();
    // // Consider silent after 30s; check every 10s; 3s HTTP timeout
    // spawn_watchdog(
    //     hb_registry.clone(),
    //     Duration::from_secs(30),
    //     Duration::from_secs(10),
    //     Duration::from_secs(3),
    // );

    // ---- MQTT config - Remove the unused environment variables
    let hb_registry = HeartbeatRegistry::new();

    let mqtt_handle = spawn_mqtt_bus(
        create_hivemq_config(), // This will use your HiveMQ Cloud settings
        hb_registry.clone(),
    );

    // Start the MQTT watchdog
    spawn_watchdog_mqtt(hb_registry.clone(), mqtt_handle.clone(), Duration::from_secs(30), Duration::from_secs(10));

    // Start the offline detector - marks devices as offline if not seen in 2 hours
    // This allows for long periods between detections (quiet areas) while relying on heartbeat
    rust_api::services::heartbeat::spawn_offline_detector(
        pool.clone(),
        Duration::from_secs(7200),  // 2 hours (120 min) offline threshold
        Duration::from_secs(300),   // Check every 5 minutes
    );
    println!("🔍 Offline detector started (2 hour threshold, checking every 5 min)");

    // Initialize sample dashboard data for development
    rust_api::utils::sample_data::populate_sample_data();
    
    // Load persistent data from database into memory
    rust_api::routes::dashboard::load_from_database(&pool).await;

    let app_factory = move || {
        let trusted_origins: Vec<String> = std::env::var("TRUSTED_ORIGINS")
            .unwrap_or_else(|_| "https://eye-dar.com".to_string())
            .split(',')
            .map(|s| s.trim().to_string())
            .collect();
        let cors = Cors::default()
            .allowed_origin_fn(move |origin, _req_head| {
                trusted_origins.iter().any(|trusted| origin.as_bytes().starts_with(trusted.as_bytes()))
            })
            .allowed_methods(vec!["GET", "POST", "PUT", "DELETE"])
            .allowed_headers(vec![
                header::AUTHORIZATION,
                header::ACCEPT,
                header::CONTENT_TYPE,
            ])
            .supports_credentials()
            .max_age(3600);

        App::new()
            .wrap(Logger::default())
            .wrap(SecurityHeadersMiddleware)
            .wrap(RateLimiter::new(60))
            .wrap(cors)
            .app_data(web::Data::new(s3_client.clone()))
            .app_data(web::Data::new(yolo_service.clone()))
            .app_data(web::Data::new(review_queue.clone()))
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(hb_registry.clone()))
            .app_data(web::Data::new(mqtt_handle.clone())) // Add MQTT handle to app data
            .service(health_check)
            .service(
                actix_files::Files::new("/dashboard", "static/")
                    .index_file("dashboard.html")
                    .show_files_listing()
            )
            .service(
                actix_files::Files::new("/api/v1/photos", "./serengeti/esp_photos")
                    .show_files_listing()
            )
            .service(web::resource("/nonexistent").route(web::get().to(root_nonexistent_handler)))
            .service(
                web::resource("/api/v1/health").route(web::get().to(|| async { actix_web::HttpResponse::Ok().body("✅ API v1 health OK") }))
            )
            .service(
                web::scope("/api/v1")
                    .configure(|cfg| {
                        cfg.service(register_hardware);
                        cfg.service(rust_api::handlers::hardware::esp_heartbeat);
                        cfg.service(rust_api::handlers::hardware::recover_esp);
                        cfg.service(rust_api::handlers::hardware::get_esp_health);
                        cfg.service(public_api_info);
                        cfg.service(text_api_info);
                        cfg.service(sensor_data);
                        cfg.service(get_token);
                        cfg.service(post_alert);
                        cfg.service(post_multipart_alert);
                        cfg.service(post_device_health);
                        cfg.service(register_hardware_device);
                        cfg.service(sensor_data_device);
                        cfg.service(device_heartbeat);
                        cfg.service(get_dashboard);
                        cfg.service(get_dashboard_overview);
                        cfg.service(get_all_device_health);
                        cfg.service(get_device_health);
                        cfg.service(get_device_metrics_by_id);
                        cfg.service(get_device_activity);
                        cfg.service(get_device_photos);
                        cfg.service(get_device_detections);
                        cfg.service(download_detections_csv);
                        cfg.service(download_device_detections_bundle_zip);
                        cfg.service(download_device_photos_zip);
                        cfg.service(control_device);
                        cfg.service(get_alerts_endpoint);
                        cfg.service(acknowledge_alert);
                        cfg.service(filter_devices);
                        cfg.service(start_camera_stream);
                        cfg.service(stop_camera_stream);
                        cfg.service(get_camera_status);
                        cfg.service(restart_device);
                        cfg.service(rust_api::routes::feedback::post_feedback);
                        cfg.service(rust_api::routes::ping::receive_ping);
                        cfg.route("/ws/v1/sensor-stream", web::get().to(hardware_ws));
                        cfg.route("/ws/camera/{device_id}", web::get().to(camera_websocket));
                        cfg.service(web::scope("/yolo").configure(create_yolo_scope));
                        cfg.service(
                            web::resource("/hardware/").route(web::post().to(register_hardware_trailing))
                        );
                        cfg.service(
                            web::resource("/hardware/test").route(web::post().to(hardware_test_handler))
                        );
                        cfg.service(
                            web::resource("/nonexistent").route(web::get().to(api_v1_nonexistent_handler))
                        );
                        cfg.default_service(web::to(debug_unmatched_route));
                    })
            )
    };

    // === 🔧 START THE APPROPRIATE SERVER(S) BASED ON TLS CONFIGURATION ===
    if let Some(config) = server_config {
        // TLS enabled - start both HTTP and HTTPS servers
        let http_server = HttpServer::new(app_factory.clone())
            .bind(format!("{host}:8080"))?
            .run();

        let https_server = HttpServer::new(app_factory)
            .bind_rustls_0_23(format!("{host}:8443"), config)?
            .run();

        future::try_join(http_server, https_server).await.map(|_| ())
    } else {
        // TLS disabled - start only HTTP server
        HttpServer::new(app_factory)
            .bind(format!("{host}:8080"))?
            .run()
            .await
    }
}
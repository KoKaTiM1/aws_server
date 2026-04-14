use actix_web::middleware::Logger;
use actix_web::{web, App, HttpServer};
use actix_web::http::header;
use actix_cors::Cors;
// use actix_files;  // DISABLED - no local files to serve in AWS
use futures_util::future;
use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::Client as S3Client;
use aws_sdk_sqs::Client as SqsClient;
use aws_sdk_secretsmanager::Client as SecretsClient;
use rust_api::{S3BucketName, QueueUrlIngest};
use rust_api::handlers::{
    health::health_check,
};
use rust_api::routes::alerts::{post_alert, post_multipart_alert};
use rust_api::routes::dashboard::{
    get_dashboard, get_dashboard_overview, get_all_device_health,
    get_device_health, get_device_metrics_by_id, get_device_activity,
    get_alerts_endpoint, get_recent_alerts
};
use rust_api::middleware::{
    rate_limit::RateLimiter, security::SecurityHeadersMiddleware,
};
use std::time::Duration;
use rust_api::services::heartbeat::HeartbeatRegistry;

use rust_api::{
    // services::yolo_training::YoloTrainingService,  // DISABLED - local files not in AWS
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

    // Handler for /dashboard - serves embedded HTML
    async fn dashboard_handler() -> actix_web::HttpResponse {
        actix_web::HttpResponse::Ok()
            .content_type("text/html; charset=utf-8")
            .body(DASHBOARD_HTML)
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
    let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
        .region(region_provider)
        .load()
        .await;

    // Extract region for logging and dynamic configuration
    let region = shared_config
        .region()
        .map(|r| r.as_ref())
        .unwrap_or("us-east-1");
    println!("🌍 AWS Region: {}", region);

    // S3 Client (native AWS, no MinIO endpoint)
    let s3_client = S3Client::new(&shared_config);

    // SQS Client (same region)
    let sqs_client = SqsClient::new(&shared_config);

    // Secrets Manager Client (for TLS certs and secrets)
    let secrets_client = SecretsClient::new(&shared_config);

    // Redis Client (from REDIS_ENDPOINT env var)
    let redis_url = env::var("REDIS_ENDPOINT")
        .unwrap_or_else(|_| "redis://localhost:6379".to_string());
    println!("✅ Redis endpoint configured: {}", redis_url);

    // Validate S3 bucket access at startup
    let s3_bucket = env::var("S3_BUCKET").expect("S3_BUCKET must be set");
    println!("🔍 DEBUG main.rs: S3_BUCKET env var read = '{}'", s3_bucket);
    println!("🔍 DEBUG main.rs: S3_BUCKET length = {}", s3_bucket.len());

    match s3_client.list_objects_v2().bucket(&s3_bucket).max_keys(1).send().await {
        Ok(_) => println!("✅ AWS S3 connection successful! Bucket '{s3_bucket}' is accessible."),
        Err(e) => {
            eprintln!("⚠️  S3 bucket not accessible: {e:?}");
            eprintln!("Ensure bucket exists and IAM role has s3:GetObject, s3:PutObject permissions");
        }
    };

    // === YOLO Service ===
    // DISABLED FOR AWS - home server had local logs and model files
    // let yolo_service = YoloTrainingService::new(
    //     "logs/training.log",
    //     r"serengeti/train_yolo.py",
    // );
    println!("⚠️  YOLO service disabled (home server local files not available in AWS)");

    // DEBUG: Add a test endpoint that returns the S3_BUCKET value
    println!("📋 REGISTERING DEBUG ROUTE: /debug/s3bucket");

    // === Initialize Postgres Connection Pool ===
    use sqlx::postgres::PgPoolOptions;
    let database_url = std::env::var("DATABASE_URL").expect("DATABASE_URL must be set");

    // Retry logic for RDS connection (it may take a moment to be accessible)
    let pool = {
        let mut retries = 0;
        let max_retries = 30;
        let pool_result = loop {
            match PgPoolOptions::new()
                .max_connections(5)
                .connect(&database_url)
                .await
            {
                Ok(p) => {
                    println!("✅ PostgreSQL connection pool created successfully");
                    break Ok(p);
                }
                Err(e) => {
                    retries += 1;
                    if retries >= max_retries {
                        break Err(format!("Failed to connect to PostgreSQL after {} attempts: {}", max_retries, e));
                    }
                    eprintln!("⏳ PostgreSQL connection attempt {} failed: {}. Retrying in 1s...", retries, e);
                    tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                }
            }
        };
        pool_result.expect("Failed to create Postgres pool after retries")
    };

    // === Load persistent data from database on startup ===
    println!("📊 Skipping database load for now (temporarily disabled)");
    // rust_api::routes::dashboard::load_from_database(&pool).await;


    // === 🔧 SSL Configuration (ONLY if TLS is enabled) ===
    let server_config = if tls_enabled {
        println!("🔒 Loading TLS certificates from AWS Secrets Manager...");

        let cert_secret_name = env::var("TLS_CERT_SECRET_NAME")
            .unwrap_or_else(|_| "eyedar-prod-tls-cert".to_string());
        let key_secret_name = env::var("TLS_KEY_SECRET_NAME")
            .unwrap_or_else(|_| "eyedar-prod-tls-key".to_string());

        // Fetch certificate from Secrets Manager
        let cert_response = secrets_client
            .get_secret_value()
            .secret_id(&cert_secret_name)
            .send()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other,
                format!("Failed to fetch TLS cert from Secrets Manager: {}", e)))?;

        let key_response = secrets_client
            .get_secret_value()
            .secret_id(&key_secret_name)
            .send()
            .await
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::Other,
                format!("Failed to fetch TLS key from Secrets Manager: {}", e)))?;

        let cert_pem = cert_response.secret_string()
            .ok_or(std::io::Error::new(std::io::ErrorKind::NotFound, "No certificate data in secret"))?
            .to_string();
        let key_pem = key_response.secret_string()
            .ok_or(std::io::Error::new(std::io::ErrorKind::NotFound, "No key data in secret"))?
            .to_string();

        // Parse certificates from PEM strings
        let mut cert_reader = std::io::Cursor::new(cert_pem);
        let mut key_reader = std::io::Cursor::new(key_pem);

        let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
            .collect::<Result<Vec<_>, _>>()
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData,
                format!("Failed to parse certificates: {}", e)))?;

        let keys: Vec<PrivateKeyDer<'static>> = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
            .map(|k| k.map(|key| PrivateKeyDer::Pkcs8(key)))
            .collect::<Result<Vec<_>, std::io::Error>>()?;

        let key = keys.into_iter().next()
            .ok_or(std::io::Error::new(std::io::ErrorKind::NotFound, "No private key found"))?;

        let config = ServerConfig::builder()
            .with_no_client_auth()
            .with_single_cert(certs, key)
            .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

        println!("✅ TLS certificates loaded from Secrets Manager successfully");
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

    // ---- MQTT config - TEMPORARILY DISABLED FOR DEBUGGING
    let hb_registry = HeartbeatRegistry::new();

    println!("⚠️  MQTT/Offline detector subsystem disabled for this deployment");

    // Initialize sample dashboard data for development
    // DISABLED FOR DEBUGGING - this was loading potentially corrupted state
    // rust_api::utils::sample_data::populate_sample_data();
    println!("⚠️  Sample data population disabled");

    // Load persistent data from database into memory (DISABLED FOR NOW - blocking startup)
    // rust_api::routes::dashboard::load_from_database(&pool).await;

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
            .app_data(web::Data::new(sqs_client.clone()))
            .app_data({
                println!("🔍 DEBUG main.rs: About to register S3_BUCKET as app_data = '{}'", s3_bucket);
                let data = web::Data::new(S3BucketName(s3_bucket.clone()));
                println!("🔍 DEBUG main.rs: S3BucketName wrapped in web::Data = '{}'", data.0);
                data
            })
            .app_data(web::Data::new(QueueUrlIngest(env::var("QUEUE_URL_INGEST").unwrap_or_default())))
            // .app_data(web::Data::new(yolo_service.clone()))  // DISABLED - local files not in AWS
            .app_data(web::Data::new(pool.clone()))
            .app_data(web::Data::new(hb_registry.clone()))
            // .app_data(web::Data::new(mqtt_handle.clone()))
            .service(health_check)
            // Dashboard page - serve embedded HTML directly
            .service(
                web::resource("/dashboard").route(
                    web::get().to(|| async {
                        actix_web::HttpResponse::PermanentRedirect()
                            .insert_header((actix_web::http::header::LOCATION, "/dashboard/"))
                            .finish()
                    })
                )
            )
            .service(
                web::resource("/dashboard/").route(web::get().to(dashboard_handler))
            )
            .service(web::resource("/nonexistent").route(web::get().to(root_nonexistent_handler)))
            .service(
                web::resource("/api/v1/health").route(web::get().to(|| async { actix_web::HttpResponse::Ok().body("✅ API v1 health OK") }))
            )
            // TEMP DEBUG: Simple endpoint to check S3_BUCKET value - read env var directly
            .service(
                web::resource("/api/v1/debug/s3bucket").route(web::get().to(|| async {
                    let s3_bucket_from_env = std::env::var("S3_BUCKET").unwrap_or_else(|_| "NOT_SET".to_string());
                    actix_web::HttpResponse::Ok().json(serde_json::json!({
                        "s3_bucket_from_env": s3_bucket_from_env,
                        "from_env_is_empty": s3_bucket_from_env.is_empty(),
                    }))
                }))
            )
            .service(
                web::scope("/api/v1")
                    .configure(|cfg| {
                        // Alert endpoints
                        cfg.service(post_alert);
                        cfg.service(post_multipart_alert);

                        // Dashboard endpoints
                        cfg.service(get_dashboard);
                        cfg.service(get_dashboard_overview);
                        cfg.service(get_all_device_health);
                        cfg.service(get_device_health);
                        cfg.service(get_device_metrics_by_id);
                        cfg.service(get_device_activity);
                        cfg.service(get_alerts_endpoint);
                        cfg.service(get_recent_alerts);
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
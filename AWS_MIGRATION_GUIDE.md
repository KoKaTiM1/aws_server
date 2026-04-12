# AWS Setup Changes for main.rs

## SECTION 1: Dynamic Region + AWS Clients (Lines 128-160)

**BEFORE (MinIO only):**
```rust
let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
let s3_endpoint = env::var("S3_ENDPOINT").unwrap_or_else(|_| "http://minio:9000".to_string());
let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
    .region(region_provider)
    .load()
    .await;
let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
    .endpoint_url(&s3_endpoint)
    .force_path_style(true)
    .build();
let s3_client = S3Client::from_conf(s3_config);
```

**AFTER (AWS Native + Dynamic Region):**
```rust
let region_provider = RegionProviderChain::default_provider().or_else("us-east-1");
let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
    .region(region_provider)
    .load()
    .await;

// Extract region for logging
let region = shared_config
    .region()
    .map(|r| r.as_ref())
    .unwrap_or("us-east-1");
println!("🌍 AWS Region: {}", region);

// S3 Client (native AWS, no MinIO endpoint)
let s3_client = S3Client::new(&shared_config);

// SQS Client (same region)
let sqs_client = aws_sdk_sqs::Client::new(&shared_config);

// Redis Client (from env var: REDIS_ENDPOINT)
let redis_url = env::var("REDIS_ENDPOINT").expect("REDIS_ENDPOINT must be set");
let redis_client = redis::Client::open(redis_url)
    .expect("Failed to create Redis client");
println!("✅ Redis connected: {}", env::var("REDIS_ENDPOINT").unwrap_or_default());

// Secrets Manager Client (for TLS certs)
let secrets_client = aws_sdk_secretsmanager::Client::new(&shared_config);
```

## SECTION 2: S3 Bucket Validation (Lines 145-156)

**BEFORE:**
```rust
let bucket = env::var("S3_BUCKET").unwrap_or_else(|_| "uploads".to_string());
match s3_client.list_objects_v2().bucket(&bucket).send().await {
    Ok(_) => println!("✅ S3/MinIO connection successful! Bucket '{bucket}' is accessible."),
    Err(e) => println!("❌ S3/MinIO connection error: {e:?}"),
}
```

**AFTER:**
```rust
let s3_bucket = env::var("S3_BUCKET").expect("S3_BUCKET must be set");
match s3_client.list_objects_v2().bucket(&s3_bucket).send().await {
    Ok(_) => println!("✅ AWS S3 connection successful! Bucket '{s3_bucket}' is accessible."),
    Err(e) => {
        println!("⚠️  S3 bucket not accessible: {e:?}");
        println!("Ensure bucket exists and IAM role has s3:GetObject, s3:PutObject permissions");
    }
}
```

## SECTION 3: TLS from Secrets Manager (Lines 197-240)

**BEFORE (local files):**
```rust
let tls_enabled = env::var("TLS_ENABLED")
    .unwrap_or_else(|_| "false".to_string())
    .to_lowercase() == "true";

let tls_key_path = env::var("TLS_KEY_PATH").unwrap_or_else(|_| "certs/key.pem".to_string());
let tls_cert_path = env::var("TLS_CERT_PATH").unwrap_or_else(|_| "certs/cert.pem".to_string());

if tls_enabled {
    let key_file = File::open(&tls_key_path).unwrap();
    let cert_file = File::open(&tls_cert_path).unwrap();
    // ... load from files
}
```

**AFTER (AWS Secrets Manager):**
```rust
let tls_enabled = env::var("TLS_ENABLED")
    .unwrap_or_else(|_| "false".to_string())
    .to_lowercase() == "true";

let server_config = if tls_enabled {
    println!("🔒 Loading TLS certificates from AWS Secrets Manager...");

    let cert_secret_name = env::var("TLS_CERT_SECRET_NAME")
        .unwrap_or_else(|_| "eyedar-prod-tls-cert".to_string());
    let key_secret_name = env::var("TLS_KEY_SECRET_NAME")
        .unwrap_or_else(|_| "eyedar-prod-tls-key".to_string());

    // Fetch certificate and key from Secrets Manager
    let cert_response = secrets_client
        .get_secret_value()
        .secret_id(&cert_secret_name)
        .send()
        .await
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other,
                format!("Failed to fetch TLS cert from Secrets Manager: {}", e))
        })?;

    let key_response = secrets_client
        .get_secret_value()
        .secret_id(&key_secret_name)
        .send()
        .await
        .map_err(|e| {
            std::io::Error::new(std::io::ErrorKind::Other,
                format!("Failed to fetch TLS key from Secrets Manager: {}", e))
        })?;

    let cert_pem = cert_response.secret_string()
        .ok_or(std::io::Error::new(std::io::ErrorKind::NotFound, "No certificate data"))?;
    let key_pem = key_response.secret_string()
        .ok_or(std::io::Error::new(std::io::ErrorKind::NotFound, "No key data"))?;

    // Parse certificates
    let mut cert_reader = std::io::Cursor::new(cert_pem);
    let mut key_reader = std::io::Cursor::new(key_pem);

    let certs: Vec<CertificateDer<'static>> = rustls_pemfile::certs(&mut cert_reader)
        .collect::<Result<Vec<_>, _>>()
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    let keys: Vec<PrivateKeyDer<'static>> = rustls_pemfile::pkcs8_private_keys(&mut key_reader)
        .map(|k| k.map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string())))
        .collect::<Result<Vec<_>, _>>()?;

    let key = keys.into_iter().next()
        .ok_or(std::io::Error::new(std::io::ErrorKind::NotFound, "No private key found"))?;

    let config = ServerConfig::builder()
        .with_no_client_auth()
        .with_single_cert(certs, key)
        .map_err(|e| std::io::Error::new(std::io::ErrorKind::InvalidData, e.to_string()))?;

    println!("✅ TLS certificates loaded from Secrets Manager");
    Some(config)
} else {
    println!("🔓 TLS disabled - running HTTP only");
    None
};
```

## SECTION 4: Pass Clients to App State (Lines 350+)

Add to web::Data for app state:
```rust
.app_data(web::Data::new(s3_client))
.app_data(web::Data::new(sqs_client))
.app_data(web::Data::new(redis_client))
.app_data(web::Data::new(s3_bucket))
.app_data(web::Data::new(env::var("QUEUE_URL_INGEST").unwrap_or_default()))
```

## SECTION 5: Required Environment Variables

For AWS deployment, set these in Terraform (or ECS task definition):

```bash
# Database
DATABASE_URL=postgresql://user:pass@eyedar-db.xxxxx.rds.amazonaws.com:5432/eyedar

# Redis (ElastiCache)
REDIS_ENDPOINT=redis://eyedar-redis.xxxxx.cache.amazonaws.com:6379

# AWS S3
S3_BUCKET=eyedar-prod-images
AWS_REGION=us-east-1

# SQS Queues (from Terraform outputs)
QUEUE_URL_INGEST=https://sqs.us-east-1.amazonaws.com/123456/detection-created
QUEUE_URL_VERIFY=https://sqs.us-east-1.amazonaws.com/123456/verify-requested
QUEUE_URL_NOTIFY=https://sqs.us-east-1.amazonaws.com/123456/verified-animals

# TLS Certificates (in Secrets Manager)
TLS_ENABLED=true
TLS_CERT_SECRET_NAME=eyedar-prod-tls-cert
TLS_KEY_SECRET_NAME=eyedar-prod-tls-key

# Optional
RUST_LOG=actix_web=debug
MAX_REQUESTS_PER_MINUTE=1000
HOST=0.0.0.0
PORT=8080
```

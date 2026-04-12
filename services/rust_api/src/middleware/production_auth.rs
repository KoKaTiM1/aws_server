// Production-Grade Authentication Middleware
// src/middleware/production_auth.rs

use actix_web::dev::{Service, ServiceRequest, ServiceResponse, Transform};
use actix_web::error::{ErrorUnauthorized, ErrorForbidden, ErrorTooManyRequests};
use actix_web::http::header;
use actix_web::{Error, HttpMessage};
use futures_util::future::{ok, Ready};
use jsonwebtoken::{decode, Algorithm, DecodingKey, Validation};
use openssl::x509::X509;
use redis::AsyncCommands;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::env;
use std::fs;
use std::future::Future;
use std::pin::Pin;
use std::sync::Arc;
use std::task::{Context, Poll};
use std::time::{SystemTime, UNIX_EPOCH};
use uuid::Uuid;

#[derive(Debug, Serialize, Deserialize)]
pub struct ProductionClaims {
    pub sub: String,              // Hardware device ID
    pub device_serial: String,    // Device serial number
    pub device_type: String,      // Type of Eye-DAR device
    pub firmware_version: String, // Firmware version
    pub location_hash: String,    // Geofenced location hash
    pub capabilities: Vec<String>, // Device capabilities
    pub security_level: u8,       // Security clearance level (1-5)
    pub exp: usize,              // Expiration timestamp
    pub iat: usize,              // Issued at timestamp
    pub jti: String,             // Unique token identifier
    pub device_cert_hash: String, // Hash of device certificate
}

#[derive(Debug)]
pub struct DeviceInfo {
    pub device_id: String,
    pub serial_number: String,
    pub device_type: String,
    pub security_level: u8,
    pub capabilities: Vec<String>,
    pub last_seen: SystemTime,
    pub is_compromised: bool,
}

pub struct ProductionAuth {
    pub redis_client: Arc<redis::Client>,
    pub device_ca_cert: Arc<X509>,
    pub jwt_public_key: Arc<DecodingKey>,
    pub jwt_secret: Vec<u8>,
    pub rate_limits: HashMap<String, u32>,
}

impl ProductionAuth {
    pub fn new() -> Result<Self, Box<dyn std::error::Error>> {
        // Load device CA certificate
        let ca_path = env::var("DEVICE_CA_PATH")
            .unwrap_or_else(|_| "./certs/device_ca/ca.pem".to_string());
        let ca_pem = fs::read_to_string(ca_path)?;
        let device_ca_cert = Arc::new(X509::from_pem(ca_pem.as_bytes())?);

        // Load JWT public key for RSA verification
        let public_key_path = env::var("JWT_PUBLIC_KEY_PATH")
            .unwrap_or_else(|_| "./certs/jwt/public_key.pem".to_string());
        let public_key_pem = fs::read_to_string(public_key_path)?;
        let jwt_public_key = Arc::new(DecodingKey::from_rsa_pem(public_key_pem.as_bytes())?);

        // Load JWT secret for HS256
        let jwt_secret = if let Ok(secret_file) = env::var("JWT_SECRET_FILE") {
            match fs::read_to_string(&secret_file) {
                Ok(secret) => {
                    println!("[production_auth] Loaded JWT secret from file: {secret_file}");
                    println!("[production_auth] Secret value: '{secret}', length: {len}", secret=secret.trim(), len=secret.trim().len());
                    secret.trim().as_bytes().to_vec()
                }
                Err(e) => {
                    println!("[production_auth] Failed to read JWT secret file '{secret_file}': {e}");
                    b"dev_secret_key_change_me".to_vec()
                }
            }
        } else if let Ok(secret) = env::var("JWT_SECRET") {
            println!("[production_auth] Loaded JWT secret from env var JWT_SECRET");
            println!("[production_auth] Secret value: '{secret}', length: {len}", secret=secret.trim(), len=secret.trim().len());
            secret.trim().as_bytes().to_vec()
        } else {
            println!("[production_auth] Using default JWT secret");
            let secret = "dev_secret_key_change_me";
            println!("[production_auth] Secret value: '{secret}', length: {len}", secret=secret, len=secret.len());
            secret.as_bytes().to_vec()
        };

        // Redis connection for rate limiting and device tracking
        let redis_url = env::var("REDIS_URL")
            .unwrap_or_else(|_| "redis://127.0.0.1:6379".to_string());
        let redis_client = Arc::new(redis::Client::open(redis_url)?);

        // Configure rate limits
        let mut rate_limits = HashMap::new();
        rate_limits.insert("auth".to_string(), 
            env::var("RATE_LIMIT_AUTH").unwrap_or_else(|_| "10".to_string()).parse().unwrap_or(10));
        rate_limits.insert("api".to_string(), 
            env::var("RATE_LIMIT_API").unwrap_or_else(|_| "60".to_string()).parse().unwrap_or(60));
        rate_limits.insert("upload".to_string(), 
            env::var("RATE_LIMIT_UPLOAD").unwrap_or_else(|_| "5".to_string()).parse().unwrap_or(5));

        Ok(ProductionAuth {
            redis_client,
            device_ca_cert,
            jwt_public_key,
            jwt_secret,
            rate_limits,
        })
    }

    async fn validate_device_certificate(&self, cert_pem: &str) -> Result<DeviceInfo, Error> {
        let cert = X509::from_pem(cert_pem.as_bytes())
            .map_err(|_| ErrorUnauthorized("Invalid device certificate format"))?;

        // Verify certificate chain
        // TODO: Implement proper certificate validation against CA
        
        // Extract device information from certificate
        let subject = cert.subject_name();
        let device_id = subject.entries_by_nid(openssl::nid::Nid::COMMONNAME)
            .next()
            .and_then(|name| name.data().as_utf8().ok())
            .map(|s| s.to_string())
            .ok_or_else(|| ErrorUnauthorized("Device ID not found in certificate"))?;

        // TODO: Extract other device info from certificate extensions
        
        Ok(DeviceInfo {
            device_id,
            serial_number: "UNKNOWN".to_string(), // Extract from cert
            device_type: "eye-dar-v1".to_string(), // Extract from cert
            security_level: 3, // Extract from cert
            capabilities: vec!["image_capture".to_string(), "yolo_inference".to_string()],
            last_seen: SystemTime::now(),
            is_compromised: false, // Check against blacklist
        })
    }

    async fn check_rate_limit(&self, device_id: &str, endpoint_type: &str) -> Result<(), Error> {
        println!("[rate_limit] Connecting to Redis for device_id={device_id}, endpoint_type={endpoint_type}");
        let mut conn = match self.redis_client.get_async_connection().await {
            Ok(c) => c,
            Err(e) => {
                println!("[rate_limit] Redis connection error: {e:?}");
                return Err(ErrorTooManyRequests("Rate limiting service unavailable"));
            }
        };

        let limit = self.rate_limits.get(endpoint_type).unwrap_or(&60);
        let key = format!("rate_limit:{device_id}:{endpoint_type}");
        let current_time = SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_secs();
        let window_start = current_time - 60; // 1-minute window

        println!("[rate_limit] Checking key='{key}', window_start={window_start}, current_time={current_time}, limit={limit}");

        // Use Redis sorted set for sliding window rate limiting
        let count: i32 = match conn.zcount(&key, window_start, current_time).await {
            Ok(c) => c,
            Err(e) => {
                println!("[rate_limit] Redis zcount error: {e:?}");
                return Err(ErrorTooManyRequests("Rate limiting error"));
            }
        };

        println!("[rate_limit] Current count for key='{key}': {count}");

        if count >= *limit as i32 {
            println!("[rate_limit] Limit reached for key='{key}'. Flushing key and allowing current request.");
            if let Err(e) = conn.del::<_, usize>(&key).await {
                println!("[rate_limit] Redis del error: {e:?}");
                return Err(ErrorTooManyRequests("Rate limiting error"));
            }
            if let Err(e) = conn.zadd::<_, _, _, usize>(&key, Uuid::new_v4().to_string(), current_time).await {
                println!("[rate_limit] Redis zadd error after flush: {e:?}");
                return Err(ErrorTooManyRequests("Rate limiting error"));
            }
            if let Err(e) = conn.expire::<_, bool>(&key, 120).await {
                println!("[rate_limit] Redis expire error after flush: {e:?}");
                return Err(ErrorTooManyRequests("Rate limiting error"));
            }
            return Ok(());
        }

        // Add current request to the window
        if let Err(e) = conn.zadd::<_, _, _, usize>(&key, Uuid::new_v4().to_string(), current_time).await {
            println!("[rate_limit] Redis zadd error: {e:?}");
            return Err(ErrorTooManyRequests("Rate limiting error"));
        }
        // Set expiration for the key
        if let Err(e) = conn.expire::<_, bool>(&key, 120).await {
            println!("[rate_limit] Redis expire error: {e:?}");
            return Err(ErrorTooManyRequests("Rate limiting error"));
        }

        println!("[rate_limit] Request allowed for key='{key}'");
        Ok(())
    }

    async fn log_security_event(&self, event_type: &str, device_id: &str, details: &str) {
        // TODO: Implement security event logging
        tracing::warn!(
            target: "security_events",
            event_type = event_type,
            device_id = device_id,
            details = details,
            timestamp = ?SystemTime::now()
        );
    }
}

impl<S, B> Transform<S, ServiceRequest> for ProductionAuth
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type InitError = ();
    type Transform = ProductionAuthMiddleware<S>;
    type Future = Ready<Result<Self::Transform, Self::InitError>>;

    fn new_transform(&self, service: S) -> Self::Future {
        ok(ProductionAuthMiddleware {
            service: Arc::new(service),
            auth: Arc::new(self.clone()),
        })
    }
}

pub struct ProductionAuthMiddleware<S> {
    service: Arc<S>,
    auth: Arc<ProductionAuth>,
}

impl<S, B> Service<ServiceRequest> for ProductionAuthMiddleware<S>
where
    S: Service<ServiceRequest, Response = ServiceResponse<B>, Error = Error> + 'static,
    S::Future: 'static,
    B: 'static,
{
    type Response = ServiceResponse<B>;
    type Error = Error;
    type Future = Pin<Box<dyn Future<Output = Result<Self::Response, Self::Error>>>>;

    fn poll_ready(&self, cx: &mut Context<'_>) -> Poll<Result<(), Self::Error>> {
        self.service.poll_ready(cx)
    }

    fn call(&self, req: ServiceRequest) -> Self::Future {
        let service = Arc::clone(&self.service);
        let auth = Arc::clone(&self.auth);
        let path = req.path().to_string();
        let client_ip = req.connection_info().realip_remote_addr().unwrap_or("unknown").to_string();
        tracing::info!(target: "production_auth", "ProductionAuth middleware called for path: {} from IP: {}", path, client_ip);

        println!("[production_auth] Entered middleware for path: {path} from IP: {client_ip}");

        Box::pin(async move {
            // Skip authentication for health check, status endpoints, and root path
            if path == "/health" || path == "/metrics" || path == "/status" || path == "/status/api" || path == "/" {
                println!("[production_auth] Skipping auth for path: {path}");
                return service.call(req).await;
            }

            // Get client IP for logging and rate limiting
            let client_ip = req
                .connection_info()
                .realip_remote_addr()
                .unwrap_or("unknown")
                .to_string();

            // Extract device certificate from headers (if required)
            if env::var("REQUIRE_DEVICE_CERTIFICATES").unwrap_or_else(|_| "false".to_string()) == "true" {
                let device_cert_header = req.headers().get("X-Device-Certificate");
                if let Some(cert_header) = device_cert_header {
                    let cert_pem = cert_header.to_str()
                        .map_err(|_| ErrorUnauthorized("Invalid device certificate header"))?;
                     println!("[production_auth] Validating device certificate for IP: {client_ip}");
                    let _device_info = auth.validate_device_certificate(cert_pem).await?;
                    // TODO: Store device info for later use
                }
            }

            // Get JWT token from Authorization header
            println!("[production_auth] Middleware entry: path={}, IP={}", req.path(), client_ip);

            let auth_header = req.headers().get(header::AUTHORIZATION)
                .ok_or_else(|| {
                    tracing::error!(target: "auth_failures", "Missing Authorization header from IP: {} Path: {}", client_ip, req.path());
                    // Note: This closure cannot be async, so we will log after this block if needed
                    ErrorUnauthorized("No authorization token provided")
                });
            if let Err(ref _e) = auth_header {
                println!("[production_auth] Missing Authorization header for path: {} from IP: {}", req.path(), client_ip);
                auth.log_security_event("missing_auth", &client_ip, &format!("Path: {}", req.path())).await;
                println!("[production_auth] Returning Unauthorized due to missing Authorization header");
            }
            let auth_header = auth_header?;

            let auth_str = auth_header.to_str()
                .map_err(|_| {
                    tracing::error!(target: "auth_failures", "Invalid Authorization header format from IP: {} Path: {}", client_ip, req.path());
                    println!("[production_auth] Invalid Authorization header format for path: {} from IP: {}", req.path(), client_ip);
                    ErrorUnauthorized("Invalid authorization header")
                })?;

            if !auth_str.starts_with("Bearer ") {
                tracing::warn!(target: "auth_failures", "Invalid authorization scheme from IP: {} Path: {} Header: {}", client_ip, req.path(), auth_str);
                // Log the event before returning
                auth.log_security_event("invalid_scheme", &client_ip, &format!("Header: {} Path: {}", auth_str, req.path())).await;
                println!("[production_auth] Invalid authorization scheme for path: {} from IP: {}", req.path(), client_ip);
                return Err(ErrorUnauthorized("Invalid authorization scheme"));
            }

            let token = &auth_str[7..];

            println!("[production_auth] Received JWT token: {token}");

            // Validate JWT token with RSA public key
            let mut validation = Validation::new(Algorithm::HS256);
            validation.validate_exp = true;
            validation.validate_aud = false; // We don't use audience claims

            let token_data = decode::<ProductionClaims>(
                token,
                &DecodingKey::from_secret(&auth.jwt_secret),
                &validation,
            ).map_err(|e| {
                println!("[production_auth] JWT validation failed: {e:?}");
                // Note: This closure cannot be async, so we will log after this block if needed
                ErrorUnauthorized("Invalid token")
            });
            if let Err(ref e) = token_data {
                auth.log_security_event("invalid_token", &client_ip, &format!("Error: {e}")).await;
                println!("[production_auth] Returning Unauthorized due to invalid token for path: {} from IP: {}", req.path(), client_ip);
            }
            let token_data = token_data?;

            println!("[production_auth] JWT validation succeeded: {token_data:?}");

            let claims = token_data.claims;

            // Check rate limiting for this device
            let endpoint_type = if req.path().starts_with("/stream") {
                "upload"
            } else if req.path().starts_with("/auth") || req.path() == "/get_token" {
                "auth"
            } else {
                "api"
            };

            println!("[production_auth] Checking rate limit for device: {} endpoint_type: {}", claims.sub, endpoint_type);
            auth.check_rate_limit(&claims.sub, endpoint_type).await?;

            // Verify device is not compromised
            if claims.security_level < 2 {
                auth.log_security_event("low_security_device", &claims.sub, "Device security level too low").await;
                println!("[production_auth] Device security level too low for device: {}", claims.sub);
                return Err(ErrorForbidden("Device security level insufficient"));
            }

            // Log successful authentication
            tracing::info!(
                target: "auth_success",
                device_id = claims.sub,
                device_type = claims.device_type,
                path = req.path(),
                client_ip = client_ip
            );
            println!("[production_auth] Authentication succeeded for device: {} path: {}", claims.sub, req.path());

            // Add device information to request extensions
            req.extensions_mut().insert(claims);

            println!("[production_auth] Passing request to next service for path: {}", req.path());
            service.call(req).await
        })
    }
}

impl Clone for ProductionAuth {
    fn clone(&self) -> Self {
        ProductionAuth {
            redis_client: Arc::clone(&self.redis_client),
            device_ca_cert: Arc::clone(&self.device_ca_cert),
            jwt_public_key: Arc::clone(&self.jwt_public_key),
            jwt_secret: self.jwt_secret.clone(),
            rate_limits: self.rate_limits.clone(),
        }
    }
}

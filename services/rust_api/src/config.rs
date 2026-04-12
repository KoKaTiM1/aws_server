use std::env;
use std::time::Duration;

#[derive(Clone)]
pub struct ServerConfig {
    pub host: String,
    pub port: u16,
    pub jwt_secret: String,
    pub trusted_origins: Vec<String>,
    pub max_requests_per_minute: u32,
    pub ssl_cert_path: String,
    pub ssl_key_path: String,
    pub hardware_auth_timeout: Duration,
    pub max_failed_auth_attempts: u32,
    pub auth_lockout_duration: Duration,
}

#[derive(Clone)]
pub struct S3Config {
    pub access_key: String,
    pub secret_key: String,
    pub region: String,
    pub bucket: String,
}

impl ServerConfig {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        Ok(ServerConfig {
            host: env::var("SERVER_HOST").unwrap_or_else(|_| "0.0.0.0".to_string()),
            port: env::var("SERVER_PORT")
                .unwrap_or_else(|_| "8080".to_string())
                .parse()
                .map_err(|_| "Invalid SERVER_PORT")?,
            jwt_secret: env::var("JWT_SECRET")
                .map_err(|_| "JWT_SECRET must be set")?,
            trusted_origins: env::var("TRUSTED_ORIGINS")
                .unwrap_or_else(|_| "https://eye-dar.com".to_string())
                .split(',')
                .map(String::from)
                .collect(),
            max_requests_per_minute: env::var("MAX_REQUESTS_PER_MINUTE")
                .unwrap_or_else(|_| "60".to_string())
                .parse()
                .map_err(|_| "Invalid MAX_REQUESTS_PER_MINUTE")?,
            ssl_cert_path: env::var("SSL_CERT_PATH")
                .unwrap_or_else(|_| "./certs/cert.pem".to_string()),
            ssl_key_path: env::var("SSL_KEY_PATH")
                .unwrap_or_else(|_| "./certs/key.pem".to_string()),
            hardware_auth_timeout: Duration::from_secs(
                env::var("HARDWARE_AUTH_TIMEOUT")
                    .unwrap_or_else(|_| "3600".to_string())
                    .parse()
                    .map_err(|_| "Invalid HARDWARE_AUTH_TIMEOUT")?
            ),
            max_failed_auth_attempts: env::var("MAX_FAILED_AUTH_ATTEMPTS")
                .unwrap_or_else(|_| "5".to_string())
                .parse()
                .map_err(|_| "Invalid MAX_FAILED_AUTH_ATTEMPTS")?,
            auth_lockout_duration: Duration::from_secs(
                env::var("AUTH_LOCKOUT_DURATION")
                    .unwrap_or_else(|_| "900".to_string())
                    .parse()
                    .map_err(|_| "Invalid AUTH_LOCKOUT_DURATION")?
            ),
        })
    }
}

impl S3Config {
    pub fn from_env() -> Result<Self, String> {
        dotenvy::dotenv().ok();

        Ok(S3Config {
            access_key: env::var("AWS_ACCESS_KEY_ID")
                .map_err(|_| "AWS_ACCESS_KEY_ID must be set")?,
            secret_key: env::var("AWS_SECRET_ACCESS_KEY")
                .map_err(|_| "AWS_SECRET_ACCESS_KEY must be set")?,
            region: env::var("AWS_REGION")
                .map_err(|_| "AWS_REGION must be set")?,
            bucket: env::var("S3_BUCKET")
                .map_err(|_| "S3_BUCKET must be set")?,
        })
    }
}

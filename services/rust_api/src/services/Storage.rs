
use aws_sdk_s3::{Client, Region, config::Credentials, config::Builder as S3ConfigBuilder};
use aws_sdk_s3::types::ByteStream;
use aws_config::meta::region::RegionProviderChain;
use std::{env, fs};
use uuid::Uuid;

pub async fn store_file(filename: String, data: bytes::Bytes) -> Result<String, Box<dyn std::error::Error>> {
    dotenvy::dotenv().ok();

    let bucket = env::var("S3_BUCKET")?;
    let region = Region::new(env::var("S3_REGION")?);
    let endpoint = env::var("S3_ENDPOINT")?;

    // Read credentials from Docker secrets
    let access_key = fs::read_to_string("/run/secrets/minio_access_key")
        .or_else(|_| env::var("AWS_ACCESS_KEY_ID")).unwrap_or_else(|_| "minioadmin".to_string());
    let secret_key = fs::read_to_string("/run/secrets/minio_secret_key")
        .or_else(|_| env::var("AWS_SECRET_ACCESS_KEY")).unwrap_or_else(|_| "minioadmin".to_string());

    let credentials = Credentials::new(
        access_key.trim(),
        secret_key.trim(),
        None,
        None,
        "custom-provider",
    );

    let config = aws_config::from_env()
        .region(region)
        .endpoint_url(endpoint)
        .credentials_provider(credentials)
        .load()
        .await;

    let client = Client::new(&config);
    let id = Uuid::new_v4().to_string();

    client.put_object()
        .bucket(bucket)
        .key(format!("uploads/{}-{}", id, filename))
        .body(ByteStream::from(data))
        .send()
        .await?;

    Ok(id)
}

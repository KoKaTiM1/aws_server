use aws_config::meta::region::RegionProviderChain;
use aws_sdk_s3::types::ObjectCannedAcl;
use aws_sdk_s3::Client as S3Client;
use std::error::Error;
use std::path::Path;
use tokio::fs::File;
use tokio::io::AsyncReadExt;

#[derive(Clone)]
pub struct MinioClient {
    pub client: S3Client,
    pub bucket_name: String,
}

impl MinioClient {
    pub async fn new(endpoint: &str, bucket: &str) -> Result<Self, Box<dyn Error>> {
        let region_provider = RegionProviderChain::first_try(Region::new("us-east-1"));

        use aws_sdk_s3::config::{Credentials, Region};
        use std::env;

        // Get credentials from environment variables
        let access_key = env::var("AWS_ACCESS_KEY_ID").unwrap_or_else(|_| "minioadmin".to_string());
        let secret_key =
            env::var("AWS_SECRET_ACCESS_KEY").unwrap_or_else(|_| "minioadmin".to_string());

        let creds = Credentials::new(&access_key, &secret_key, None, None, "static");

        let shared_config = aws_config::defaults(aws_config::BehaviorVersion::latest())
            .region(region_provider)
            .endpoint_url(endpoint)
            .credentials_provider(creds)
            .load()
            .await;

        let s3_config = aws_sdk_s3::config::Builder::from(&shared_config)
            .force_path_style(true)
            .build();

        let s3_client = S3Client::from_conf(s3_config);

        Ok(MinioClient {
            client: s3_client,
            bucket_name: bucket.to_string(),
        })
    }

    pub async fn put_object(
        &self,
        bucket: &str,
        key: &str,
        data: Vec<u8>,
        content_type: Option<String>,
    ) -> Result<(), Box<dyn Error>> {
        let mut put_request = self
            .client
            .put_object()
            .bucket(bucket)
            .key(key)
            .body(data.into())
            .acl(ObjectCannedAcl::PublicRead);

        if let Some(ctype) = content_type {
            put_request = put_request.content_type(ctype);
        }

        put_request.send().await?;
        Ok(())
    }

    pub async fn upload_file(&self, file_path: &Path, key: &str) -> Result<String, Box<dyn Error>> {
        let mut file = File::open(file_path).await?;
        let mut contents = Vec::new();
        file.read_to_end(&mut contents).await?;

        let _put_object_output = self
            .client
            .put_object()
            .bucket(&self.bucket_name)
            .key(key)
            .body(contents.into())
            .acl(ObjectCannedAcl::PublicRead)
            .send()
            .await?;

        Ok(format!("s3://{}/{}", self.bucket_name, key))
    }

    pub async fn get_file(&self, key: &str) -> Result<Vec<u8>, Box<dyn Error>> {
        let get_object_output = self
            .client
            .get_object()
            .bucket(&self.bucket_name)
            .key(key)
            .send()
            .await?;

        let data = get_object_output.body.collect().await?;
        Ok(data.to_vec())
    }
}

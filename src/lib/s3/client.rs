use aws_sdk_s3::primitives::ByteStream;
use crate::vicky::errors::{S3ClientError};
use log::{info};

#[derive(Clone, )]
pub struct S3Client {
    inner: aws_sdk_s3::Client,
    bucket: String,
}

impl S3Client {

    pub fn new(inner: aws_sdk_s3::Client, bucket: String) -> Self {
        S3Client { inner, bucket }
    }

    pub async fn get_logs(&self, task_id: &str) -> Result<Vec<String>, S3ClientError> {

        let key = format!("vicky-logs/{}.log", task_id);

        let get_object_result = self.inner.get_object()
            .bucket(self.bucket.clone())
            .key(key.clone()).send().await?;

        let existing_vec = get_object_result.body.collect().await?;
        let res = String::from_utf8(existing_vec.to_vec()).unwrap().split('\n').map(|x| x.to_string()).collect();
        Ok(res)
    }

    pub async fn upload_log_parts(&self, task_id: &str, log_lines: Vec<String>) -> Result<(), S3ClientError> {

        let key = format!("vicky-logs/{}.log", task_id);

        info!("Checking, if {} already exists", key);
        let get_object_result = self.inner.get_object()
            .bucket(self.bucket.clone())
            .key(key.clone()).send().await;

        let mut new_vec = vec![];

        match get_object_result {
            Ok(gor) => {
                info!("{} already exists, downloading...", key);
                let existing_vec = gor.body.collect().await?;
                new_vec.append(&mut existing_vec.to_vec());
            },
            // This object does not exist, this is fine, currently there is no better way to do this.
            Err(_) => {
                info!("{} does not exist", key);
            }
        }

        let new_line_log_lines: Vec<String> = log_lines.iter().map(|x| format!("{}\n", x)).collect();
        new_vec.append(&mut new_line_log_lines.join("").as_bytes().to_vec());
        let bs = ByteStream::from(new_vec);
        info!("Uploading {}", key);
        self.inner.put_object().bucket(&self.bucket).key(&key).body(bs).send().await?;

        Ok(())
    }

}



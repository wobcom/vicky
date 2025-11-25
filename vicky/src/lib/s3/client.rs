use crate::errors::S3ClientError;
use aws_sdk_s3::primitives::ByteStream;
use log::info;
use uuid::Uuid;

#[derive(Clone)]
pub struct S3Client {
    inner: aws_sdk_s3::Client,
    bucket: String,
}

impl S3Client {
    pub fn new(inner: aws_sdk_s3::Client, bucket: String) -> Self {
        S3Client { inner, bucket }
    }

    pub async fn get_logs(&self, task_id: &str) -> Result<Vec<String>, S3ClientError> {
        let key = format!("vicky-logs/{task_id}.log");

        let get_object_result = self
            .inner
            .get_object()
            .bucket(self.bucket.clone())
            .key(key.clone())
            .send()
            .await?;

        let existing_vec = get_object_result.body.collect().await?;
        let res = String::from_utf8(existing_vec.to_vec())?
            .split(['\n', '\r'])
            .map(|x| x.to_string())
            .collect();
        Ok(res)
    }

    pub async fn upload_log_parts(
        &self,
        task_id: Uuid,
        log_lines: Vec<String>,
    ) -> Result<(), S3ClientError> {
        let key = format!("vicky-logs/{task_id}.log");

        info!("Checking, if {key} already exists");
        let get_object_result = self
            .inner
            .get_object()
            .bucket(self.bucket.clone())
            .key(key.clone())
            .send()
            .await;

        let mut new_vec = vec![];

        match get_object_result {
            Ok(gor) => {
                info!("{key} already exists, downloading...");
                let existing_vec = gor.body.collect().await?;
                new_vec.append(&mut existing_vec.to_vec());
            }
            // This object does not exist, this is fine, currently there is no better way to do this.
            Err(_) => {
                info!("{key} does not exist");
            }
        }

        new_vec.extend(log_lines.join("\n").as_bytes());
        new_vec.push(b'\n');

        let bs = ByteStream::from(new_vec);
        info!("Uploading {key}");
        self.inner
            .put_object()
            .bucket(&self.bucket)
            .key(&key)
            .body(bs)
            .send()
            .await?;

        Ok(())
    }
}

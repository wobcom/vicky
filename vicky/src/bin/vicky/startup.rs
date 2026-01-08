use crate::config::OIDCConfigResolved;
use aws_sdk_s3::{Client, error::SdkError, operation::create_bucket::CreateBucketError};
use rocket::figment;
use snafu::{ResultExt, Snafu};

pub type Result<T, E = Error> = std::result::Result<T, E>;

#[derive(Debug, Snafu)]
#[snafu(visibility(pub(crate)), context(suffix(Err)))]
pub enum Error {
    #[snafu(display("load config: {source}"))]
    Config {
        #[snafu(source(from(figment::Error, Box::new)))]
        source: Box<figment::Error>,
    },

    #[snafu(display("fetch OIDC config: {source}"))]
    OidcFetch { source: reqwest::Error },

    #[snafu(display("parse OIDC config: {source}"))]
    OidcParse { source: reqwest::Error },

    #[snafu(display("prepare log bucket: {source}"))]
    BucketCreate {
        #[snafu(source(from(SdkError<CreateBucketError>, Box::new)))]
        source: Box<SdkError<CreateBucketError>>,
    },

    #[snafu(display("connection to database failed"))]
    DatabaseConnect,

    #[snafu(display("launch server: {source}"))]
    Launch {
        #[snafu(source(from(rocket::Error, Box::new)))]
        source: Box<rocket::Error>,
    },
}

pub async fn fetch_oidc_config(uri: &str) -> Result<OIDCConfigResolved> {
    reqwest::get(uri)
        .await
        .context(OidcFetchErr)?
        .json()
        .await
        .context(OidcParseErr)
}

pub async fn ensure_bucket(client: &Client, bucket: &str) -> Result<()> {
    log::info!("ensuring log bucket {}", bucket);
    match client.create_bucket().bucket(bucket).send().await {
        Ok(b) => {
            log::info!(
                "Bucket \"{}\" was successfully created on the log drain.",
                b.location().unwrap_or_default()
            );
            Ok(())
        }
        Err(err) => {
            match &err {
                SdkError::ServiceError(c)
                    if c.err().is_bucket_already_exists()
                        || c.err().is_bucket_already_owned_by_you() =>
                {
                    log::info!("Bucket \"{bucket}\" is already present on the log drain.");
                    return Ok(());
                }
                SdkError::DispatchFailure(_) => {
                    log::error!(
                        "Failed to communicate with Log Drain / S3 Bucket Connector (is the bucket running and available?): {err:?}"
                    );
                }
                _ => {
                    log::error!(
                        "Log Drain / S3 Bucket Connector ran into an irrecoverable error: {err:?}"
                    );
                }
            }

            Err(Error::BucketCreate { source: err.into() })
        }
    }
}

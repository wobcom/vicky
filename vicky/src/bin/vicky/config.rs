use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use rocket::figment::providers::{Env, Format, Toml};
use rocket::figment::{Figment, Profile};
use rocket::serde::{Deserialize, Serialize};
use vickylib::s3::client::S3Client;

#[derive(Deserialize)]
pub struct S3Config {
    pub endpoint: String,
    access_key_id: String,
    secret_access_key: String,
    pub region: String,
    pub log_bucket: String,
}

#[derive(Deserialize)]
pub struct OIDCConfig {
    pub well_known_uri: String,
}

#[derive(Deserialize)]
pub struct OIDCConfigResolved {
    pub userinfo_endpoint: String,
    pub jwks_uri: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WebConfig {
    pub authority: String,
    pub client_id: String,
}

#[derive(Deserialize)]
pub struct Config {
    pub machines: Vec<String>,
    pub s3_config: S3Config,
    pub oidc_config: OIDCConfig,
    pub web_config: WebConfig,
}

pub fn build_rocket_config() -> Figment {
    // Taken from rocket source code and added .split("__") to be able to add keys in nested structures.
    Figment::from(rocket::Config::default())
        .merge(Toml::file(Env::var_or("ROCKET_CONFIG", "config.toml")).nested())
        .merge(
            Env::prefixed("ROCKET_")
                .ignore(&["PROFILE"])
                .split("__")
                .global(),
        )
        .select(Profile::from_env_or(
            "ROCKET_PROFILE",
            rocket::Config::DEFAULT_PROFILE,
        ))
}

impl S3Config {
    pub fn credentials(&self) -> Credentials {
        Credentials::new(
            &self.access_key_id,
            &self.secret_access_key,
            None,
            None,
            "static",
        )
    }

    pub fn build_config(&self) -> aws_sdk_s3::Config {
        log::info!("building s3 client");

        aws_sdk_s3::Config::builder()
            .behavior_version(BehaviorVersion::v2024_03_28())
            .force_path_style(true)
            .endpoint_url(&self.endpoint)
            .credentials_provider(self.credentials())
            .region(Region::new(self.region.clone()))
            .build()
    }

    pub fn create_client(&self) -> aws_sdk_s3::Client {
        aws_sdk_s3::Client::from_conf(self.build_config())
    }

    pub fn create_bucket_client(&self) -> S3Client {
        S3Client::new(self.create_client(), self.log_bucket.clone())
    }
}

impl OIDCConfigResolved {
    pub fn jwks_verifier(&self) -> jwtk::jwk::RemoteJwksVerifier {
        jwtk::jwk::RemoteJwksVerifier::new(
            self.jwks_uri.clone(),
            None,
            std::time::Duration::from_secs(300),
        )
    }
}

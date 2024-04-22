use std::collections::HashMap;

use auth::User;
use aws_sdk_s3::config::{Credentials, Region};
use etcd_client::{Identity, Certificate, TlsOptions, ConnectOptions};
use log::info;

use rand::Rng;
use rocket::fairing::AdHoc;
use rocket::figment::{Figment, Profile};
use rocket::figment::providers::{Toml, Env, Format};
use rocket::routes;
use rocket_oauth2::OAuth2;
use serde::Deserialize;
use tokio::sync::broadcast;
use vickylib::etcd::election::{NodeId, Election};
use vickylib::logs::LogDrain;
use vickylib::s3::client::S3Client;

use crate::tasks::{tasks_claim, tasks_finish, tasks_get_machine, tasks_get_user, tasks_add, tasks_get_logs, tasks_put_logs, tasks_specific_get_machine, tasks_specific_get_user};
use crate::events::{get_global_events, GlobalEvent};

use crate::auth::{github_login, github_callback, logout, GitHubUserInfo};
use crate::user::{get_user};

mod tasks;
mod auth;
mod user;
mod events;
mod errors;

#[derive(Deserialize)]
pub struct TlsConfigOptions {
    ca_file: String,
    certificate_file: String,
    key_file: String,

}
#[derive(Deserialize)]
pub struct EtcdConfig {
    endpoints: Vec<String>,
    tls_options: Option<TlsConfigOptions>
}

#[derive(Deserialize)]
pub struct S3Config {
    endpoint: String,
    access_key_id: String,
    secret_access_key: String,
    region: String,
    
    log_bucket: String,
}

#[derive(Deserialize)]
pub struct Config {
    users: HashMap<String, User>,
    machines: Vec<String>,

    etcd_config: EtcdConfig,
    s3_config: S3Config,

}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder().filter_module("vicky", log::LevelFilter::Debug).init();

    // Took from rocket source code and added .split("__") to be able to add keys in nested structures.
    let rocket_config_figment = Figment::from(rocket::Config::default())
        .merge(Toml::file(Env::var_or("ROCKET_CONFIG", "Rocket.toml")).nested())
        .merge(Env::prefixed("ROCKET_").ignore(&["PROFILE"]).split("__").global())
        .select(Profile::from_env_or("ROCKET_PROFILE", rocket::Config::DEFAULT_PROFILE));

    let build_rocket = rocket::custom(rocket_config_figment);

    let app_config = build_rocket.figment().extract::<Config>()?;

    let options = match app_config.etcd_config.tls_options {
        Some(tls_options) => {
            let server_root_ca_cert = std::fs::read_to_string(tls_options.ca_file)?;
            let server_root_ca_cert = Certificate::from_pem(server_root_ca_cert);
            let client_cert = std::fs::read_to_string(tls_options.certificate_file)?;
            let client_key = std::fs::read_to_string(tls_options.key_file)?;
            let client_identity = Identity::from_pem(client_cert, client_key);

            Some(
                TlsOptions::new()
                    .ca_certificate(server_root_ca_cert)
                    .identity(client_identity)
            )

        },
        None => None,
    };

    let connect_options = options.map(|options: TlsOptions| ConnectOptions::new().with_tls(options));
    let etcd_client = etcd_client::Client::connect(app_config.etcd_config.endpoints, connect_options).await?;

    let aws_cfg = app_config.s3_config; 

    let aws_conf = aws_config::from_env()
        .endpoint_url(aws_cfg.endpoint)
        .credentials_provider(Credentials::new(aws_cfg.access_key_id, aws_cfg.secret_access_key, None, None, "static"))
        .region(Region::new(aws_cfg.region))
        .load()
        .await;
    
    
    let s3_client = aws_sdk_s3::Client::new(&aws_conf);
    let s3_ext_client_drain = S3Client::new(s3_client.clone(), aws_cfg.log_bucket.clone());
    let s3_ext_client = S3Client::new(s3_client, aws_cfg.log_bucket.clone());

    let mut rng = rand::thread_rng();
    let node_id: NodeId = format!("node_{}", rng.gen::<i32>()).to_string();
    info!("Generated unique node id as {}", node_id);

    let mut election = Election::new(&etcd_client, node_id);
    election.keep_alive();

    election.elect().await?;
    info!("Leader election won, we are now the leader!");

    let log_drain = LogDrain::new(s3_ext_client_drain);

    let (tx_global_events, _rx_task_events) = broadcast::channel::<GlobalEvent>(5);

    build_rocket
        .manage(etcd_client)
        .manage(s3_ext_client)
        .manage(log_drain)
        .manage(tx_global_events)
        .attach(OAuth2::<GitHubUserInfo>::fairing("github"))
        .attach(AdHoc::config::<Config>())
        .mount("/api/v1/user", routes![get_user])
        .mount("/api/v1/auth", routes![github_login, github_callback, logout])
        .mount("/api/v1/events", routes![get_global_events])
        .mount("/api/v1/tasks", routes![tasks_get_machine, tasks_get_user, tasks_specific_get_machine, tasks_specific_get_user, tasks_claim, tasks_finish, tasks_add, tasks_get_logs, tasks_put_logs])
        .launch()
        .await?;

    Ok(())
}

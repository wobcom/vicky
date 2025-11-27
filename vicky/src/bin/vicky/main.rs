use std::time::Duration;

use aws_sdk_s3::config::{BehaviorVersion, Credentials, Region};
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};
use jwtk::jwk::RemoteJwksVerifier;
use rocket::fairing::AdHoc;
use rocket::figment::providers::{Env, Format, Toml};
use rocket::figment::{Figment, Profile};
use rocket::{routes, Build, Rocket};
use serde::{Deserialize, Serialize};
use snafu::ResultExt;
use tokio::sync::broadcast;

use errors::AppError;
use vickylib::database::entities::Database;
use vickylib::logs::LogDrain;
use vickylib::s3::client::S3Client;

use crate::events::{get_global_events, GlobalEvent};
use crate::locks::{
    locks_get_active_machine, locks_get_active_user, locks_get_detailed_poisoned_machine,
    locks_get_detailed_poisoned_user, locks_get_poisoned_machine, locks_get_poisoned_user,
    locks_unlock,
};
use crate::startup::Result;
use crate::tasks::{
    tasks_add, tasks_claim, tasks_download_logs, tasks_finish, tasks_get_logs, tasks_get_machine,
    tasks_get_user, tasks_put_logs, tasks_specific_get_machine, tasks_specific_get_user,
};
use crate::user::get_user;
use crate::webconfig::get_web_config;

mod auth;
mod errors;
mod events;
mod locks;
mod startup;
mod tasks;
mod user;
mod webconfig;

#[derive(Deserialize)]
pub struct S3Config {
    endpoint: String,
    access_key_id: String,
    secret_access_key: String,
    region: String,

    log_bucket: String,
}

#[derive(Deserialize)]
pub struct OIDCConfig {
    well_known_uri: String,
}

#[derive(Deserialize)]
pub struct OIDCConfigResolved {
    userinfo_endpoint: String,
    jwks_uri: String,
}

#[derive(Deserialize, Serialize, Clone)]
pub struct WebConfig {
    authority: String,
    client_id: String,
}

#[derive(Deserialize)]
pub struct Config {
    machines: Vec<String>,

    s3_config: S3Config,

    oidc_config: OIDCConfig,

    web_config: WebConfig,
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn run_migrations(connection: &mut impl MigrationHarness<diesel::pg::Pg>) -> Result<(), AppError> {
    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(_) => {
            log::info!("Migrations successfully completed");
            Ok(())
        }
        Err(e) => {
            log::error!("Error running migrations {e}");
            Err(AppError::MigrationError(e.to_string()))
        }
    }
}

async fn run_rocket_migrations(rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
    log::info!("Running database migrations");

    let Some(db) = Database::get_one(&rocket).await else {
        log::error!("Failed to get a database connection");
        return Err(rocket);
    };

    match db.run(run_migrations).await {
        Ok(_) => Ok(rocket),
        Err(_) => Err(rocket),
    }
}

#[tokio::main]
async fn main() {
    if let Err(e) = inner_main().await {
        log::error!("Fatal: {e}");
    }
}

async fn inner_main() -> Result<()> {
    env_logger::builder()
        .filter_module("vicky", log::LevelFilter::Debug)
        .init();
    log::info!("vicky starting...");

    // Took from rocket source code and added .split("__") to be able to add keys in nested structures.
    let rocket_config_figment = Figment::from(rocket::Config::default())
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
        ));

    let build_rocket = rocket::custom(rocket_config_figment);

    log::info!("loading service config...");
    let app_config = build_rocket
        .figment()
        .extract::<Config>()
        .context(startup::ConfigErr)?;

    log::info!(
        "fetching OIDC discovery from {}",
        app_config.oidc_config.well_known_uri
    );
    let oidc_config_resolved =
        startup::fetch_oidc_config(&app_config.oidc_config.well_known_uri).await?;

    log::info!(
        "Fetched OIDC configuration, found jwks_uri={}",
        oidc_config_resolved.jwks_uri
    );

    let jwks_verifier = RemoteJwksVerifier::new(
        oidc_config_resolved.jwks_uri.clone(),
        None,
        Duration::from_secs(300),
    );

    let env_aws_cfg = app_config.s3_config;

    let creds = Credentials::new(
        env_aws_cfg.access_key_id,
        env_aws_cfg.secret_access_key,
        None,
        None,
        "static",
    );

    log::info!("building s3 client");
    let s3_conf = aws_sdk_s3::Config::builder()
        .behavior_version(BehaviorVersion::v2024_03_28())
        .force_path_style(true)
        .endpoint_url(env_aws_cfg.endpoint)
        .credentials_provider(creds)
        .region(Region::new(env_aws_cfg.region))
        .build();

    let s3_client = aws_sdk_s3::Client::from_conf(s3_conf);

    log::info!("ensuring log bucket {}", env_aws_cfg.log_bucket);
    startup::ensure_bucket(&s3_client, &env_aws_cfg.log_bucket).await?;

    let s3_ext_client_drain = S3Client::new(s3_client.clone(), env_aws_cfg.log_bucket.clone());
    let s3_ext_client = S3Client::new(s3_client, env_aws_cfg.log_bucket.clone());

    let log_drain = LogDrain::new(s3_ext_client_drain);

    let (tx_global_events, _rx_task_events) = broadcast::channel::<GlobalEvent>(5);

    log::info!("starting web api");
    build_rocket
        .manage(s3_ext_client)
        .manage(log_drain)
        .manage(jwks_verifier)
        .manage(tx_global_events)
        .manage(app_config.web_config)
        .manage(oidc_config_resolved)
        .attach(Database::fairing())
        .attach(AdHoc::config::<Config>())
        .attach(AdHoc::try_on_ignite(
            "run migrations",
            run_rocket_migrations,
        ))
        .mount("/api/v1/web-config", routes![get_web_config])
        .mount("/api/v1/user", routes![get_user])
        .mount("/api/v1/events", routes![get_global_events])
        .mount(
            "/api/v1/tasks",
            routes![
                tasks_get_machine,
                tasks_get_user,
                tasks_specific_get_machine,
                tasks_specific_get_user,
                tasks_claim,
                tasks_finish,
                tasks_add,
                tasks_get_logs,
                tasks_put_logs,
                tasks_download_logs,
            ],
        )
        .mount(
            "/api/v1/locks",
            routes![
                locks_get_poisoned_user,
                locks_get_poisoned_machine,
                locks_get_detailed_poisoned_user,
                locks_get_detailed_poisoned_machine,
                locks_get_active_user,
                locks_get_active_machine,
                locks_unlock
            ],
        )
        .launch()
        .await
        .context(startup::LaunchErr)?;

    Ok(())
}

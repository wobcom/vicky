use std::time::Duration;

use aws_sdk_s3::config::{Credentials, Region};
use errors::AppError;
use jwtk::jwk::RemoteJwksVerifier;

use rocket::fairing::AdHoc;
use rocket::figment::providers::{Env, Format, Toml};
use rocket::figment::{Figment, Profile};
use rocket::{routes, Rocket, Build};
use serde::Deserialize;
use tokio::sync::broadcast;
use vickylib::database::entities::Database;
use vickylib::logs::LogDrain;
use vickylib::s3::client::S3Client;
use diesel_migrations::{embed_migrations, EmbeddedMigrations, MigrationHarness};

use crate::events::{get_global_events, GlobalEvent};
use crate::tasks::{
    tasks_add, tasks_claim, tasks_finish, tasks_get_logs, tasks_get_machine, tasks_get_user,
    tasks_put_logs, tasks_specific_get_machine, tasks_specific_get_user,
};

use crate::user::get_user;

mod auth;
mod errors;
mod events;
mod tasks;
mod user;

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
    jwks_url: String,
}

#[derive(Deserialize)]
pub struct Config {
    machines: Vec<String>,

    s3_config: S3Config,

    oidc_config: OIDCConfig,
}

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn run_migrations(connection: &mut impl MigrationHarness<diesel::pg::Pg>) -> Result<(), AppError> {
    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(_) => {
            log::info!("Migrations successfully completed");
            Ok(())
        },
        Err(e) => {
            log::error!("Error running migrations {e}");
            Err(AppError::MigrationError(e.to_string()))
        }
    }
}

async fn run_rocket_migrations(rocket: Rocket<Build>) -> Result<Rocket<Build>,Rocket<Build>> {
    let db: Database = Database::get_one(&rocket).await.unwrap();
    match db.run(run_migrations).await {
        Ok(_) => Ok(rocket),
        Err(_) => Err(rocket)
    }
}

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_module("vicky", log::LevelFilter::Debug)
        .init();

    // Took from rocket source code and added .split("__") to be able to add keys in nested structures.
    let rocket_config_figment = Figment::from(rocket::Config::default())
        .merge(Toml::file(Env::var_or("ROCKET_CONFIG", "Rocket.toml")).nested())
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

    let app_config = build_rocket.figment().extract::<Config>()?;

    let jwks_verifier = RemoteJwksVerifier::new(
        app_config.oidc_config.jwks_url,
        None,
        Duration::from_secs(300),
    );

    let aws_cfg = app_config.s3_config;

    let aws_conf = aws_config::from_env()
        .endpoint_url(aws_cfg.endpoint)
        .credentials_provider(Credentials::new(
            aws_cfg.access_key_id,
            aws_cfg.secret_access_key,
            None,
            None,
            "static",
        ))
        .region(Region::new(aws_cfg.region))
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&aws_conf);
    let s3_ext_client_drain = S3Client::new(s3_client.clone(), aws_cfg.log_bucket.clone());
    let s3_ext_client = S3Client::new(s3_client, aws_cfg.log_bucket.clone());

    let log_drain = LogDrain::new(s3_ext_client_drain);

    let (tx_global_events, _rx_task_events) = broadcast::channel::<GlobalEvent>(5);

    build_rocket
        .manage(s3_ext_client)
        .manage(log_drain)
        .manage(jwks_verifier)
        .manage(tx_global_events)
        .attach(Database::fairing())
        .attach(AdHoc::config::<Config>())
        .attach(AdHoc::try_on_ignite("run migrations", run_rocket_migrations))
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
                tasks_put_logs
            ],
        )
        .launch()
        .await?;

    Ok(())
}

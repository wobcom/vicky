use crate::config::{Config, OIDCConfigResolved, build_rocket_config};
use crate::events::{GlobalEvent, get_global_events};
use crate::locks::{
    locks_get_active, locks_get_detailed_poisoned, locks_get_poisoned, locks_unlock,
};
use crate::startup::Result;
use crate::tasks::{
    tasks_add, tasks_claim, tasks_confirm, tasks_count, tasks_download_logs, tasks_finish,
    tasks_get, tasks_get_logs, tasks_get_specific, tasks_put_logs,
};
use crate::user::get_user;
use crate::webconfig::get_web_config;
use diesel_migrations::{EmbeddedMigrations, MigrationHarness, embed_migrations};
use errors::AppError;
use jwtk::jwk::RemoteJwksVerifier;
use log::{error, info, LevelFilter};
use rocket::fairing::AdHoc;
use rocket::{routes, Build, Rocket};
use snafu::ResultExt;
use tokio::sync::broadcast;
use tokio::sync::broadcast::Sender;
use vickylib::database::entities::Database;
use vickylib::logs::LogDrain;
use vickylib::s3::client::S3Client;

mod auth;
mod config;
mod errors;
mod events;
mod locks;
mod startup;
mod tasks;
mod user;
mod webconfig;

pub const MIGRATIONS: EmbeddedMigrations = embed_migrations!();

fn run_migrations(connection: &mut impl MigrationHarness<diesel::pg::Pg>) -> Result<(), AppError> {
    match connection.run_pending_migrations(MIGRATIONS) {
        Ok(_) => {
            info!("Migrations successfully completed");
            Ok(())
        }
        Err(e) => {
            error!("Error running migrations {e}");
            Err(AppError::MigrationError(e.to_string()))
        }
    }
}

async fn run_rocket_migrations(rocket: Rocket<Build>) -> Result<Rocket<Build>, Rocket<Build>> {
    info!("Running database migrations");

    let Some(db) = Database::get_one(&rocket).await else {
        error!("Failed to get a database connection");
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
        error!("Fatal: {e}");
    }
}

async fn inner_main() -> Result<()> {
    env_logger::builder()
        .filter_module("vicky", LevelFilter::Debug)
        .init();
    info!("vicky starting...");

    info!("loading service config...");
    let rocket_config = build_rocket_config();
    let app_config = rocket_config
        .extract::<Config>()
        .context(startup::ConfigErr)?;
    let build_rocket = rocket::custom(build_rocket_config());

    info!(
        "fetching OIDC discovery from {}",
        app_config.oidc_config.well_known_uri
    );
    let oidc_config_resolved =
        startup::fetch_oidc_config(&app_config.oidc_config.well_known_uri).await?;

    info!(
        "Fetched OIDC configuration, found jwks_uri={}",
        oidc_config_resolved.jwks_uri
    );

    let jwks_verifier = oidc_config_resolved.jwks_verifier();

    let s3_conf = app_config.s3_config.build_config();
    let s3_client = aws_sdk_s3::Client::from_conf(s3_conf);
    startup::ensure_bucket(&s3_client, &app_config.s3_config.log_bucket).await?;

    let s3_log_bucket_client = app_config.s3_config.create_bucket_client();
    let log_drain = LogDrain::new(s3_log_bucket_client.clone());

    let (tx_global_events, _rx_task_events) = broadcast::channel::<GlobalEvent>(5);

    serve_web_api(
        app_config,
        build_rocket,
        oidc_config_resolved,
        jwks_verifier,
        s3_log_bucket_client,
        log_drain,
        tx_global_events,
    )
    .await?;

    Ok(())
}

async fn serve_web_api(
    app_config: Config,
    build_rocket: Rocket<Build>,
    oidc_config_resolved: OIDCConfigResolved,
    jwks_verifier: RemoteJwksVerifier,
    s3_log_bucket_client: S3Client,
    log_drain: LogDrain,
    tx_global_events: Sender<GlobalEvent>,
) -> Result<()> {
    info!("starting web api");

    build_rocket
        .manage(s3_log_bucket_client)
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
                tasks_count,
                tasks_get,
                tasks_get_specific,
                tasks_claim,
                tasks_finish,
                tasks_add,
                tasks_get_logs,
                tasks_put_logs,
                tasks_download_logs,
                tasks_confirm
            ],
        )
        .mount(
            "/api/v1/locks",
            routes![
                locks_get_poisoned,
                locks_get_detailed_poisoned,
                locks_get_active,
                locks_unlock
            ],
        )
        .launch()
        .await
        .context(startup::LaunchErr)?;

    Ok(())
}

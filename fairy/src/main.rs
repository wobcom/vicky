use rocket::figment::{Figment, Profile};
use rocket::figment::providers::{Env, Format, Toml};
use tokio::task::spawn_blocking;

use crate::config::AppConfig;

mod api;
mod config;
mod error;
mod tasks;
mod utils;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::builder()
        .filter_level(log::LevelFilter::Debug)
        .init();

    #[cfg(not(feature = "nixless-test-mode"))]
    ensure_nix();

    log::info!("Fairy starting up.");

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

    let app_config = rocket_config_figment.extract::<AppConfig>()?;
    match spawn_blocking(|| tasks::runner::run(app_config)).await?.await {
        Ok(_) => log::info!("Runner exited successfully."),
        Err(e) => {
            for l in e.chain() { // chain here for full debug info
                log::error!("Runner encountered a fatal error: {:?}", l);
            }
            return Err(e);
        }
    }

    Ok(())
}

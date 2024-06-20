use std::process::{exit, Stdio};
use std::sync::Arc;

use anyhow::anyhow;
use api::HttpClient;
use futures_util::{Sink, StreamExt, TryStreamExt};
use serde::{Deserialize, Serialize};
use tokio::process::Command;
use tokio_util::codec::{FramedRead, LinesCodec};
use uuid::Uuid;
use which::which;
use reqwest::{self, Method};

use rocket::figment::providers::{Env, Format, Toml};
use rocket::figment::{Figment, Profile};

mod api;
pub mod error;


#[derive(Deserialize, Debug)]
pub struct OIDCConfig {
    issuer_url: String,
    client_id: String,
    client_secret: String,
}

#[derive(Deserialize, Debug)]
pub(crate) struct AppConfig {
    pub(crate) vicky_url: String,
    pub(crate) vicky_external_url: String,
    pub(crate) features: Vec<String>,
    pub(crate) oidc_config: OIDCConfig,
}

const CODE_NIX_NOT_INSTALLED: i32 = 1;

fn ensure_nix() {
    if which("nix").is_err() {
        log::error!("\"nix\" binary not found. Please install nix or run on a nix-os host.");
        exit(CODE_NIX_NOT_INSTALLED);
    }
}

fn main() -> anyhow::Result<()> {
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
    run(app_config)
}



#[derive(Debug, Deserialize)]
pub struct FlakeRef {
    pub flake: String,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
    New,
    Running,
    Finished(TaskResult),
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub flake_ref: FlakeRef,
}

fn log_sink(
    cfg: Arc<AppConfig>,
    task_id: Uuid,
) -> impl Sink<Vec<String>, Error = anyhow::Error> + Send {
    let vicky_client_task = HttpClient::new(cfg.clone());

    futures_util::sink::unfold(vicky_client_task, move |mut http_client, lines: Vec<String>| {
        async move {
            let response = http_client.do_request::<_, ()>(
                Method::POST,
                &format!("api/v1/tasks/{}/logs", task_id),
                &serde_json::json!({ "lines": lines }),
            )
            .await;

            match response {
                Ok(_) => {
                    log::info!("logged {} line(s) from task", lines.len());
                    Ok(http_client)
                }
                Err(e) => {
                    log::error!(
                        "could not log from task. {} lines were dropped",
                        lines.len()
                    );
                    Err(e)
                }
            }
        }
    })
}

async fn try_run_task(cfg: Arc<AppConfig>, task: &Task) -> anyhow::Result<()> {
    let mut args = vec!["run".into(), "-L".into(), task.flake_ref.flake.clone()];
    args.extend(task.flake_ref.args.clone());

    let mut child = Command::new("nix")
        .args(args)
        .env("VICKY_API_URL", &cfg.vicky_external_url)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let logger = log_sink(cfg.clone(), task.id);

    let lines = futures_util::stream::select(
        FramedRead::new(child.stdout.take().unwrap(), LinesCodec::new()),
        FramedRead::new(child.stderr.take().unwrap(), LinesCodec::new()),
    );

    lines
        .ready_chunks(1024) // TODO switch to try_ready_chunks
        .map(|v| v.into_iter().collect::<Result<Vec<_>, _>>())
        .map_err(anyhow::Error::from)
        .forward(logger)
        .await?;
    let exit_status = child.wait().await?;

    if exit_status.success() {
        log::info!("task finished: {} {} 🎉", task.id, task.display_name);
        Ok(())
    } else {
        Err(anyhow!("exit code {:?}", exit_status.code()))
    }
}

async fn run_task(cfg: Arc<AppConfig>, task: Task) {

    let mut vicky_client_task = HttpClient::new(cfg.clone());

    #[cfg(not(feature = "nixless-test-mode"))]
    let result = match try_run_task(cfg.clone(), &task).await {
            Err(e) => {
                log::info!("task failed: {} {} {:?}", task.id, task.display_name, e);
                TaskResult::Error
            }
            Ok(_) => TaskResult::Success,
        };

    #[cfg(feature = "nixless-test-mode")]
    let result = TaskResult::Success;

    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let _ = vicky_client_task.do_request::<_, ()>(
        Method::POST,
        &format!("api/v1/tasks/{}/finish", task.id),
        &serde_json::json!({ "result": result }),
    )
    .await;
}

async fn try_claim(cfg: Arc<AppConfig>, vicky_client: &mut HttpClient) -> anyhow::Result<()> {
    log::debug!("trying to claim task...");
    if let Some(task) = vicky_client.do_request::<_, Option<Task>>(
        Method::POST,
        "api/v1/tasks/claim",
        &serde_json::json!({ "features": cfg.features }),
    )
    .await?
    {
        log::info!("task claimed: {} {} 🎉", task.id, task.display_name);
        log::debug!("{:#?}", task);

        tokio::task::spawn(run_task(cfg.clone(), task));
    } else {
        log::debug!("no work available...");
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn run(cfg: AppConfig) -> anyhow::Result<()> {
    let cfg = Arc::new(cfg);
    let mut vicky_client_mgmt = HttpClient::new(cfg.clone());
    vicky_client_mgmt.renew_access_token().await?;

    log::info!("config valid, starting communication with vicky");
    log::info!("waiting for tasks...");

    loop {
        if let Err(e) = try_claim(cfg.clone(), &mut vicky_client_mgmt).await {
            log::error!("{}", e);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

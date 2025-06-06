use std::process::{exit, Stdio};
use std::sync::Arc;

use anyhow::anyhow;
use futures_util::{Sink, StreamExt, TryStreamExt};
use hyper::{Body, Client, Method, Request};
use rocket::figment::{Figment, Profile};
use rocket::figment::providers::{Env, Format, Toml};
use serde::{Deserialize, Serialize};
use serde::de::DeserializeOwned;
use tokio::process::Command;
use tokio_util::codec::{FramedRead, LinesCodec};
use uuid::Uuid;
use which::which;

#[derive(Deserialize)]
pub(crate) struct AppConfig {
    pub(crate) vicky_url: String,
    pub(crate) vicky_external_url: String,
    pub(crate) machine_token: String,
    pub(crate) features: Vec<String>,
    pub(crate) verbose_nix_logs: bool,
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

async fn api<BODY: Serialize, RESPONSE: DeserializeOwned>(
    cfg: &AppConfig,
    method: Method,
    endpoint: &str,
    q: &BODY,
) -> anyhow::Result<RESPONSE> {
    let client = Client::new();
    let req_data = serde_json::to_vec(q)?;

    let request = Request::builder()
        .uri(format!("{}/{}", cfg.vicky_url, endpoint))
        .method(method)
        .header("content-type", "application/json")
        .header("authorization", &cfg.machine_token)
        .body(Body::from(req_data))?;

    let response = client.request(request).await?;

    if !response.status().is_success() {
        anyhow::bail!("API error: {:?}", response);
    }

    let resp_data = hyper::body::to_bytes(response.into_body()).await?;
    Ok(serde_json::from_slice(&resp_data)?)
}

#[derive(Debug, Deserialize)]
pub struct FlakeRef {
    pub flake: String,
    pub args: Vec<String>,
}

#[derive(Debug, Serialize, Deserialize)]
#[serde(tag = "result", rename_all="UPPERCASE")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "state", rename_all="UPPERCASE")]
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
    futures_util::sink::unfold((), move |_, lines: Vec<String>| {
        let cfg = cfg.clone();
        async move {
            let response = api::<_, ()>(
                &cfg,
                Method::POST,
                &format!("api/v1/tasks/{}/logs", task_id),
                &serde_json::json!({ "lines": lines }),
            )
            .await;

            match response {
                Ok(_) => {
                    log::info!("logged {} line(s) from task", lines.len());
                    Ok(())
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
    let mut args = vec!["run".into()];

    if !&cfg.verbose_nix_logs {
        args.push("--quiet".into());
    }

    args.extend(vec!["-L".into(), task.flake_ref.flake.clone()]);
    args.extend(task.flake_ref.args.clone());

    let mut child = Command::new("nix")
        .args(args)
        .env("VICKY_API_URL", &cfg.vicky_external_url)
        .env("VICKY_MACHINE_TOKEN", &cfg.machine_token)
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
    let _ = api::<_, ()>(
        &cfg,
        Method::POST,
        &format!("api/v1/tasks/{}/finish", task.id),
        &serde_json::json!({ "result": result }),
    )
    .await;
}

async fn try_claim(cfg: Arc<AppConfig>) -> anyhow::Result<()> {
    log::debug!("trying to claim task...");
    if let Some(task) = api::<_, Option<Task>>(
        &cfg,
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
    log::info!("config valid, starting communication with vicky");
    log::info!("waiting for tasks...");

    let cfg = Arc::new(cfg);
    loop {
        if let Err(e) = try_claim(cfg.clone()).await {
            log::error!("{}", e);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

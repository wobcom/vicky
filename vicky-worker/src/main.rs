mod config;

use config::Config;

use anyhow::anyhow;
use std::process::Stdio;
use std::sync::Arc;
use uuid::Uuid;
use hyper::{Client, Request, Body, Method};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};
use tokio::process::Command;
use tokio_util::codec::{FramedRead, LinesCodec};
use futures_util::{Sink, StreamExt, TryStreamExt};

fn main() -> anyhow::Result<()> {
    env_logger::init();
    let cfg: Config = serde_yaml::from_slice(&std::fs::read("config.yaml")?)?;
    run(cfg)
}

async fn api<Q: Serialize, R: DeserializeOwned>(cfg: &Config, method: Method, endpoint: &str, q: &Q) -> anyhow::Result<R> {
    let client = Client::new();
    let req_data = serde_json::to_vec(&q)?;

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
#[serde(tag = "result")]
pub enum TaskResult {
    SUCCESS,
    ERROR,
}

#[derive(Debug, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
    NEW,
    RUNNING,
    FINISHED(TaskResult)
}

#[derive(Debug, Deserialize)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub flake_ref: FlakeRef,
}

fn log_sink(cfg: Arc<Config>, task_id: Uuid) -> impl Sink<String, Error = anyhow::Error> + Send {
    futures_util::sink::unfold((), move |_, line| {
        let cfg = cfg.clone();
        async move {
            api::<_, ()>(&cfg, Method::POST, &format!("api/v1/tasks/{}/logs", task_id), &serde_json::json!({ "lines": [line] })).await
        }
    })
}

async fn try_run_task(cfg: Arc<Config>, task: &Task) -> anyhow::Result<()> {

    let mut args = Vec::new();
    args.push("run".into());
    args.push("-v".into());
    args.push("-L".into());
    args.push(task.flake_ref.flake.clone());
    args.extend(task.flake_ref.args.clone());
    let mut child = Command::new("nix")
        .args(args)
        .kill_on_drop(true)
        .stdin(Stdio::null())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()?;

    let logger = log_sink(cfg.clone(), task.id);

    let mut lines = tokio_stream::StreamExt::merge(
        FramedRead::new(child.stdout.take().unwrap(), LinesCodec::new()),
        FramedRead::new(child.stderr.take().unwrap(), LinesCodec::new())
    );

    lines.map_err(anyhow::Error::from).forward(logger).await?;
    let exit_status = child.wait().await?;

    if exit_status.success() {
        log::info!("task finished: {} {} 🎉", task.id, task.display_name);
        Ok(())
    } else {
        Err(anyhow!("exit code {:?}", exit_status.code()))
    }
}

async fn run_task(cfg: Arc<Config>, task: Task) {
    let result = match try_run_task(cfg.clone(), &task).await {
        Err(e) => {
            log::info!("task failed: {} {} {:?}", task.id, task.display_name, e);
            TaskResult::ERROR
        },
        Ok(_) => TaskResult::SUCCESS,
    };
    tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    let _ = api::<_, ()>(&cfg, Method::POST, &format!("api/v1/tasks/{}/finish", task.id), &serde_json::json!({ "result": result })).await;
}

async fn try_claim(cfg: Arc<Config>) -> anyhow::Result<()> {
    if let Some(task) = api::<_, Option<Task>>(&cfg, Method::POST, "api/v1/tasks/claim", &None::<u32>).await? {
        log::info!("task claimed: {} {} 🎉", task.id, task.display_name);
        log::debug!("{:#?}", task);

        tokio::task::spawn(run_task(cfg.clone(), task));
    }

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn run(cfg: Config) -> anyhow::Result<()> {
    println!("Hello, world!");

    let cfg = Arc::new(cfg);
    loop {
        if let Err(e) = try_claim(cfg.clone()).await {
            log::error!("{}", e);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
        tokio::time::sleep(std::time::Duration::from_secs(1)).await;
    }
}

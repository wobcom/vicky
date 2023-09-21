mod config;

use config::Config;

use uuid::Uuid;
use hyper::{Client, Request, Body, Method};
use serde::de::DeserializeOwned;
use serde::{Serialize, Deserialize};

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

#[derive(Debug, Deserialize)]
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

async fn try_claim(cfg: &Config) -> anyhow::Result<()> {
    let task: Task = api(cfg, Method::POST, "api/v1/tasks/claim", &None::<u32>).await?;
    log::info!("task claimed! 🎉");
    log::debug!("{:#?}", task);

    Ok(())
}

#[tokio::main(flavor = "current_thread")]
async fn run(cfg: Config) -> anyhow::Result<()> {
    println!("Hello, world!");

    loop {
        if let Err(e) = try_claim(&cfg).await {
            log::error!("{}", e);
            tokio::time::sleep(std::time::Duration::from_secs(5)).await;
        }
    }
}

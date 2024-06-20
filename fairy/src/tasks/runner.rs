use std::process::Stdio;
use std::sync::Arc;

use anyhow::anyhow;
use futures_util::{StreamExt, TryStreamExt};
use reqwest::Method;
use tokio::process::Command;
use tokio_util::codec::{FramedRead, LinesCodec};

use crate::api::HttpClient;
use crate::config::AppConfig;
use crate::tasks::types::{Task, TaskResult};

#[tokio::main(flavor = "current_thread")]
pub async fn run(cfg: AppConfig) -> anyhow::Result<()> {
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
    let _ = vicky_client_task
        .do_request::<_, ()>(
            Method::POST,
            &format!("api/v1/tasks/{}/finish", task.id),
            &serde_json::json!({ "result": result }),
        )
        .await;
}

async fn try_claim(cfg: Arc<AppConfig>, vicky_client: &mut HttpClient) -> anyhow::Result<()> {
    log::debug!("trying to claim task...");
    if let Some(task) = vicky_client
        .do_request::<_, Option<Task>>(
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

use std::sync::Arc;

use futures_util::Sink;
use reqwest::Method;
use uuid::Uuid;

use crate::api::HttpClient;
use crate::config::AppConfig;

pub fn log_sink(
    cfg: Arc<AppConfig>,
    task_id: Uuid,
) -> impl Sink<Vec<String>, Error = anyhow::Error> + Send {
    let vicky_client_task = HttpClient::new(cfg.clone());

    futures_util::sink::unfold(
        vicky_client_task,
        move |mut http_client, lines: Vec<String>| async move {
            let response = http_client
                .do_request::<_, ()>(
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
        },
    )
}

use crate::cli::{AppContext, TaskData, TasksArgs};
use log::debug;
use reqwest::Method;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use vickyctllib::api::HttpClient;
use yansi::Paint;

use crate::error::Error;
use crate::http_client::{prepare_client, print_http};
use crate::{humanize};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "result")]
pub enum TaskResult {
    Success,
    Error,
}

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
#[serde(tag = "state")]
pub enum TaskStatus {
    New,
    Running,
    Finished(TaskResult),
}

type FlakeURI = String;

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub struct FlakeRef {
    pub flake: FlakeURI,
    pub args: Vec<String>,
}

type Maow = u8; // this does not exist. look away. it's all for a reason.

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub locks: Vec<Maow>,
    pub flake_ref: FlakeRef,
    pub features: Vec<String>,
}

pub fn show_tasks(tasks_args: &TasksArgs, client: &HttpClient) -> Result<(), Error> {
    if tasks_args.ctx.humanize {
        humanize::ensure_jless("tasks")?;
    }

    let (client, vicky_url) = prepare_client(auth_state)?;
    let request = client
        .get(format!("{}/{}", vicky_url, "api/v1/tasks"))
        .build()?;
    let response = client.execute(request)?.error_for_status()?;

    let text = response.text()?;
    debug!("got response from server, presenting output");
    humanize::handle_user_response(&tasks_args.ctx, &text)?;
    Ok(())
}

impl TaskData {
    fn to_json(&self) -> serde_json::Value {
        let locks: serde_json::Value = self
            .lock_name
            .iter()
            .zip(self.lock_type.iter())
            .map(|(name, ty)| {
                json!({
                    "name": name,
                    "type": ty
                })
            })
            .collect();
        json!({
            "display_name": self.name,
            "flake_ref": {
                "flake": self.flake_url,
                "args": self.flake_arg
            },
            "locks": locks,
            "features": self.features
        })
    }
}

#[derive(Deserialize, Serialize)]
struct RoTaskStatus {
    state: String,
}

#[derive(Deserialize, Serialize)]
struct RoTaskCreate {
    id: String,
    status: RoTaskStatus,
}

pub fn create_task(task_data: &TaskData, ctx: &AppContext, client: &HttpClient) -> Result<(), Error> {
    let request = client.create_request(Method::POST, "api/v1/tasks")?
        .body(task_data.to_json().to_string())
        .build()?;

    let response = request.send()?
        .error_for_status()
        .map_err(|e| (e, "Task couldn't be scheduled.".to_string()))?;

    let status = response.status();
    let text = response.text()?;
    let pretty_json: RoTaskCreate = serde_json::de::from_str(&text)?;
    if ctx.humanize {
        print_http(
            Some(status),
            &format!(
                "Task was scheduled under id {}. State: {}",
                pretty_json.id.bright_blue(),
                pretty_json.status.state.bright_yellow()
            ),
        );
    } else {
        println!("{}", serde_json::ser::to_string(&pretty_json)?);
    }
    Ok(())
}

pub fn claim_task(features: &[String], ctx: &AppContext, client: &HttpClient) -> Result<(), Error> {
    let (client, vicky_url) = prepare_client(auth_state)?;
    let data: serde_json::Value = json!({
        "features": features
    });
    let request = client
        .post(format!("{}/{}", vicky_url, "api/v1/tasks/claim"))
        .body(data.to_string())
        .build()?;

    let response = client
        .execute(request)?
        .error_for_status()
        .map_err(|e| (e, "Task couldn't be claimed".to_string()))?;

    let status = response.status();
    let text = response.text()?;
    let pretty_json: serde_json::Value = serde_json::de::from_str(&text)?;
    let pretty_data = serde_json::ser::to_string(&pretty_json)?;
    if ctx.humanize {
        print_http(
            Some(status),
            &format!("Task was claimed: {}", pretty_data.bright_blue()),
        );
    } else {
        println!("{}", pretty_data);
    }
    Ok(())
}

pub fn finish_task(id: &Uuid, status: &String, ctx: &AppContext, client: &HttpClient) -> Result<(), Error> {
    let (client, vicky_url) = prepare_client(auth_state)?;
    let data = json!({
        "result": status
    });
    let request = client
        .post(format!(
            "{}/{}/{}/{}",
            vicky_url, "api/v1/tasks", id, "finish"
        ))
        .body(data.to_string())
        .build()?;

    let response = client
        .execute(request)?
        .error_for_status()
        .map_err(|e| (e, "Task couldn't be finished".to_string()))?;

    let status = response.status();
    let text = response.text()?;
    let pretty_json: serde_json::Value = serde_json::de::from_str(&text)?;
    let pretty_data = serde_json::ser::to_string(&pretty_json)?;
    if ctx.humanize {
        print_http(
            Some(status),
            &format!(
                "Task was finished. Finished Task: {}",
                pretty_data.bright_blue()
            ),
        );
    } else {
        println!("{}", pretty_data);
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::TaskData;
    use serde_json::json;

    #[test]
    fn test_empty_task_data_to_json() {
        let data = TaskData {
            name: "".to_string(),
            lock_name: vec![],
            lock_type: vec![],
            flake_url: "".to_string(),
            flake_arg: vec![],
            features: vec![],
        };

        let should_be = json!({
            "display_name": "",
            "locks": [],
            "flake_ref": {
                "flake": "",
                "args": []
            },
            "features": []
        });

        assert_eq!(data.to_json(), should_be);
    }

    #[test]
    fn test_full_task_data_to_json() {
        let data = TaskData {
            name: "deployment 5".to_string(),
            lock_name: vec![
                "first".to_string(),
                "second".to_string(),
                "third".to_string(),
            ],
            lock_type: vec!["WRITE".to_string(), "WRITE".to_string(), "READ".to_string()],
            flake_url: "github:wobcom/vicky".to_string(),
            flake_arg: vec!["flaked".to_string(), "really!".to_string()],
            features: vec![
                "feat1".to_string(),
                "big_cpu".to_string(),
                "huge_cpu".to_string(),
                "gigantonormous_gpu".to_string(),
            ],
        };

        let should_be = json!({
            "display_name": "deployment 5",
            "locks": [
                {
                    "name": "first",
                    "type": "WRITE",
                },
                {
                    "name": "second",
                    "type": "WRITE",
                },
                {
                    "name": "third",
                    "type": "READ",
                }
            ],
            "flake_ref": {
                "flake": "github:wobcom/vicky",
                "args": [ "flaked", "really!" ]
            },
            "features": [ "feat1", "big_cpu", "huge_cpu", "gigantonormous_gpu" ]
        });

        assert_eq!(data.to_json(), should_be);
    }
}

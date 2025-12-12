use crate::cli::{AppContext, TaskData, TasksArgs};
use crate::error::Error;
use crate::http_client::{prepare_client, print_http};
use crate::humanize;
use log::debug;
use serde::{Deserialize, Serialize};
use serde_json::json;
use uuid::Uuid;
use vickylib::database::entities::task::{FlakeRef, TaskResult, TaskStatus};
use vickylib::database::entities::Lock;
use yansi::Paint;

#[derive(Debug, PartialEq, Serialize, Deserialize, Clone)]
pub struct Task {
    pub id: Uuid,
    pub display_name: String,
    pub status: TaskStatus,
    pub locks: Vec<Lock>,
    pub flake_ref: FlakeRef,
    pub features: Vec<String>,
}

pub fn show_tasks(tasks_args: &TasksArgs) -> Result<(), Error> {
    if tasks_args.ctx.humanize {
        humanize::ensure_jless("tasks")?;
    }

    let client = prepare_client(&tasks_args.ctx)?;
    let request = client
        .get(format!("{}/api/v1/tasks", tasks_args.ctx.vicky_url))
        .build()?;
    let response = client.execute(request)?.error_for_status()?;

    let text = response.text()?;
    debug!("got response from server, presenting output");
    humanize::handle_user_response(&tasks_args.ctx, &text)?;
    Ok(())
}

impl TaskData {
    pub fn to_json(&self) -> serde_json::Value {
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
            "features": self.features,
            "needs_confirmation": self.needs_confirmation,
            "group": self.group,
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

pub fn create_task(task_data: &TaskData, ctx: &AppContext) -> Result<(), Error> {
    let client = prepare_client(ctx)?;
    let request = client
        .post(format!("{}/api/v1/tasks", ctx.vicky_url))
        .body(task_data.to_json().to_string())
        .build()?;

    let response = client
        .execute(request)?
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

pub fn claim_task(features: &[String], ctx: &AppContext) -> Result<(), Error> {
    let client = prepare_client(ctx)?;
    let data: serde_json::Value = json!({
        "features": features
    });
    let request = client
        .post(format!("{}/api/v1/tasks/claim", ctx.vicky_url))
        .json(&data)
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
        println!("{pretty_data}");
    }
    Ok(())
}

pub fn finish_task(id: &Uuid, status: TaskResult, ctx: &AppContext) -> Result<(), Error> {
    let client = prepare_client(ctx)?;
    let data = json!({
        "result": status
    });
    let request = client
        .post(format!("{}/api/v1/tasks/{id}/finish", ctx.vicky_url))
        .json(&data)
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
        println!("{pretty_data}");
    }
    Ok(())
}

pub fn confirm_task(id: &Uuid, ctx: &AppContext) -> Result<(), Error> {
    let client = prepare_client(ctx)?;
    let request = client
        .post(format!("{}/api/v1/tasks/{id}/confirm", ctx.vicky_url))
        .build()?;

    let response = client
        .execute(request)?
        .error_for_status()
        .map_err(|e| (e, "Task couldn't be confirmed".to_string()))?;

    let status = response.status();
    let text = response.text()?;

    if text.trim().is_empty() {
        if ctx.humanize {
            print_http(Some(status), &format!("Task {id} confirmed."));
        } else {
            println!();
        }
        return Ok(());
    }

    if ctx.humanize {
        if let Ok(task) = serde_json::de::from_str::<Task>(&text) {
            print_http(
                Some(status),
                &format!(
                    "Task {} confirmed. New status: {:?}",
                    task.id.to_string().bright_blue(),
                    task.status
                ),
            );
            return Ok(());
        }
    }

    match serde_json::de::from_str::<serde_json::Value>(&text) {
        Ok(pretty_json) => {
            let pretty_data = serde_json::ser::to_string(&pretty_json)?;
            if ctx.humanize {
                print_http(
                    Some(status),
                    &format!("Task was confirmed: {}", pretty_data.bright_blue()),
                );
            } else {
                println!("{pretty_data}");
            }
        }
        Err(_) => {
            if ctx.humanize {
                print_http(
                    Some(status),
                    &format!("Task was confirmed: {}", text.bright_blue()),
                );
            } else {
                println!("{text}");
            }
        }
    };

    Ok(())
}

#[cfg(test)]
mod tests {
    use crate::cli::TaskData;
    use serde_json::json;
    use vickylib::database::entities::LockKind;

    #[test]
    fn test_empty_task_data_to_json() {
        let data = TaskData {
            name: "".to_string(),
            lock_name: vec![],
            lock_type: vec![],
            flake_url: "".to_string(),
            flake_arg: vec![],
            features: vec![],
            group: None,
            needs_confirmation: false,
        };

        let should_be = json!({
            "display_name": "",
            "locks": [],
            "flake_ref": {
                "flake": "",
                "args": []
            },
            "features": [],
            "needs_confirmation": false,
            "group": null,
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
            lock_type: vec![LockKind::Write, LockKind::Write, LockKind::Read],
            flake_url: "github:wobcom/vicky".to_string(),
            flake_arg: vec!["flaked".to_string(), "really!".to_string()],
            features: vec![
                "feat1".to_string(),
                "big_cpu".to_string(),
                "huge_cpu".to_string(),
                "gigantonormous_gpu".to_string(),
            ],
            group: None,
            needs_confirmation: true,
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
            "features": [ "feat1", "big_cpu", "huge_cpu", "gigantonormous_gpu" ],
            "needs_confirmation": true,
            "group": null,
        });

        assert_eq!(data.to_json(), should_be);
    }
}

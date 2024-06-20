use std::fs::{self, File};
use std::path::PathBuf;

use clap::Parser;
use figment::Figment;
use figment::providers::{Env, Format, Json};
use serde::{Deserialize, Serialize};

use account::{login, show};
use cli::AccountCommands;

use crate::cli::{Cli, TaskCommands};
use crate::tasks::{claim_task, create_task, finish_task};

mod account;
mod cli;
mod error;
mod http_client;
mod humanize;
mod locks;
mod tasks;
mod tui;

#[derive(Serialize, Deserialize, Debug)]
pub struct EnvConfig {
    issuer_url: String,
    url: String,
    client_id: String,
    client_secret: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileConfig {
    issuer_url: String,
    vicky_url: String,
    client_id: String,
    refresh_token: String,
}

#[derive(Debug)]
pub enum AuthState {
    EnvironmentAuthenticated(EnvConfig),
    FileAuthenticated(FileConfig),
    Unauthenticated,
}

impl FileConfig {
    fn save(&self) -> Result<(), anyhow::Error> {
        let mut path: PathBuf = dirs::config_dir().unwrap();

        path.push("vickyctl");
        fs::create_dir_all(path.clone())?;

        path.push("account.json");
        let config_file = File::create(path)?;

        serde_json::to_writer_pretty(config_file, self)?;
        Ok(())
    }
}

fn main() {
    let cli = Cli::parse();

    let mut account_config_path: PathBuf = dirs::config_dir().unwrap();
    account_config_path.push("vickyctl/account.json");

    let account_config: Option<FileConfig> = Figment::new()
        .merge(Json::file(account_config_path))
        .extract()
        .ok();

    let env_config: Option<EnvConfig> =
        Figment::new().merge(Env::prefixed("VICKY_")).extract().ok();

    let mut auth_state = AuthState::Unauthenticated;

    if let Some(env_config) = env_config {
        auth_state = AuthState::EnvironmentAuthenticated(env_config);
    } else if let Some(account_config) = account_config {
        auth_state = AuthState::FileAuthenticated(account_config);
    }

    let error: Result<_, _> = match cli {
        Cli::Task(task_args) => match task_args.commands {
            TaskCommands::Create(task_data) => create_task(&task_data, &task_args.ctx, &auth_state),
            TaskCommands::Claim { features } => claim_task(&features, &task_args.ctx, &auth_state),
            TaskCommands::Finish { id, status } => {
                finish_task(&id, &status, &task_args.ctx, &auth_state)
            }
        },
        Cli::Tasks(tasks_args) => tasks::show_tasks(&tasks_args, &auth_state),
        Cli::Locks(locks_args) => tui::show_locks(&locks_args, &auth_state),
        Cli::Resolve(_) => tui::resolve_lock(&auth_state),

        Cli::Account(account_args) => match account_args.commands {
            AccountCommands::Show => show(&auth_state).map_err(crate::error::Error::from),
            AccountCommands::Login {
                vicky_url,
                client_id,
                issuer_url,
            } => login(vicky_url, issuer_url, client_id).map_err(crate::error::Error::from),
        },
    };

    match error {
        Ok(()) => {}
        Err(e) => println!("{}", e),
    }
}

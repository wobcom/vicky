mod cli;
mod error;
mod http_client;
mod humanize;
mod locks;
mod tasks;
mod tui;
mod account;

use std::fs::{File, self};
use std::path::PathBuf;

use crate::cli::{Cli, TaskCommands};
use crate::tasks::{claim_task, create_task, finish_task};
use account::{login, show};
use clap::Parser;
use cli::AccountCommands;
use figment::Figment;
use figment::providers::{Env, Json, Format};
use vickyctllib::api::{FileConfig, EnvConfig, ConfigState, HttpClient};


fn main() {
    let cli = Cli::parse();

    let mut account_config_path:PathBuf = dirs::config_dir().unwrap();
    account_config_path.push("vickyctl/account.json");

    let account_config: Option<FileConfig> = Figment::new()
        .merge(Json::file(account_config_path))
        .extract().ok();

    let env_config: Option<EnvConfig> = Figment::new()
        .merge(Env::prefixed("VICKY_"))
        .extract().ok();

    
    let mut config_state = ConfigState::Unauthenticated;

    if let Some(env_config) = env_config {
        config_state = ConfigState::EnvironmentAuthenticated(env_config);
    } else if let Some(account_config) = account_config {
        config_state = ConfigState::FileAuthenticated(account_config);
    }

    let user_agent = format!("vickyctl/{}", env!("CARGO_PKG_VERSION"));
    let http_client = HttpClient::new(&config_state, user_agent);

    let error: Result<_, _> = match cli {
        Cli::Task(task_args) => match task_args.commands {
            TaskCommands::Create(task_data) => create_task(&task_data, &task_args.ctx, &config_state),
            TaskCommands::Claim { features } => claim_task(&features, &task_args.ctx, &config_state),
            TaskCommands::Finish { id, status } => finish_task(&id, &status, &task_args.ctx, &config_state),
        },
        Cli::Tasks(tasks_args) => tasks::show_tasks(&tasks_args, &config_state),
        Cli::Locks(locks_args) => tui::show_locks(&locks_args, &config_state),
        Cli::Resolve(_) => tui::resolve_lock(&config_state),

        Cli::Account(account_args) => match account_args.commands {
            AccountCommands::Show => show(&config_state).map_err(crate::error::Error::from),
            AccountCommands::Login{ vicky_url, client_id, issuer_url} => login( vicky_url, issuer_url, client_id).map_err(crate::error::Error::from)
        }

    };

    match error {
        Ok(()) => {}
        Err(e) => println!("{}", e),
    }
}

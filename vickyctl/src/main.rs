mod cli;
mod error;
mod http_client;
mod humanize;
mod locks;
mod tasks;

use crate::cli::{Cli, TaskCommands};
use crate::tasks::{claim_task, create_task, finish_task};
use clap::Parser;

fn main() {
    let cli = Cli::parse();

    let error: Result<_, _> = match cli {
        Cli::Task(task_args) => match task_args.commands {
            TaskCommands::Create(task_data) => create_task(&task_data, &task_args.ctx),
            TaskCommands::Claim { features } => claim_task(&features, &task_args.ctx),
            TaskCommands::Finish { id, status } => finish_task(&id, &status, &task_args.ctx),
        },
        Cli::Tasks(tasks_args) => tasks::show_tasks(&tasks_args),
        Cli::Locks(locks_args) => locks::show_locks(&locks_args),
        Cli::Resolve(resolve_args) => locks::resolve_lock(&resolve_args),
    };

    match error {
        Ok(()) => {}
        Err(e) => println!("{}", e),
    }
}

mod http_client;
mod humanize;
mod tasks;
mod error;
mod locks;

use crate::tasks::{claim_task, create_task, finish_task};
use clap::{Args, Parser, Subcommand};
use uuid::Uuid;
use yansi::Paint;

#[derive(Parser, Debug, Clone)]
struct AppContext {
    #[clap(env)]
    vicky_url: String,

    #[clap(env)]
    vicky_token: String,

    #[clap(long)]
    humanize: bool,
}

#[derive(Parser, Debug, Clone)]
struct TaskData {
    #[clap(short, long)]
    name: String,
    #[clap(long)]
    lock_name: Vec<String>,
    #[clap(long)]
    lock_type: Vec<String>,
    #[clap(long)]
    flake_url: String,
    #[clap(long)]
    flake_arg: Vec<String>,
    #[clap(long)]
    features: Vec<String>,
}

#[derive(Subcommand, Debug)]
enum TaskCommands {
    Create(TaskData),
    // Logs, // TODO: could add this later
    Claim { features: Vec<String> },
    Finish { id: Uuid, status: String },
}

#[derive(Args, Debug)]
#[command(version, about = "Manage tasks on the vicky delegation server", long_about = None)]
struct TaskArgs {
    #[command(subcommand)]
    commands: TaskCommands,

    #[command(flatten)]
    ctx: AppContext,
}

#[derive(Args, Debug)]
#[command(version, about = "Show all tasks vicky is managing", long_about = None)]
struct TasksArgs {
    #[command(flatten)]
    ctx: AppContext,
}

#[derive(Args, Debug)]
#[command(version, about = "Show all poisoned locks vicky is managing", long_about = None)]
struct LocksArgs {
    #[command(flatten)]
    ctx: AppContext,
    #[clap(long)]
    active: bool,
    #[clap(long)]
    poisoned: bool,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Cli {
    Task(TaskArgs),
    Tasks(TasksArgs),
    Locks(LocksArgs)
}

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
    };

    match error {
        Ok(()) => {}
        Err(e) => println!("{} {}", "Error:".bright_red(), e.bright_red()),
    }
}

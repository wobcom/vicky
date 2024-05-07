mod tasks;
mod http_client;
mod humanize;

use clap::{Args, Parser, Subcommand};
use log::error;
use uuid::Uuid;

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
    Logs,
    Claim {
        id: Uuid
    },
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

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
enum Cli {
    Task(TaskArgs),
    Tasks(TasksArgs),
}

fn main() {
    env_logger::builder()
        .format_timestamp(None)
        .format_target(false)
        .init();
    let cli = Cli::parse();

    let error: Result<_, _> = match cli {
        Cli::Task(task_args) => {
            match task_args.commands {
                TaskCommands::Create(task_data) => { todo!() }
                TaskCommands::Logs => { todo!() }
                TaskCommands::Claim { id } => { todo!() }
                TaskCommands::Finish { id, status } => { todo!() }
            }
        }
        Cli::Tasks(tasks_args) => {
            tasks::show_tasks(&tasks_args)
        }
    };

    match error {
        Ok(()) => {},
        Err(e) => error!("{e}"),
    }
}

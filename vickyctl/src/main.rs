mod tasks;

use clap::{Args, Parser, Subcommand};
use uuid::Uuid;

#[derive(Parser, Debug, Clone)]
struct AppContext {
    #[clap(env)]
    vicky_url: String,

    #[clap(env)]
    token: String,
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
enum Cli {
    Task(TaskArgs),
    Tasks(TasksArgs),
}

fn main() {
    let cli = Cli::parse();

    match cli {
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
    }
}

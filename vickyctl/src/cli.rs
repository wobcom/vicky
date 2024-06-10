use clap::{Args, Parser, Subcommand};
use uuid::Uuid;

// TODO: Add abouts to arguments
#[derive(Parser, Debug, Clone)]
pub struct AppContext {
    #[clap(env)]
    pub vicky_url: String,

    #[clap(env)]
    pub vicky_token: String,

    #[clap(long)]
    pub humanize: bool,
}

#[derive(Parser, Debug, Clone)]
pub struct TaskData {
    #[clap(short, long)]
    pub name: String,
    #[clap(long)]
    pub lock_name: Vec<String>,
    #[clap(long)]
    pub lock_type: Vec<String>,
    #[clap(long)]
    pub flake_url: String,
    #[clap(long)]
    pub flake_arg: Vec<String>,
    #[clap(long)]
    pub features: Vec<String>,
}

#[derive(Subcommand, Debug)]
pub enum TaskCommands {
    Create(TaskData),
    // Logs, // TODO: could add this later
    Claim { features: Vec<String> },
    Finish { id: Uuid, status: String },
}

#[derive(Args, Debug)]
#[command(version, about = "Manage tasks on the vicky delegation server", long_about = None)]
pub struct TaskArgs {
    #[command(subcommand)]
    pub commands: TaskCommands,

    #[command(flatten)]
    pub ctx: AppContext,
}

#[derive(Args, Debug)]
#[command(version, about = "Show all tasks vicky is managing", long_about = None)]
pub struct TasksArgs {
    #[command(flatten)]
    pub ctx: AppContext,
}

#[derive(Args, Debug)]
#[command(version, about = "Show all poisoned locks vicky is managing", long_about = None)]
pub struct LocksArgs {
    #[command(flatten)]
    pub ctx: AppContext,
    #[clap(long)]
    pub active: bool,
    #[clap(long)]
    pub poisoned: bool,
}

#[derive(Args, Debug)]
#[command(version, about = "Show all poisoned locks vicky is managing", long_about = None)]
pub struct ResolveArgs {
    #[command(flatten)]
    pub ctx: AppContext,
    #[clap(long)]
    pub all: bool,
    #[clap(long,short)]
    pub task_id: Option<String>,
}

#[derive(Parser, Debug)]
#[command(version, about, long_about = None)]
pub enum Cli {
    Task(TaskArgs),
    Tasks(TasksArgs),
    Locks(LocksArgs),
    Resolve(ResolveArgs),
}


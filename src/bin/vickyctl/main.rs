use anyhow::Result;
use clap::{Parser, Subcommand};
use etcd_client::Client;
use serde_yaml::to_string;
use vickylib::manifests::NodeManifest;
use std::{env, fs, thread};


#[derive(Subcommand)]
pub enum Commands {
    /// Project commands
    #[clap(name = "node")]
    Node(Node),

    // Other subcommand groups can go here
}

#[derive(Parser)]
pub struct Node {
         #[structopt(subcommand)]
        pub node_commands: NodeCommands,
}


#[derive(Subcommand)]
pub enum NodeCommands {
    Apply {
        path: String,
    },
    Delete {
        path: String,
    },
}

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Args {
    #[command(subcommand)]
    command: Commands,
}

#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut client = Client::connect(["localhost:2379"], None).await?;


    match args.command {
        Commands::Node(args) => match args.node_commands {
            NodeCommands::Apply { path } => {
                let yaml = fs::read_to_string(path).unwrap();
                let node_manifest: NodeManifest = serde_yaml::from_str(&yaml)?;
                let node_manifest_string = to_string(&node_manifest)?;

                let node_key = format!("vicky.wobcom.de/node/{}", node_manifest.name);
                client.put(node_key, node_manifest_string, None).await?;
            },
            NodeCommands::Delete { path } => {
                let yaml = fs::read_to_string(path).unwrap();
                let node_manifest: NodeManifest = serde_yaml::from_str(&yaml)?;

                let node_key = format!("vicky.wobcom.de/node/{}", node_manifest.name);
                client.delete(node_key, None).await?;
            },
        }
    }

    Ok(())
}
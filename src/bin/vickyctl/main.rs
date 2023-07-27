use anyhow::Result;
use aws_config::provider_config::ProviderConfig;
use aws_sdk_s3::{config::{Credentials, Region}, primitives::ByteStream};
use clap::{Parser, Subcommand};
use etcd_client::Client;
use serde_yaml::to_string;
use uuid::Uuid;
use std::{fs, path::Path};
use vickylib::documents::DeviceManifest;

#[derive(Subcommand)]
pub enum Commands {
    /// Project commands
    #[clap(name = "node")]
    Node(Node),

    #[clap(name = "config-bundle")]
    ConfigBundle(ConfigBundle),
}

#[derive(Parser)]
pub struct Node {
    #[structopt(subcommand)]
    pub node_commands: NodeCommands,
}

#[derive(Subcommand)]
pub enum NodeCommands {
    Apply { path: String },
    Delete { path: String },
}

#[derive(Parser)]
pub struct ConfigBundle {
    #[structopt(subcommand)]
    pub config_bundle_commands: ConfigBundleCommands,
}

#[derive(Subcommand)]
pub enum ConfigBundleCommands {
    Add { path: String },
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

    let mut etcd_client = Client::connect(["127.0.0.1:2379"], None).await?;

    let creds = Credentials::new(
        "minio",
        "aichudiKohr6aithi4ahh3aeng2eL7xo",
        None,
        None,
        "static",
    );

    let config = aws_config::from_env()
        .endpoint_url("http://localhost:9000")
        .credentials_provider(creds)
        .region(Region::new("us-east-1"))
        .load()
        .await;

    let s3_client = aws_sdk_s3::Client::new(&config);

    match args.command {
        Commands::ConfigBundle(args) => match args.config_bundle_commands {
            ConfigBundleCommands::Add { path } => {

                let file_id = Uuid::new_v4();
                let path = Path::new(&path);

                // We are currently rocking a Minio backend, which does not like files within the root folder. Therefore,
                // we use the bucket name to nest the files into a folder.
                let bucket_file_name = format!("vicky-configs/{}.{}", file_id, path.extension().unwrap().to_str().unwrap());

                let body = ByteStream::read_from()
                    .path(path)
                    .buffer_size(2048)
                    .build()
                    .await?;
                
                s3_client
                    .put_object()
                    .bucket("vicky-configs")
                    .key(&bucket_file_name)
                    .body(body)
                    .send()
                    .await?;

                println!("{}", bucket_file_name)

            },
        }
        
        Commands::Node(args) => match args.node_commands {
            NodeCommands::Apply { path } => {
                let yaml = fs::read_to_string(path).unwrap();
                let node_manifest: DeviceManifest = serde_yaml::from_str(&yaml)?;
                let node_manifest_string = to_string(&node_manifest)?;

                let node_key = format!("vicky.wobcom.de/node/manifest/{}", node_manifest.name);
                etcd_client
                    .put(node_key, node_manifest_string, None)
                    .await?;
            }
            NodeCommands::Delete { path } => {
                let yaml = fs::read_to_string(path).unwrap();
                let node_manifest: DeviceManifest = serde_yaml::from_str(&yaml)?;

                let node_key = format!("vicky.wobcom.de/node/manifest/{}", node_manifest.name);
                etcd_client.delete(node_key, None).await?;
            }
        },
    }

    Ok(())
}

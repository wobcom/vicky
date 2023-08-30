use anyhow::Result;
use aws_config::provider_config::ProviderConfig;
use aws_sdk_s3::{config::{Credentials, Region}, primitives::ByteStream};
use clap::{Parser, Subcommand};
use etcd_client::Client;
use serde_yaml::to_string;
use uuid::Uuid;
use std::{fs, path::Path};

#[derive(Subcommand)]
pub enum Commands {

    #[clap(name = "config-bundle")]
    ConfigBundle(ConfigBundle),
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
     
    }

    Ok(())
}

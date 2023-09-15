use std::collections::HashMap;

use auth::User;
use aws_sdk_s3::config::{Credentials, Region};
use log::info;

use rand::Rng;
use rocket::fairing::AdHoc;
use rocket::routes;
use rocket_oauth2::OAuth2;
use serde::Deserialize;
use vickylib::etcd::election::{NodeId, Election};
use vickylib::logs::LogDrain;
use vickylib::s3::client::S3Client;

use crate::tasks::{tasks_claim, tasks_finish, tasks_get_machine, tasks_get_user, tasks_add, tasks_get_logs, tasks_put_logs};
use crate::auth::{github_login, github_callback, logout, GitHubUserInfo};
use crate::user::{get_user};

mod tasks;
mod auth;
mod user;



#[derive(Deserialize)]
pub struct Config {
    users: HashMap<String, User>,
    machines: Vec<String>,
}


#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut rng = rand::thread_rng();

    let etcd_client = etcd_client::Client::connect(["localhost:2379"], None).await?;

    let aws_conf = aws_config::from_env().endpoint_url("http://localhost:9000").credentials_provider(Credentials::new("minio", "aichudiKohr6aithi4ahh3aeng2eL7xo", None, None, "example")).region(Region::new("us-east-1")).load().await;
    let s3_client = aws_sdk_s3::Client::new(&aws_conf);
    let s3_ext_client_drain = S3Client::new(s3_client.clone(), String::from("vicky-logs"));
    let s3_ext_client = S3Client::new(s3_client, String::from("vicky-logs"));

    let node_id: NodeId = format!("node_{}", rng.gen::<i32>()).to_string();
    info!("Generated unique node id as {}", node_id);

    let mut election = Election::new(&etcd_client, node_id);
    election.keep_alive();

    election.elect().await?;
    info!("Leader election won, we are now the leader!");

    let log_drain = LogDrain::new(s3_ext_client_drain);

    let _rocket = rocket::build()
        .manage(etcd_client)
        .manage(s3_ext_client)
        .manage(log_drain)
        .attach(OAuth2::<GitHubUserInfo>::fairing("github"))
        .attach(AdHoc::config::<Config>())
        .mount("/api/v1/user", routes![get_user])
        .mount("/api/v1/auth", routes![github_login, github_callback, logout])
        .mount("/api/v1/tasks", routes![tasks_get_machine, tasks_get_user, tasks_claim, tasks_finish, tasks_add, tasks_get_logs, tasks_put_logs])
        .launch()
        .await?;

    Ok(())
}

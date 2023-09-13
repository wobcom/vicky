use std::collections::HashMap;

use auth::User;
use etcd_client::{Client};
use log::info;

use rand::Rng;
use rocket::fairing::AdHoc;
use rocket::routes;
use rocket_oauth2::OAuth2;
use serde::Deserialize;
use vickylib::etcd::election::{NodeId, Election};

use crate::tasks::{tasks_claim, tasks_finish, tasks_get_machine, tasks_get_user, tasks_add, tasks_get_logs};
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

    let client = Client::connect(["localhost:2379"], None).await?;

    let node_id: NodeId = format!("node_{}", rng.gen::<i32>()).to_string();
    info!("Generated unique node id as {}", node_id);

    let mut election = Election::new(&client, node_id);
    election.keep_alive();

    election.elect().await?;
    info!("Leader election won, we are now the leader!");


    let _rocket = rocket::build()
        .manage(client)
        .attach(OAuth2::<GitHubUserInfo>::fairing("github"))
        .attach(AdHoc::config::<Config>())
        .mount("/api/v1/user", routes![get_user])
        .mount("/api/v1/auth", routes![github_login, github_callback, logout])
        .mount("/api/v1/tasks", routes![tasks_get_machine, tasks_get_user, tasks_claim, tasks_finish, tasks_add, tasks_get_logs])
        .launch()
        .await?;

    Ok(())
}

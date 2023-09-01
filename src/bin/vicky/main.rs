#[macro_use] extern crate rocket;

use etcd_client::{Client};
use log::info;

use rand::Rng;
use rocket::routes;
use vickylib::etcd::election::{NodeId, Election};

use crate::routes::{tasks_claim, tasks_finish, tasks_get, tasks_add};

mod routes;

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
        .mount("/api/v1/tasks", routes![tasks_get, tasks_claim, tasks_finish, tasks_add])
        .launch()
        .await?;

    Ok(())
}

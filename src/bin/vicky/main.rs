use etcd_client::{Client, Error};
use log::info;

use vickylib::etcd::{Election, NodeId};
use rand::Rng;
use std::{thread, time};

mod healthchecker;

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    env_logger::init();

    let mut rng = rand::thread_rng();

    let client = Client::connect(["localhost:2379"], None).await?;

    let node_id: NodeId = format!("node_{}", rng.gen::<i32>()).to_string();

    info!("Generated unique node id as {}", node_id);

    let mut election = Election::new(client.clone(), node_id);
    election.keep_alive();
    
    election.elect().await?;
    info!("Leader election won, we are now the leader!");

    let mut hs = healthchecker::Healthchecker::new(client.clone());

    hs.check_nodes().await?;

    loop {
        thread::sleep(time::Duration::from_secs(5))
    }

    Ok(())
}

use etcd_client::{Client};
use log::info;

use rand::Rng;
use vickylib::etcd::election::{NodeId, Election};
use std::{thread, time};

mod healthchecker;
mod operator;

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

    let mut hs = healthchecker::Healthchecker::new(&client);
    let mut op = operator::Operator::new(&client);

    loop {
        hs.check_nodes().await?;

        op.evaluate_tasks().await?;
        thread::sleep(time::Duration::from_secs(10))
    }

}

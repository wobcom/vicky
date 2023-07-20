use etcd_client::{Client, GetOptions};
use log::{info, debug};
use vickylib::{etcd::ClientExt, manifests::NodeManifest};



pub struct Healthchecker {
    c: Client
}


impl Healthchecker {

    pub fn new(c: Client) -> Self {
        Healthchecker { c }
    }

    pub async fn check_nodes(&mut self) -> anyhow::Result<()>{

        let nodes: Vec<NodeManifest> = self.c.get_yaml_list("vicky.wobcom.de/node".to_string(), Some(GetOptions::new().with_prefix())).await?;

        debug!("{:?}", nodes);

        for n in nodes {
            info!("Checking {}", n.name)
        }

        return Ok(())
    }

}

use etcd_client::{Client, GetOptions, KvClient};
use log::{info, warn};
use vickylib::{documents::{DeviceManifest, DeviceHealth, DeviceTypeEnum}, etcd::client::ClientExt};

trait DeviceOperator {
    fn evaluate_tasks(&self, device_manifest: &DeviceManifest, device_health: &DeviceHealth) -> Vec<()>;
}

pub struct DummyOperator {

}

impl DeviceOperator for DummyOperator {
    fn evaluate_tasks(&self, _device_manifest: &DeviceManifest, _device_health: &DeviceHealth) -> Vec<()> {
        vec![]
    }
}

/**
 * Operator does the heavy lifting within Vicky.
 * It manages the tasks which are needed to reach the desired state of each node.
 * Therefore this module decides what to do in each case.
 * 
 * We also implemented different operators per device type, since the flow of a NixOS machine may
 * be quite different from a Junos router.
 * The operator scrapes and delivers all information needed for a decision to the different DeviceOperators.
 * DeviceOperators returns a set of tasks. The DeviceOperator does not need to check if those tasks are already 
 * queued, we will remove duplicate tasks. We also provide seperate functionalty for scheduled tasks later on.
 *  
 */
pub struct Operator {
    kv_client: KvClient,

    dummy_operator: DummyOperator,
}


impl Operator {

    pub fn new(c: &Client) -> Self {

        let kv_client = c.kv_client();

        let dummy_operator = DummyOperator {};

        Operator { 
            kv_client, 

            dummy_operator,
        }
    }

    fn get_device_operator(&self, device_type: &DeviceTypeEnum) -> &impl DeviceOperator {

        match device_type {
            DeviceTypeEnum::Dummy => {
                &self.dummy_operator
            },
        }

    }

    pub async fn evaluate_tasks(&mut self) -> anyhow::Result<()>{

        let node_manifests: Vec<DeviceManifest> = self.kv_client.get_yaml_list("vicky.wobcom.de/node/manifest".to_string(), Some(GetOptions::new().with_prefix())).await?;
        let node_healths: Vec<DeviceHealth> = self.kv_client.get_yaml_list("vicky.wobcom.de/node/health".to_string(), Some(GetOptions::new().with_prefix())).await?;

        for node in node_manifests {
            
            let node_health = node_healths.iter().find(|health| health.name == node.name);

            match node_health {
                Some(h) => {
                    let operator = self.get_device_operator(&node.device_type);
                    info!("Evaluating tasks for {}", node.name);
                    operator.evaluate_tasks(&node, h);
                },
                None => {
                    warn!("Node {} has no attached health manifest, ignoring...", node.name);
                },
            }
        }
        Ok(())

    }

}

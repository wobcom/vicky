use anyhow::Result;
use etcd_client::{Client, GetOptions, KvClient};
use log::{info, debug};
use vickylib::{documents::{DeviceManifest, DeviceHealth, DeviceHealthEnum, DeviceTypeEnum}, etcd::client::ClientExt};

trait DeviceHealthchecker {
    fn get_device_health(&self, device: &DeviceManifest) -> Result<DeviceHealth>;
}


/**
 * DummyHealthchecker is - as named - a dummy implementation. 
 * Those are scattered around the application and should work together.
 * There are no real devices involved.
 */
pub struct DummyHealthchecker {

}

impl DeviceHealthchecker for DummyHealthchecker {
    fn get_device_health(&self, device: &DeviceManifest) -> Result<DeviceHealth> {
        Ok(DeviceHealth {
            name: device.name.clone(),
            status: DeviceHealthEnum::Operational,
            info: vec![],
            warnings: vec![],
            errors: vec![],
        })
    }
}

/**
 * Healthchecker maintains a record over the "externally monitored" state of a node.
 * A node should be reachable at all times and also should have a high CPU, memory or storage load.
 * This would lead to a non-operational state which can be taken into account within the operator.
 * 
 * We need to implement a special health checker for each and every device type. 
 * Therefore a NixOS machine has other needs than a Junos router and it also gets queried in a different way.
 * We try to create a uniform, displayable API for a future front end later on.
 * 
 * Currently, we create a new DeviceHealthchecker instance for each device type, not for every device.
 * Therefore the user only gets a shared reference which only should 
 * be used to read possible informations from the DeviceHealthchecker.
 */

pub struct Healthchecker {
    kv_client: KvClient,
    dummy_healthchecker: DummyHealthchecker,
}


impl Healthchecker {

    pub fn new(c: &Client) -> Self {

        let kv_client = c.kv_client();

        Healthchecker { 
            kv_client, 
            dummy_healthchecker: DummyHealthchecker {  }
        }
    }

    fn get_device_healthchecker(&self, device_type: &DeviceTypeEnum) -> &impl DeviceHealthchecker {

        match device_type {
            DeviceTypeEnum::Dummy => {
                &self.dummy_healthchecker
            },
        }

    }

    pub async fn check_nodes(&mut self) -> anyhow::Result<()>{

        let nodes: Vec<DeviceManifest> = self.kv_client.get_yaml_list("vicky.wobcom.de/node/manifest".to_string(), Some(GetOptions::new().with_prefix())).await?;

        debug!("{:?}", nodes);

        for n in nodes {
            info!("Checking {}", n.name);

            let dev_healthchecker = self.get_device_healthchecker(&n.device_type);
            let health = dev_healthchecker.get_device_health(&n)?;
            debug!("status: {:?}", health.status);

            self.kv_client.put_yaml(format!("vicky.wobcom.de/node/health/{}", n.name), health, None).await?;
        }

        Ok(())
    }

}

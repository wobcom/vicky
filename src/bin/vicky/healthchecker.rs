use anyhow::Result;
use etcd_client::{Client, GetOptions, KvClient};
use log::{info, debug};
use vickylib::{documents::{DeviceManifest, DeviceHealth, DeviceHealthEnum, DeviceTypeEnum}, etcd::client::ClientExt};

trait DeviceHealthchecker {
    fn get_device_health(&self, device: &DeviceManifest) -> Result<DeviceHealth>;
}

pub struct DummyHealthchecker {

}

impl DeviceHealthchecker for DummyHealthchecker {
    fn get_device_health(&self, _device: &DeviceManifest) -> Result<DeviceHealth> {
        Ok(DeviceHealth {
            status: DeviceHealthEnum::Operational,
            info: vec![],
            warnings: vec![],
            errors: vec![],
        })
    }
}

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

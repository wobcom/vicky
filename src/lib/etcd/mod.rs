use anyhow::anyhow;
use async_trait::async_trait;
use etcd_client::{Error, ProclaimOptions, LeaseClient, GetOptions, PutOptions, KeyValue, GetResponse, Client};
use log::{debug, info};
use serde::de::DeserializeOwned;
use serde::{Deserialize, Serialize};
use tokio::sync::Mutex;
use std::sync::{Arc};
use std::{thread, time};

const ETCD_PREFIX: &str = "vicky.wobcom.de";

const ELECTION_NAME: &str = "vicky.wobcom.de/leader-election";

pub type NodeId = String;

use thiserror::Error;

#[derive(Error, Debug)]
pub enum ClientError {
    #[error("Failed to interact with etcd")]
    EtcdError(#[from] etcd_client::Error),
    #[error("Failed to parse yaml")]
    YamlError(#[from] serde_yaml::Error),
}

#[async_trait]
pub trait ClientExt {
    async fn get_yaml_list<T: DeserializeOwned>(&mut self, key: String,options: Option<GetOptions>) -> Result<Vec<T>, ClientError>;
    // async fn get_yaml<T: DeserializeOwned>(&mut self, key: String,options: Option<GetOptions>) -> Result<Option<&T>, ClientError>;
    async fn put_yaml<T: Serialize + Send>(&mut self, key: String, elem: T, options: Option<PutOptions>) -> Result<(), ClientError>;
}

#[async_trait]
impl ClientExt for etcd_client::Client {

    async fn get_yaml_list<T: DeserializeOwned>(&mut self, key: String, options: Option<GetOptions>) -> Result<Vec<T>, ClientError> {

        let get_resp = self.get(key, options).await?;
        let x = get_resp.kvs();
        
        let mut ret_val = vec![];

        for elem in x {
            let elem: T = serde_yaml::from_str(elem.value_str()?)?;
            ret_val.push(elem);
        }

        return Ok(ret_val);
    }

    async fn put_yaml<T: Serialize + Send>(&mut self, key: String, elem: T, options: Option<PutOptions>) -> Result<(), ClientError> {
        let yaml_str = serde_yaml::to_string(&elem)?;
        self.put(key, yaml_str, options).await?;
        return Ok(())
    }
}

enum ElectionState {
    IDLE,
    WAITING,
    LEADER,
}

#[derive(Debug)]
enum LeaseState {
    NOLEASE,
    LEASE { leaseId: i64 }
}

pub struct Election {
    node_id: NodeId,
    c: etcd_client::Client,

    state: ElectionState,

    lease_state: Arc<Mutex<LeaseState>>,

}

impl Election {
    
    pub fn new(c: etcd_client::Client, node_id: NodeId) -> Election {

        let m = Mutex::new(LeaseState::NOLEASE);

        Election {
            c,
            node_id,

            state: ElectionState::IDLE,
            lease_state: m.into()
        }
    }

    pub async fn elect(&mut self) -> Result<(), Error> {

        self.state = ElectionState::WAITING;

        let resp = self.c.lease_grant(10, None).await?;
        let lease_id = resp.id();

        {
            let mut x = self.lease_state.lock().await;
            *x = LeaseState::LEASE { leaseId: lease_id };
        }
        
        debug!("grant ttl:{:?}, id:{:?}, lease_id: {:?}", resp.ttl(), resp.id(), lease_id);
    
        // campaign
        let resp = self.c.campaign(ELECTION_NAME, self.node_id.clone(), lease_id).await?;
        let leader = resp.leader().unwrap();
        debug!(
            "election name:{:?}, leaseId:{:?}",
            leader.name_str(),
            leader.lease()
        );
    
        // observe
        let mut msg = self.c.observe(leader.name()).await?;
        loop {
            if let Some(resp) = msg.message().await? {
                debug!("observe key {:?}", resp.kv().unwrap().key_str());
                if resp.kv().is_some() {
                    break;
                }
            }
        }
    
        // leader
        let resp = self.c.leader(ELECTION_NAME).await?;
        let kv = resp.kv().unwrap();
        debug!("key is {:?}", kv.key_str());
        debug!("value is {:?}", kv.value_str());

        self.state = ElectionState::LEADER;

        Ok(())
    }

    pub fn keep_alive(&self) {


        let mut lease_client = self.c.lease_client();
        let lease_state = Arc::clone(&self.lease_state);
        debug!("spawning refresh lease thread");

        tokio::spawn(
            async move {

                loop {
                    {

                        let x = lease_state.lock().await;

                        match *x {
                            LeaseState::NOLEASE => {},
                            LeaseState::LEASE { leaseId } => {
                                debug!("refreshing lease {}", leaseId);
                                lease_client.keep_alive(leaseId).await.unwrap();
                            },
                        };
                    }
    
                    thread::sleep(time::Duration::from_secs(5));
    
        
                }
            }
        );

    }

}
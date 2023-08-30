use etcd_client::{ElectionClient, Client, LeaseClient};
use log::{debug};
use std::sync::Arc;
use std::{thread, time};
use tokio::sync::Mutex;

use super::client::{ClientError};

const ELECTION_NAME: &str = "vicky.wobcom.de/leader-election";

pub type NodeId = String;


enum ElectionState {
    Idle,
    Waiting,
    Leader,
}

#[derive(Debug)]
enum LeaseState {
    NoLease,
    Lease { lease_id: i64 },
}

pub struct Election {
    node_id: NodeId,
    lease_client: LeaseClient,
    election_client: ElectionClient,

    state: ElectionState,

    lease_state: Arc<Mutex<LeaseState>>,
}

impl Election {
    pub fn new(c: &Client, node_id: NodeId) -> Election {
        let m = Mutex::new(LeaseState::NoLease);

        let lease_client = c.lease_client().clone();
        let election_client = c.election_client().clone();

        Election {
            lease_client,
            election_client,
            node_id,

            state: ElectionState::Idle,
            lease_state: m.into(),
        }
    }

    pub async fn elect(&mut self) -> Result<(), ClientError> {
        self.state = ElectionState::Waiting;

        let resp = self.lease_client.grant(10, None).await?;
        let lease_id = resp.id();

        {
            let mut x = self.lease_state.lock().await;
            *x = LeaseState::Lease { lease_id };
        }

        debug!(
            "grant ttl:{:?}, id:{:?}, lease_id: {:?}",
            resp.ttl(),
            resp.id(),
            lease_id
        );

        // campaign
        let resp = self
            .election_client
            .campaign(ELECTION_NAME, self.node_id.clone(), lease_id)
            .await?;
        let leader = resp.leader().unwrap();
        debug!(
            "election name:{:?}, leaseId:{:?}",
            leader.name_str(),
            leader.lease()
        );

        // leader
        let resp = self.election_client.leader(ELECTION_NAME).await?;
        let kv = resp.kv().unwrap();
        debug!("key is {:?}", kv.key_str());
        debug!("value is {:?}", kv.value_str());

        self.state = ElectionState::Leader;

        Ok(())
    }

    pub fn keep_alive(&self) {
        let lease_state = Arc::clone(&self.lease_state);
        debug!("spawning refresh lease thread");

        let mut lease_client = self.lease_client.clone();


        // tokio does some funky stuff here, it blocks the requests sometimes.
        tokio::spawn(async move {
            loop {
                {
                    let x = lease_state.lock().await;

                    match *x {
                        LeaseState::NoLease => {}
                        LeaseState::Lease { lease_id } => {
                            debug!("refreshing lease {}", lease_id);
                            lease_client.keep_alive(lease_id).await.unwrap();
                        }
                    };
                }

                thread::sleep(time::Duration::from_secs(5));
            }
        });
    }
}

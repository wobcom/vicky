
use serde::{Serialize, Deserialize};
use std::net::IpAddr;

#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct NodeManifest {
    pub name: String,
    pub ipv4: IpAddr,
}
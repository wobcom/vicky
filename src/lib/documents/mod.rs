
use serde::{Serialize, Deserialize};
use std::net::IpAddr;

/**
 * Manifests contain static documents, which are only edited by the user or other external components, e.g. NodeManifest.
 * They never get changed by Vicky themself.
 * 
 * Other documents, not explicitly named *Manifest, are volatile and can be edited by Vicky at any time, e.g. NodeHealth.
 */

#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="kebab-case")]
pub enum DeviceTypeEnum {
    Dummy,
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceManifest {
    pub name: String,

    pub device_type: DeviceTypeEnum,  
    pub ipv4: IpAddr,
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
#[serde(rename_all="kebab-case")]
pub enum DeviceHealthEnum {
    Operational,
    Degraded,
    HumanNeeded,
}


#[derive(Debug, PartialEq, Serialize, Deserialize)]
pub struct DeviceHealth {

    pub status: DeviceHealthEnum,

    pub info: Vec<String>,
    pub warnings: Vec<String>,
    pub errors: Vec<String>,
}


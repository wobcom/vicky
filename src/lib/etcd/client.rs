use async_trait::async_trait;
use etcd_client::{
    GetOptions, PutOptions, KvClient
};
use serde::de::DeserializeOwned;
use serde::{Serialize};

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
    async fn get_yaml_list<T: DeserializeOwned>(
        &mut self,
        key: String,
        options: Option<GetOptions>,
    ) -> Result<Vec<T>, ClientError>;
    async fn get_yaml<T: DeserializeOwned>(
        &mut self,
        key: String,
        options: Option<GetOptions>,
    ) -> Result<Option<T>, ClientError>;
    async fn put_yaml<T: Serialize + Send + Sync>(
        &mut self,
        key: String,
        elem: &T,
        options: Option<PutOptions>,
    ) -> Result<(), ClientError>;
}

#[async_trait]
impl ClientExt for KvClient {
    async fn get_yaml_list<T: DeserializeOwned>(
        &mut self,
        key: String,
        options: Option<GetOptions>,
    ) -> Result<Vec<T>, ClientError> {
        let get_resp = self.get(key, options).await?;
        let x = get_resp.kvs();

        let mut ret_val = vec![];

        for elem in x {
            let elem: T = serde_yaml::from_str(elem.value_str()?)?;
            ret_val.push(elem);
        }

        return Ok(ret_val);
    }

    async fn get_yaml<T: DeserializeOwned>(
        &mut self,
        key: String,
        options: Option<GetOptions>,
    ) -> Result<Option<T>, ClientError> {
        let mut list: Vec<T> = self.get_yaml_list(key, options).await?;
        assert!(list.len() <= 1, "list had too many entries");

        if list.len() == 0 {
            return Ok(None);
        }

        let out = list.remove(0);

        Ok(Some(out))
    }

    async fn put_yaml<T: Serialize + Send + Sync>(
        &mut self,
        key: String,
        elem: &T,
        options: Option<PutOptions>,
    ) -> Result<(), ClientError> {
        let yaml_str = serde_yaml::to_string(elem)?;
        self.put(key, yaml_str, options).await?;
        return Ok(());
    }
}
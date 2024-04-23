use async_trait::async_trait;
use etcd_client::{
    GetOptions, PutOptions, KvClient
};
use serde::de::DeserializeOwned;
use serde::{Serialize};

use crate::errors::VickyError;

#[async_trait]
pub trait ClientExt {
    async fn get_yaml_list<T: DeserializeOwned>(
        &mut self,
        key: String,
        options: Option<GetOptions>,
    ) -> Result<Vec<T>, VickyError>;
    async fn get_yaml<T: DeserializeOwned>(
        &mut self,
        key: String,
        options: Option<GetOptions>,
    ) -> Result<Option<T>, VickyError>;
    async fn put_yaml<T: Serialize + Send + Sync>(
        &mut self,
        key: String,
        elem: &T,
        options: Option<PutOptions>,
    ) -> Result<(), VickyError>;
}

#[async_trait]
impl ClientExt for KvClient {
    async fn get_yaml_list<T: DeserializeOwned>(
        &mut self,
        key: String,
        options: Option<GetOptions>,
    ) -> Result<Vec<T>, VickyError> {
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
    ) -> Result<Option<T>, VickyError> {
        let mut list: Vec<T> = self.get_yaml_list(key, options).await?;
        assert!(list.len() <= 1, "list had too many entries");

        if list.is_empty() {
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
    ) -> Result<(), VickyError> {
        let yaml_str = serde_yaml::to_string(elem)?;
        self.put(key, yaml_str, options).await?;
        return Ok(());
    }
}

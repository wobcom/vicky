use std::fs::{File, self};
use std::path::PathBuf;
use std::sync::{Mutex, Arc};

use chrono::{Utc, DateTime, Duration};
use log::info;
use openidconnect::core::{CoreClient, CoreProviderMetadata};
use openidconnect::{ClientId, ClientSecret, IssuerUrl, OAuth2TokenResponse, Scope};
use reqwest::Method;
use serde::de::DeserializeOwned;
use reqwest::blocking::{self, RequestBuilder, Client, ClientBuilder};
use openidconnect::reqwest::{http_client};

use serde::{Serialize, Deserialize};


#[derive(Serialize, Deserialize, Debug)]
pub struct EnvConfig {
    issuer_url: String,
    url: String,
    client_id: String,
    client_secret: String,
}

#[derive(Serialize, Deserialize, Debug)]
pub struct FileConfig {
    issuer_url: String,
    vicky_url: String,
    client_id: String,
    refresh_token: String,
}

impl FileConfig {
    pub fn save(&self) -> Result<(), anyhow::Error> {
        let mut path:PathBuf = dirs::config_dir().unwrap();

        path.push("vickyctl");
        fs::create_dir_all(path.clone())?;

        path.push("account.json");
        let config_file = File::create_new(path)?;

        serde_json::to_writer_pretty(config_file, self)?;
        Ok(())
    }
}


#[derive(Debug)]
pub enum ConfigState {
    EnvironmentAuthenticated(EnvConfig),
    FileAuthenticated(FileConfig),
    Unauthenticated,
}

impl ConfigState {
    fn get_base_url(&self) -> String {
        match self {
            ConfigState::EnvironmentAuthenticated(env_cfg) => env_cfg.url.clone(),
            ConfigState::FileAuthenticated (file_cfg) => file_cfg.vicky_url.clone(),
            ConfigState::Unauthenticated => panic!(),
        }
    }
}


#[derive(Debug, Clone)]
pub enum HttpClientState {
    Authenticated {
        access_token: String,
        expires_at: DateTime<Utc>,
    },
    Unauthenticated,
}
#[derive(Debug)]
pub struct HttpClient<'a> {
    config_state: &'a ConfigState,
    http_client: Client,
    client_state: Arc<Mutex<HttpClientState>>,
}

impl HttpClient<'_> {
    pub fn new(cfg: &ConfigState, user_agent: String) -> HttpClient {
        HttpClient {
            config_state: cfg,
            http_client: ClientBuilder::new().user_agent(user_agent).build().unwrap(),
            client_state: Arc::new(Mutex::new(HttpClientState::Unauthenticated)),
        }
    }

    fn renew_access_token(&self, client_state: &mut HttpClientState) -> anyhow::Result<String> {

        match self.config_state {
            ConfigState::EnvironmentAuthenticated(env_config) => {
                let client_id = ClientId::new(env_config.client_id.clone());
                let client_secret = ClientSecret::new(env_config.client_secret.clone());
                let issuer_url = IssuerUrl::new(env_config.issuer_url.clone())?;

                info!("Using {:?} as client_id to try to authorize to {:?}..", client_id, issuer_url);

                let provider_metadata = CoreProviderMetadata::discover(&issuer_url, &http_client)?;
                let client = CoreClient::from_provider_metadata(
                    provider_metadata,
                    client_id,
                    Some(client_secret),
                );

                let ccreq = client
                    .exchange_client_credentials()
                    .add_scope(Scope::new("openid".to_string()));

                let ccres = ccreq.request(http_client)?;

                let access_token = ccres.access_token().secret();
                let expires_at = Utc::now() + ccres.expires_in().unwrap() - Duration::seconds(5);

                info!("Accquired access token, expiring at {:?} ..", expires_at);

                *client_state = HttpClientState::Authenticated { access_token: access_token.clone(), expires_at };

                Ok(access_token.clone())
            },
            ConfigState::FileAuthenticated(_) => todo!(),
            ConfigState::Unauthenticated => todo!(),
        }



        
    }

    pub fn create_request(&self, method: Method, endpoint: &str) -> anyhow::Result<RequestBuilder> {

        let now = Utc::now();

        info!("client_state: {:?}", self.client_state);
        let base_url = self.config_state.get_base_url();
        let url = format!("{}/{}", base_url, endpoint);

        // We need to clone client_state here to release the lock immediatly.
        let mut client_state = self.client_state.lock().unwrap();
        let access_token_to_use = match &*client_state {
            HttpClientState::Authenticated { expires_at, access_token } => {
                if expires_at > &now {
                    access_token.to_string()
                } else {
                    self.renew_access_token(&mut client_state)?
                }
            },
            HttpClientState::Unauthenticated => {
                self.renew_access_token(&mut client_state)?
            },
        };

        Ok(self.http_client.request(method, url).header("Authorization", format!("Bearer {}", access_token_to_use)))
    }    


    pub fn do_request<BODY: Serialize, RESPONSE: DeserializeOwned>(
        &self,
        method: Method,
        endpoint: &str,
        q: &BODY,
    ) -> anyhow::Result<RESPONSE> {

    
        let response = self
            .create_request(method, endpoint)?
            .header("content-type", "application/json")
            .json(q)
            .send()?;
    
    
        if !response.status().is_success() {
            anyhow::bail!("API error: {:?}", response);
        }
    
        Ok(response.json()?)
    }
}

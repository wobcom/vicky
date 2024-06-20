use std::sync::Arc;

use chrono::{Utc, DateTime, Duration};
use log::info;
use openidconnect::core::{CoreClient, CoreProviderMetadata};
use openidconnect::{ClientId, ClientSecret, IssuerUrl, OAuth2TokenResponse, Scope};
use serde::de::DeserializeOwned;
use serde::{Serialize};
use reqwest::{self, Method, RequestBuilder};
use openidconnect::reqwest::async_http_client;


use crate::AppConfig;
use crate::error::FairyError;

#[derive(Debug)]
pub enum HttpClientState {
    Authenticated {
        access_token: String,
        expires_at: DateTime<Utc>,
    },
    Unauthenticated,
}
#[derive(Debug)]
pub struct HttpClient {
    app_config: Arc<AppConfig>,
    http_client: reqwest::Client,
    client_state: HttpClientState
}

impl HttpClient {
    pub fn new(cfg: Arc<AppConfig>) -> HttpClient {
        HttpClient {
            app_config: cfg,
            http_client: reqwest::Client::new(),
            client_state: HttpClientState::Unauthenticated,
        }
    }

    pub async fn renew_access_token(&mut self) -> anyhow::Result<String> {
        let client_id = ClientId::new(self.app_config.oidc_config.client_id.clone());
        let client_secret = ClientSecret::new(self.app_config.oidc_config.client_secret.clone());
        let issuer_url = IssuerUrl::new(self.app_config.oidc_config.issuer_url.clone())?;

        info!("Using {:?} as client_id to try to authorize to {:?}..", client_id, issuer_url);

        let provider_metadata = CoreProviderMetadata::discover_async(issuer_url, &async_http_client).await?;
        let client = CoreClient::from_provider_metadata(
            provider_metadata,
            client_id,
            Some(client_secret),
        );

        let ccreq = client
            .exchange_client_credentials()
            .add_scope(Scope::new("openid".to_string()));

        let ccres = ccreq.request_async(async_http_client).await
            .map_err(|_| FairyError::Unauthorized)?; 

        let access_token = ccres.access_token().secret();
        let expires_at = Utc::now() + ccres.expires_in().unwrap() - Duration::seconds(5);

        info!("Accquired access token, expiring at {:?} ..", expires_at);

        self.client_state = HttpClientState::Authenticated { access_token: access_token.clone(), expires_at };
        Ok(access_token.clone())
    }

    async fn create_request<U: reqwest::IntoUrl>(&mut self, method: Method, url: U) -> anyhow::Result<RequestBuilder> {

        let now = Utc::now();

        info!("client_state: {:?}", self.client_state);

        let access_token_to_use = match &self.client_state {
            HttpClientState::Authenticated { expires_at, access_token } => {
                if expires_at > &now {
                    access_token.to_string()
                } else {
                    self.renew_access_token().await?
                }
            },
            HttpClientState::Unauthenticated => {
                self.renew_access_token().await?
            },
        };

        Ok(self.http_client.request(method, url).header("Authorization", format!("Bearer {}", access_token_to_use)))
    }    


    pub async fn do_request<BODY: Serialize, RESPONSE: DeserializeOwned>(
        &mut self,
        method: Method,
        endpoint: &str,
        q: &BODY,
    ) -> anyhow::Result<RESPONSE> {
    
        let response = self
            .create_request(method, format!("{}/{}", self.app_config.vicky_url, endpoint)).await?
            .header("content-type", "application/json")
            .json(q)
            .send().await?;
    
    
        if !response.status().is_success() {
            anyhow::bail!("API error: {:?}", response);
        }
    
        Ok(response.json().await?)
    }
}

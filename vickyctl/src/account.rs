use openidconnect::{ErrorResponse, ClientId, IssuerUrl, core::{CoreProviderMetadata, CoreClient, CoreDeviceAuthorizationResponse, CoreAuthDisplay, CoreClientAuthMethod, CoreClaimName, CoreClaimType, CoreGrantType, CoreJweContentEncryptionAlgorithm, CoreJweKeyManagementAlgorithm, CoreJsonWebKey, CoreResponseMode, CoreResponseType, CoreSubjectIdentifierType, CoreJwsSigningAlgorithm, CoreJsonWebKeyType, CoreJsonWebKeyUse}, Scope, reqwest::{http_client}, AdditionalProviderMetadata, ProviderMetadata, DeviceAuthorizationUrl, AuthType, OAuth2TokenResponse};
use serde::{Deserialize, Serialize};

use crate::{cli::AppContext, error::Error, FileConfig, AuthState};


// Taken from https://github.com/ramosbugs/openidconnect-rs/blob/support/3.x/examples/okta_device_grant.rs
#[derive(Clone, Debug, Deserialize, Serialize)]
struct DeviceEndpointProviderMetadata {
    device_authorization_endpoint: DeviceAuthorizationUrl,
}
impl AdditionalProviderMetadata for DeviceEndpointProviderMetadata {}
type DeviceProviderMetadata = ProviderMetadata<
    DeviceEndpointProviderMetadata,
    CoreAuthDisplay,
    CoreClientAuthMethod,
    CoreClaimName,
    CoreClaimType,
    CoreGrantType,
    CoreJweContentEncryptionAlgorithm,
    CoreJweKeyManagementAlgorithm,
    CoreJwsSigningAlgorithm,
    CoreJsonWebKeyType,
    CoreJsonWebKeyUse,
    CoreJsonWebKey,
    CoreResponseMode,
    CoreResponseType,
    CoreSubjectIdentifierType,
>;


pub fn show(auth_state: &AuthState) -> Result<(), anyhow::Error> {
    print!("{:?}", auth_state.clone());
    Ok(())
}

pub fn login(ctx: &AppContext, vicky_url_str: String, issuer_url_str: String, client_id_str: String) -> Result<(), anyhow::Error> {

    let client_id = ClientId::new(client_id_str.clone().to_string());
    let issuer_url = IssuerUrl::new(issuer_url_str.clone().to_string())?;


    let provider_metadata = DeviceProviderMetadata::discover(&issuer_url, http_client)?;

    let device_authorization_endpoint = provider_metadata
        .additional_metadata()
        .device_authorization_endpoint
        .clone();

    let client = CoreClient::from_provider_metadata(
        provider_metadata,
        client_id,
        None,
    )
        .set_device_authorization_uri(device_authorization_endpoint)
        .set_auth_type(AuthType::RequestBody);

    let details: CoreDeviceAuthorizationResponse = client
        .exchange_device_code()?
        .add_scope(Scope::new("profile".to_string()))
        .request(http_client)?;


    println!("Fetching device code...");
    dbg!(&details);

    // Display the URL and user-code.
    println!(
        "Open this URL in your browser:\n{}\nand enter the code: {}",
        details.verification_uri_complete().unwrap().secret(),
        details.user_code().secret()
    );

    // Now poll for the token
    let token = client
        .exchange_device_access_token(&details)
        .request(http_client, std::thread::sleep, None)?;

    let account_cfg = FileConfig {
        vicky_url: vicky_url_str,
        client_id: client_id_str,
        issuer_url: issuer_url_str,
        refresh_token: token.refresh_token().unwrap().secret().to_string(),
    };
    account_cfg.save()?;

    Ok(())
}


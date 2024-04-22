use anyhow::{Context, Error};
use jwtk::jwk::RemoteJwksVerifier;
use log::{warn, debug};
use rocket::get;
use rocket::http::Status;
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::response::{Debug, Redirect};
use rocket::{request, State};
use serde::Deserialize;
use serde_json::{Value, Map};

use crate::{Config, OIDCConfig};

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
}

#[derive(Deserialize)]
pub struct User {
    pub full_name: String,
    pub role: Role,
}

pub struct Machine {}

#[rocket::async_trait]
impl<'r> request::FromRequest<'r> for User {
    type Error = ();

    async fn from_request(request: &'r request::Request<'_>) -> request::Outcome<User, ()> {
        let jwks_verifier: &State<_> = request
            .guard::<&State<RemoteJwksVerifier>>()
            .await
            .expect("request KeyStore");

        if let Some(auth_header) = request.headers().get_one("Authorization") {

            if !auth_header.starts_with("Bearer") {
                return request::Outcome::Forward(()) 
            }

            let token = auth_header.trim_start_matches("Bearer ");

            return match jwks_verifier.verify::<Map<String, Value>>(token).await {
                Ok(jwt) => {
                    debug!("{:?}", jwt);
                    request::Outcome::Success(User {
                        full_name: "Test Wurst".to_string(),
                        role: Role::Admin,
                    })
                }
                Err(x) => {
                    warn!("Login failed: {:?}", x);
                    request::Outcome::Failure((Status::Forbidden, ()))
                }
            }
        }

        request::Outcome::Forward(())
    }
}

#[rocket::async_trait]
impl<'r> request::FromRequest<'r> for Machine {
    type Error = ();

    async fn from_request(request: &'r request::Request<'_>) -> request::Outcome<Machine, ()> {
        let config = request
            .guard::<&State<Config>>()
            .await
            .expect("request Config");

        if let Some(auth_header) = request.headers().get_one("Authorization") {
            let cfg_user = config.machines.iter().find(|x| *x == auth_header);

            match cfg_user {
                Some(_) => return request::Outcome::Success(Machine {}),
                None => return request::Outcome::Failure((Status::Forbidden, ())),
            }
        }

        request::Outcome::Forward(())
    }
}

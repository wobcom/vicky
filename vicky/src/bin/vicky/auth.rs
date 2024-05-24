use std::str::FromStr;

use jwtk::jwk::RemoteJwksVerifier;
use log::{warn};
use rocket::http::Status;
use rocket::{request, State};
use serde::Deserialize;
use serde_json::{Map, Value};
use uuid::Uuid;
use vickylib::database::entities::Database;

use vickylib::database::entities::user::db_impl::{UserDatabase, DbUser};

use crate::{Config, OIDCConfigResolved};
use crate::errors::AppError;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin,
}

#[allow(dead_code)]
#[derive(Deserialize)]
pub struct User {
    pub id: Uuid,
    pub full_name: String,
    pub role: Role,
}

pub struct Machine {}

async fn extract_user_from_token(jwks_verifier: &State<RemoteJwksVerifier>, db: &Database, oidc_config: &OIDCConfigResolved, token: &str) -> Result<DbUser, AppError> {
    let jwt = jwks_verifier.verify::<Map<String, Value>>(token).await?;
    
    let sub = match &jwt.claims().sub {
        Some(sub) => Some(Uuid::from_str(sub)?),
        None => return Err(AppError::JWTFormatError("JWT must contain sub".to_string()))
    };

    let user = db.run(move |conn| conn.get_user(sub.unwrap())).await?;

    match user {
        Some(user) => {
            Ok(user)
        }
        None => {
            let oidc_client = reqwest::Client::new();
            let x = oidc_client.get(oidc_config.userinfo_endpoint.clone())
                .header("Authorization", format!("Bearer {}", token))
                .send()
                .await?;

            let user_info = x.json::<serde_json::Value>().await?;

            let name = match user_info.get("name").and_then(|x| x.as_str()) {
                Some(name) => Some(name),
                None => return Err(AppError::JWTFormatError("user_info must contain name".to_string()))
            };

            let new_user = DbUser {
                sub: sub.unwrap(),
                name: name.unwrap().to_string(),
                role: "ADMIN".to_string(),
            };

            let new_user_create = new_user.clone();
            db.run(move |conn| conn.upsert_user(new_user_create)).await?;
            Ok(new_user)
        }
    }
}

#[rocket::async_trait]
impl<'r> request::FromRequest<'r> for User {
    type Error = ();

    async fn from_request(request: &'r request::Request<'_>) -> request::Outcome<User, ()> {
        let jwks_verifier: &State<_> = request
            .guard::<&State<RemoteJwksVerifier>>()
            .await
            .expect("request KeyStore");

        let db: &Database = &request
            .guard::<Database>()
            .await
            .expect("request Database");

        
        let oidc_config_resolved: &OIDCConfigResolved = request
            .guard::<&State<OIDCConfigResolved>>()
            .await
            .expect("request OIDCConfigResolved");

        if let Some(auth_header) = request.headers().get_one("Authorization") {
            if !auth_header.starts_with("Bearer") {
                return request::Outcome::Forward(Status::Forbidden);
            }

            let token = auth_header.trim_start_matches("Bearer ");

            return match extract_user_from_token(jwks_verifier, db, oidc_config_resolved, token).await {
                Ok(user) => {
                    let user = User {
                        id: user.sub,
                        full_name: user.name,
                        role: Role::Admin
                    };

                    request::Outcome::Success(user)
                }

                Err(x) => {
                    warn!("Login failed: {:?}", x);
                    request::Outcome::Error((Status::Forbidden, ()))
                }
            }
        }

        request::Outcome::Forward(Status::Forbidden)
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

            return match cfg_user {
                Some(_) => request::Outcome::Success(Machine {}),
                None => request::Outcome::Error((Status::Forbidden, ())),
            };
        }

        request::Outcome::Forward(Status::Forbidden)
    }
}

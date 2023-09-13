use anyhow::{Context, Error};
use reqwest::header::{ACCEPT, AUTHORIZATION, USER_AGENT};
use rocket::http::{Cookie, CookieJar, SameSite};
use rocket::{request, State};
use rocket::response::{Debug, Redirect};
use rocket::{get};
use rocket_oauth2::{OAuth2, TokenResponse};
use serde::Deserialize;
use rocket::http::Status;

use crate::Config;

#[derive(Deserialize, Clone)]
#[serde(rename_all = "lowercase")]
pub enum Role {
    Admin
}

#[derive(Deserialize)]
pub struct User {
    pub full_name: String,
    pub role: Role 
}

pub struct Machine {
    
}

#[rocket::async_trait]
impl<'r> request::FromRequest<'r> for User {
    type Error = ();

    async fn from_request(request: &'r request::Request<'_>) -> request::Outcome<User, ()> {

        let cookies = request
            .guard::<&CookieJar<'_>>()
            .await
            .expect("request cookies");

        let config = request
            .guard::<&State<Config>>()
            .await
            .expect("request Config");

        if let Some(cookie) = cookies.get_private("vicky_username") {

            let username = cookie.value().to_string();
            
            let cfg_user = config.users.get(&username);
            match cfg_user {
                Some(cfg_user) => {
                    return request::Outcome::Success(User {
                        full_name: cfg_user.full_name.clone(),
                        role: cfg_user.role.clone(),
                    })
                },
                None => {
                    return request::Outcome::Failure((Status::Forbidden, ()))
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
                Some(_) => {
                    return request::Outcome::Success(Machine {})
                },
                None => {
                    return request::Outcome::Failure((Status::Forbidden, ()))
                }
            }
        }

        request::Outcome::Forward(())
    }
}

/// User information to be retrieved from the GitHub API.
#[derive(serde::Deserialize)]
pub struct GitHubUserInfo {
    #[serde(default)]
    login: String,
}

// NB: Here we are using the same struct as a type parameter to OAuth2 and
// TokenResponse as we use for the user's GitHub login details. For
// `TokenResponse` and `OAuth2` the actual type does not matter; only that they
// are matched up.
#[get("/login/github")]
pub fn github_login(oauth2: OAuth2<GitHubUserInfo>, cookies: &CookieJar<'_>) -> Redirect {
    oauth2.get_redirect(cookies, &["user:read"]).unwrap()
}

#[get("/callback/github")]
pub async fn github_callback(
    token: TokenResponse<GitHubUserInfo>,
    cookies: &CookieJar<'_>,
    config: &State<Config>,
) -> Result<Redirect, Debug<Error>> {
    // Use the token to retrieve the user's GitHub account information.
    let user_info: GitHubUserInfo = reqwest::Client::builder()
        .build()
        .context("failed to build reqwest client")?
        .get("https://api.github.com/user")
        .header(AUTHORIZATION, format!("token {}", token.access_token()))
        .header(ACCEPT, "application/vnd.github.v3+json")
        .header(USER_AGENT, "Vicky")
        .send()
        .await
        .context("failed to complete request")?
        .json()
        .await
        .context("failed to deserialize response")?;


    // We only set a cookie, if the user was allowed to do this.
    // We also check, if the username within the cookie still matches our list later on.

    let user = config.users.get(&user_info.login);

    if user.is_some() {
        // Set a private cookie with the user's name, and redirect to the home page.
        cookies.add_private(
            Cookie::build("vicky_username", user_info.login)
                .same_site(SameSite::Lax)
                .finish(),
        );
        Ok(Redirect::to("/"))
    } else {
        Ok(Redirect::to("/login/error"))
    }

   
}

#[get("/logout")]
pub fn logout(cookies: &CookieJar<'_>) -> Redirect {
    cookies.remove(Cookie::named("vicky_username"));
    Redirect::to("/")
}

use std::time::Duration;

use crate::{
    model::{User, COOKIE_NAME},
    session::DynamoSessionStore,
};

use super::super::error::CustomError;
use async_session::{Session, SessionStore};
use aws_lambda_events::{
    apigw::{ApiGatewayV2httpRequest as Request, ApiGatewayV2httpResponse as Response},
    http::HeaderMap,
};
use lambda_runtime::{Error, LambdaEvent};
use oauth2::{basic::BasicClient, AuthUrl, ClientId, ClientSecret, RedirectUrl, TokenUrl};
use oauth2::{CsrfToken, Scope};
use serde::Deserialize;
use tracing::info;

pub const GITHUB_AUTH_URL: &str = "https://github.com/login/oauth/authorize";
pub const GITHUB_TOKEN_URL: &str = "https://github.com/login/oauth/access_token";

#[derive(Deserialize, Debug)]
pub struct GithubTokenResponse {
    pub access_token: String,
    pub scope: String,
    pub token_type: String,
}

#[derive(Deserialize, Debug)]
pub struct GithubUserEmail {
    pub email: String,
    pub primary: bool,
    pub verified: bool,
}

pub fn oauth_client(
    client_id: String,
    client_secret: String,
    auth_url: String,
    token_url: String,
    redirect_url: String,
) -> BasicClient {
    BasicClient::new(
        ClientId::new(client_id),
        Some(ClientSecret::new(client_secret)),
        AuthUrl::new(auth_url).unwrap(),
        Some(TokenUrl::new(token_url).unwrap()),
    )
    .set_redirect_uri(RedirectUrl::new(redirect_url).unwrap())
}

pub async fn oauth_redirect(
    ssm_client: &aws_sdk_ssm::Client,
    event: LambdaEvent<Request>,
) -> Result<Response, Error> {
    let host = event
        .payload
        .headers
        .get("host")
        .ok_or(CustomError::new("no header: host"))?
        .to_str()
        .unwrap()
        .to_string();

    let path = event
        .payload
        .raw_path
        .ok_or(CustomError::new("no raw path"))?
        .replace("/start", "/callback");

    let callback_url = format!("https://{host}{path}");
    info!("callback url: {}", callback_url);

    let (Ok(gh_client_id), Ok(gh_client_secret)) = 
        (std::env::var("PARAM_GITHUB_CLIENT_ID"), 
         std::env::var("PARAM_GITHUB_CLIENT_SECRET")) else {
        return Err(CustomError::new("OAUTH PARMS not set").into())
    };

    let res = ssm_client
        .get_parameters()
        .with_decryption(true)
        .names(gh_client_id) //TODO parameterize
        .names(gh_client_secret)
        .send()
        .await
        .unwrap();
    let p = res.parameters().unwrap();

    let client_id = p[0].value().unwrap().to_string();
    let client_secret = p[1].value().unwrap().to_string();

    let oc = oauth_client(
        client_id,
        client_secret,
        GITHUB_AUTH_URL.to_string(),
        GITHUB_TOKEN_URL.to_string(),
        callback_url,
    );

    let (auth_url, _csrf_token) = oc
        .authorize_url(CsrfToken::new_random)
        .add_scope(Scope::new("user:email".to_string()))
        .url();

    let auth_url = auth_url.as_ref();

    let mut headers = HeaderMap::new();
    headers.insert("Location", auth_url.parse().unwrap());

    let resp = Response {
        status_code: 307,
        body: None,
        headers,
        multi_value_headers: HeaderMap::new(),
        is_base64_encoded: None,
        cookies: vec![],
    };

    Ok(resp)
}

pub async fn oauth_callback(
    ssm_client: &aws_sdk_ssm::Client,
    rest_client: &reqwest::Client,
    session_store: &DynamoSessionStore,
    event: LambdaEvent<Request>,
) -> Result<Response, Error> {
    let host = event
        .payload
        .headers
        .get("host")
        .ok_or(CustomError::new("no header: host"))?
        .to_str()
        .unwrap()
        .to_string();

    let stage = event
        .payload
        .request_context
        .stage
        .unwrap_or(String::default());

    let res = ssm_client
        .get_parameters()
        .with_decryption(true)
        .names("/oath/dev/oauth/github/client_id")
        .names("/oath/dev/oauth/github/client_secret")
        .send()
        .await
        .unwrap();
    let params = res.parameters().ok_or(CustomError::new("no params"))?;

    let client_id = params[0]
        .value()
        .ok_or(CustomError::new("No client_id"))?
        .to_owned();
    let client_secret = params[1]
        .value()
        .ok_or(CustomError::new("No client_secret"))?
        .to_owned();

    let code = event
        .payload
        .query_string_parameters
        .first("code")
        .ok_or(CustomError::new("No code"))?
        .to_owned();
    let _state = event
        .payload
        .query_string_parameters
        .first("state")
        .ok_or(CustomError::new("no state"))?
        .to_owned();

    let params = [
        ("code", code),
        ("client_id", client_id),
        ("client_secret", client_secret),
    ];

    let access_token = rest_client
        .post(GITHUB_TOKEN_URL)
        .header("Accept", "application/json")
        .form(&params)
        .send()
        .await
        .map_err(Box::new)?
        .json::<GithubTokenResponse>()
        .await
        .map_err(Box::new)?;

    if access_token.scope.as_str() != "user:email" {
        return Err(CustomError::new("No email scope"));
    }

    let user_emails = rest_client
        .get("https://api.github.com/user/emails")
        .header(
            "Authorization",
            format!("Bearer {}", access_token.access_token),
        )
        .send()
        .await
        .map_err(Box::new)?
        .json::<Vec<GithubUserEmail>>()
        .await
        .map_err(Box::new)?;

    let email = user_emails
        .iter()
        .find(|email| email.primary && email.verified)
        .map(|email| email.email.to_string())
        .ok_or(CustomError::new("no primary email"))?;

    let user = User { email };
    let mut session = Session::new();
    session.insert("user", user).unwrap();
    session.expire_in(Duration::from_secs(604800));

    let Ok(Some(cookie)) =  session_store.store_session(session).await else {
        return Err(CustomError::new("failed to store session"))
    };

    let cookie_str = format!("{}={}; SameSite=Lax; Path=/", COOKIE_NAME, cookie);

    let mut headers = HeaderMap::new();
    headers.insert("Set-Cookie", cookie_str.parse().unwrap());
    // route back to /protected
    let route = format!("https://{host}/{stage}/protected?session={cookie}");
    headers.insert("Location", route.parse().unwrap());

    let resp = Response {
        status_code: 307,
        body: None,
        headers,
        multi_value_headers: HeaderMap::new(),
        is_base64_encoded: None,
        cookies: vec![],
    };

    Ok(resp)
}

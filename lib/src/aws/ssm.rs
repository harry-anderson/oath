use aws_sdk_ssm::Client;

pub async fn create_client() -> Client {
    let config = ::aws_config::load_from_env().await;
    aws_sdk_ssm::Client::new(&config)
}


#[cfg(test)]
mod tests {
    use crate::aws::ssm::create_client;
    use crate::oauth::github::{oauth_client, GITHUB_AUTH_URL, GITHUB_TOKEN_URL};
    use oauth2::{CsrfToken, Scope};

    #[tokio::test]
    async fn get_multi_values() {
        let client = create_client().await;

        let res = client
            .get_parameters()
            .with_decryption(true)
            .names("/oath/dev/oauth/github/client_id")
            .names("/oath/dev/oauth/github/client_secret")
            .send()
            .await
            .unwrap();
        let _ = res.parameters().unwrap();
    }
    #[tokio::test]
    async fn test_oauth_client() {
        let client = create_client().await;

        let res = client
            .get_parameters()
            .with_decryption(true)
            .names("/oath/dev/oauth/github/client_id")
            .names("/oath/dev/oauth/github/client_secret")
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
            String::from("http://localhost:3000/login/github/callback"),
        );

        let (auth_url, _csrf_token) = oc
            .authorize_url(CsrfToken::new_random)
            .add_scope(Scope::new("identify".to_string()))
            .url();

        let auth_url = auth_url.as_ref();

        dbg!(auth_url);
    }
}

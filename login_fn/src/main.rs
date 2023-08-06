use aws_lambda_events::apigw::{
    ApiGatewayV2httpRequest as Request, ApiGatewayV2httpResponse as Response,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use lib::{
    aws::dynamodb::DbClient,
    error::CustomError,
    oauth::github::{oauth_callback, oauth_redirect},
    session::DynamoSessionStore,
};

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();
    let rest_client = reqwest::Client::builder()
        .user_agent("oath")
        .build()
        .map_err(Box::new)?;
    let ssm_client = lib::aws::ssm::create_client().await;
    let Ok(table_name) = std::env::var("TABLE_NAME") else {
        return Err(CustomError::new("ENV VAR TABLE_NAME no set").into())
    };
    let db_client = DbClient::new(&table_name).await;
    let session_store = DynamoSessionStore::new(db_client.clone()).await;
    let rest_client_ref = &rest_client;
    let ssm_client_ref = &ssm_client;
    let session_store_ref = &session_store;

    let func = service_fn(move |event| async move {
        function_handler(event, rest_client_ref, ssm_client_ref, session_store_ref).await
    });

    run(func).await?;

    Ok(())
}

async fn function_handler(
    event: LambdaEvent<Request>,
    rest_client: &reqwest::Client,
    ssm_client: &aws_sdk_ssm::Client,
    session_store: &DynamoSessionStore,
) -> Result<Response, Error> {
    let provider = &event
        .payload
        .path_parameters
        .get("provider")
        .ok_or(CustomError::new("no req param: provider"))?;
    let action = &event
        .payload
        .path_parameters
        .get("action")
        .ok_or(CustomError::new("no req param: acition"))?;
    match (provider.as_str(), action.as_str()) {
        ("github", "start") => oauth_redirect(ssm_client, event).await,
        ("github", "callback") => {
            oauth_callback(ssm_client, rest_client, session_store, event).await
        }
        _ => Err(CustomError::new("unknown command")), // TODO return 404
    }
}

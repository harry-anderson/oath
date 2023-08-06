use async_session::SessionStore;
use aws_lambda_events::apigw::{
    ApiGatewayV2CustomAuthorizerSimpleResponse as Response,
    ApiGatewayV2CustomAuthorizerV2Request as Request,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use lib::{aws::dynamodb::DbClient, error::CustomError, model::User, session::DynamoSessionStore};
use serde_json::json;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let Ok(table_name) = std::env::var("TABLE_NAME") else {
        return Err(CustomError::new("ENV VAR TABLE_NAME not set").into())
    };
    let db_client = DbClient::new(&table_name).await;
    let session_store = DynamoSessionStore::new(db_client.clone()).await;
    let session_store_ref = &session_store;

    let func =
        service_fn(move |event| async move { function_handler(event, session_store_ref).await });

    run(func).await?;

    Ok(())
}

async fn function_handler(
    event: LambdaEvent<Request>,
    session_store: &DynamoSessionStore,
) -> Result<Response, Error> {
    let Some(c) = event.payload.cookies.iter().find(|s| s.contains("SESSION=")) else {
        return reject()
    };
    let Some((_, cookie)) = c.split_once('=') else {
        return reject()
    };
    let Ok(Some(session)) = session_store.load_session(cookie.to_string()) .await else {
        return reject();
    };
    let Some(user) = session.get::<User>("user") else {
        return reject();
    };
    if session.is_expired() {
        if let Err(e) = session_store.destroy_session(session).await {
            tracing::error!("failed to destroy session error: {}", e)
        }
        return reject();
    }
    accept(json!({ "email": user.email}))
}

fn accept(context: serde_json::Value) -> Result<Response, Error> {
    Ok(Response {
        is_authorized: true,
        context,
    })
}

fn reject() -> Result<Response, Error> {
    Ok(Response {
        is_authorized: false,
        context: json!({}),
    })
}

use aws_lambda_events::{
    apigw::{ApiGatewayV2httpRequest as Request, ApiGatewayV2httpResponse as Response},
    encodings::Body,
};
use lambda_runtime::{run, service_fn, Error, LambdaEvent};
use lib::helpers::{html_response, response};
use tracing::info;

#[tokio::main]
async fn main() -> Result<(), Error> {
    tracing_subscriber::fmt()
        .with_max_level(tracing::Level::INFO)
        .with_target(false)
        .without_time()
        .init();

    let func = service_fn(move |event| async move { function_handler(event).await });

    run(func).await?;

    Ok(())
}

async fn function_handler(event: LambdaEvent<Request>) -> Result<Response, Error> {
    // info!("{:?}", event);

    let Some(auth_ctx) = event.payload.request_context.authorizer else {
        return Ok(html_response(403, Some(Body::Text(String::from("<p>Forbidden</p>")))));
    };
    let Some(email) = &auth_ctx.lambda.get("email") else {
        return Ok(html_response(403, Some(Body::Text(String::from("<p>Forbidden</p>")))));
    };
    info!("email {:?}", email);
    let res = html_response(200, Some(Body::Text(format!("<h1>hello {email}</h1>"))));
    Ok(res)
}

use aws_lambda_events::{
    apigw::ApiGatewayV2httpResponse as Response, encodings::Body, http::HeaderMap,
};

pub fn response(status_code: i64, body: Option<Body>) -> Response {
    Response {
        status_code,
        body,
        headers: HeaderMap::new(),
        multi_value_headers: HeaderMap::new(),
        is_base64_encoded: None,
        cookies: vec![],
    }
}

pub fn html_response(status_code: i64, body: Option<Body>) -> Response {
let mut headers = HeaderMap::new();
    headers.insert("Content-Type", "text/html".parse().unwrap());
    Response {
        status_code,
        body,
        headers,
        multi_value_headers: HeaderMap::new(),
        is_base64_encoded: None,
        cookies: vec![],
    }
}

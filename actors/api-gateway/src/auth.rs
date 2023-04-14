use anyhow::Result;
use serde::Deserialize;
use wasmbus_rpc::actor::prelude::WasmHost;
use wasmbus_rpc::common::Context;
use wasmbus_rpc::error::RpcResult;
use wasmcloud_interface_httpserver::{HeaderMap, HttpResponse};
use wasmcloud_interface_surrealdb::{AuthParams, RequestScope, SurrealDb, SurrealDbSender};

const WWW_AUTHENTICATE: &str = "WWW-Authenticate";
const WWW_AUTHENTICATE_CHALLENGE: &str = "Bearer scope=\"user_scope\"";
const AUTHORIZATION_HEADER: &str = "Authorization";
const BEARER_AUTH_SCHEME: &str = "Bearer ";

/// If a request is made to begin a new scan or retrieve existing reports,
/// this function extracts any valid JWT from the headers.
pub fn get_jwt_from_headers(headers: &HeaderMap) -> Option<String> {
    // TODO: authenticate with surrealDB here, return different error if token
    //       has expired of is invalid so the appropriate HTTP response can be
    //       sent to user.
    let auth_header = headers.get(AUTHORIZATION_HEADER)?.first()?;
    let auth_header = std::str::from_utf8(auth_header.as_bytes()).ok()?;
    if auth_header.contains(BEARER_AUTH_SCHEME) {
        Some(
            auth_header
                .trim_start_matches(BEARER_AUTH_SCHEME)
                .to_string(),
        )
    } else {
        None
    }
}

pub async fn sign_in(ctx: &Context, credentials: AuthParams) -> Result<HttpResponse> {
    let surreal_client: SurrealDbSender<WasmHost> = SurrealDbSender::new();
    let scope = RequestScope {
        auth_params: Some(credentials),
        scope_name: Some("user_scope".into()),
        ..Default::default()
    };
    let response = match surreal_client.sign_in(ctx, &scope).await {
        Ok(response) => {
            if response.success && response.jwt.is_some() {
                HttpResponse::json_with_headers("{}", 200, jwt_as_cookie(response.jwt.unwrap()))?
            } else if let Some(err) = response.error {
                if err.name == "SIGNIN_ERROR".to_string() {
                    unauthorised_http_response(None)
                } else {
                    HttpResponse::internal_server_error("Something went wrong")
                }
            } else {
                HttpResponse::internal_server_error("Something went terribly wrong")
            }
        }
        Err(_) => HttpResponse::internal_server_error("Something went wrong"),
    };

    Ok(response)
}

pub async fn sign_up(ctx: &Context, credentials: AuthParams) -> Result<HttpResponse> {
    let surreal_client: SurrealDbSender<WasmHost> = SurrealDbSender::new();
    let scope = RequestScope {
        auth_params: Some(credentials),
        scope_name: Some("user_scope".into()),
        ..Default::default()
    };

    let response = match surreal_client.sign_up(ctx, &scope).await {
        Ok(response) => {
            if response.success && response.jwt.is_some() {
                HttpResponse::json_with_headers("{}", 200, jwt_as_cookie(response.jwt.unwrap()))?
            } else if let Some(err) = response.error {
                if err.name == "SIGNIN_ERROR".to_string() {
                    unauthorised_http_response(None)
                } else {
                    HttpResponse::internal_server_error("Something went wrong")
                }
            } else {
                HttpResponse::internal_server_error("Something went terribly wrong")
            }
        }
        Err(_) => HttpResponse::internal_server_error("Something went wrong"),
    };

    Ok(response)
}

pub async fn auth(ctx: &Context, headers: &HeaderMap) -> Result<String> {
    todo!()
}

pub fn jwt_as_cookie(jwt: String) -> HeaderMap {
    todo!()
}

pub fn unauthorised_http_response(body: Option<Vec<u8>>) -> HttpResponse {
    HttpResponse {
        status_code: 401,
        header: www_auth_header(),
        body: vec![],
    }
}

pub fn www_auth_header() -> HeaderMap {
    HeaderMap::from([(
        WWW_AUTHENTICATE.to_string(),
        vec![WWW_AUTHENTICATE_CHALLENGE.to_string()],
    )])
}

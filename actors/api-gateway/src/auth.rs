use anyhow::{anyhow, Result};
use serde_json::json;
use wasmbus_rpc::actor::prelude::WasmHost;
use wasmbus_rpc::common::Context;
use wasmcloud_interface_httpserver::{HeaderMap, HttpResponse};
use wasmcloud_interface_surrealdb::{AuthParams, RequestScope, SurrealDb, SurrealDbSender};

const WWW_AUTHENTICATE: &str = "WWW-Authenticate";
const WWW_AUTHENTICATE_CHALLENGE: &str = "Bearer scope=\"user_scope\"";
const AUTHORIZATION_HEADER: &str = "Authorization";
const COOKIE_HEADER: &str = "Cookie";
const BEARER_AUTH_SCHEME: &str = "Bearer ";
const SET_COOKIE: &str = "Set-Cookie";

//TODO: shouldn't really be storing raw JWT in headers/cookies
//      - implement proper session tokens with HMAC etc
/// If a request is made to begin a new scan or retrieve existing reports,
/// this function extracts any valid JWT from the headers.
pub fn get_jwt_from_headers(headers: &HeaderMap) -> Option<String> {
    // TODO: authenticate with surrealDB here, return different error if token
    //       has expired of is invalid so the appropriate HTTP response can be
    //       sent to user.
    if let Some(jwt) = get_jwt_from_cookies(headers) {
        Some(jwt)
    } else {
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
}

fn get_jwt_from_cookies(headers: &HeaderMap) -> Option<String> {
    let cookies = headers.get(COOKIE_HEADER)?;
    for cookie in cookies {
        // let cookie = std::str::from_utf8(cookie.as_bytes()).ok()?;
        if cookie.contains("jwt=") {
            return Some(cookie.trim_start_matches("jwt=").into());
        }
    }
    None
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
                HttpResponse::json_with_headers("{}", 200, set_jwt_cookie(response.jwt.unwrap()))?
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
    if credentials.password.is_empty() || credentials.username.is_empty() {
        return Ok(unauthorised_http_response(Some(serde_json::to_vec(
            &json!({"msg": "username or password is empty"}),
        )?)));
    }
    let surreal_client: SurrealDbSender<WasmHost> = SurrealDbSender::new();
    let scope = RequestScope {
        auth_params: Some(credentials),
        scope_name: Some("user_scope".into()),
        ..Default::default()
    };

    let response = match surreal_client.sign_up(ctx, &scope).await {
        Ok(response) => {
            if response.success && response.jwt.is_some() {
                HttpResponse::json_with_headers("{}", 200, set_jwt_cookie(response.jwt.unwrap()))?
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

pub fn set_jwt_cookie(jwt: String) -> HeaderMap {
    HeaderMap::from([(
        SET_COOKIE.to_string(),
        vec![
            format!("jwt={jwt}"),
            "Secure".into(),
            "HttpOnly".into(),
            "SameSite=Strict".into(),
        ],
    )])
}

pub fn unauthorised_http_response(body: Option<Vec<u8>>) -> HttpResponse {
    HttpResponse {
        status_code: 401,
        header: www_auth_header(),
        body: body.unwrap_or(vec![]),
    }
}

pub fn www_auth_header() -> HeaderMap {
    HeaderMap::from([(
        WWW_AUTHENTICATE.to_string(),
        vec![WWW_AUTHENTICATE_CHALLENGE.to_string()],
    )])
}

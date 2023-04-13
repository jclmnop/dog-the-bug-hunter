use wasmcloud_interface_httpserver::{HeaderMap};

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
        Some(auth_header.trim_start_matches(BEARER_AUTH_SCHEME).to_string())
    } else {
        None
    }
}

pub fn www_auth_header() -> HeaderMap {
    HeaderMap::from([(WWW_AUTHENTICATE.to_string(), vec![WWW_AUTHENTICATE_CHALLENGE.to_string()])])
}
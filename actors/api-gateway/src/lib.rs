mod auth;

use anyhow::{anyhow, Error};
use dtbh_interface::api_gateway_prelude::*;
use dtbh_interface::{ORCHESTRATOR_ACTOR, REPORT_WRITER_ACTOR};
use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_httpserver::{HttpRequest, HttpResponse, HttpServer, HttpServerReceiver};
use wasmcloud_interface_surrealdb::AuthParams;

#[allow(dead_code)]
const CALL_ALIAS: &str = "dtbh/api-gateway";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, HttpServer)]
struct ApiGatewayActor {}

//TODO: UI request for '/' path?
enum RequestType {
    GetReports(GetReportsRequest),
    Scan(ScanRequest),
    SignIn(AuthParams),
    SignUp(AuthParams),
    // TODO: LoginPage
    // Auth(HeaderMap),
    Invalid(Error),
    Unauthorised,
}

//TODO: auth
//TODO: better error handling
#[async_trait]
impl HttpServer for ApiGatewayActor {
    async fn handle_request(&self, ctx: &Context, req: &HttpRequest) -> RpcResult<HttpResponse> {
        // info!("{req:#?}");
        match RequestType::from(req.to_owned()) {
            RequestType::GetReports(reports_request) => Ok(get_reports(ctx, reports_request)
                .await
                .unwrap_or(HttpResponse::not_found())),
            //TODO check auth before sending scan request (otherwise it wastes time getting endpoints before being rejected by db)
            RequestType::Scan(scan_request) => Ok(scan(ctx, scan_request)
                .await
                .unwrap_or(HttpResponse::not_found())),
            RequestType::SignIn(credentials) => Ok(auth::sign_in(ctx, credentials)
                .await
                .unwrap_or(HttpResponse::internal_server_error("Error signing in"))),
            RequestType::SignUp(credentials) => Ok(auth::sign_up(ctx, credentials)
                .await
                .unwrap_or(HttpResponse::internal_server_error("Error signing up"))),
            // RequestType::Auth(headers) => todo!("Redirect to authorised page?"),
            // RequestType::LoginPage => todo!("HTML for login page")
            RequestType::Invalid(e) => {
                error!("{e}");
                Ok(HttpResponse::not_found())
            }
            RequestType::Unauthorised => Ok(auth::unauthorised_http_response(None)),
        }
    }
}

async fn scan(ctx: &Context, req: ScanRequest) -> RpcResult<HttpResponse> {
    debug!("Scan request: {:#?}", req);
    let orchestrator: OrchestratorSender<_> = OrchestratorSender::to_actor(ORCHESTRATOR_ACTOR);
    let targets = req.targets;
    let mut failures: Vec<String> = vec![];

    for target in targets {
        //TODO: add user agent tag
        let scan_req = RunScansRequest {
            target: target.clone(),
            jwt: req.jwt.clone(),
        };
        match orchestrator.run_scans(ctx, &scan_req).await {
            Ok(success) => {
                if !success {
                    error!("Failed to begin scan: {target}");
                    failures.push(target)
                }

            }
            Err(e) => {
                error!("Failed to begin scan: {target}");
                error!("{e}");
                failures.push(target)
            },
        }
    }

    if failures.is_empty() {
        Ok(HttpResponse::ok(vec![]))
    } else {
        let mut error_string = String::from("The following targets failed to begin scanning:");
        failures
            .into_iter()
            .for_each(|f| error_string.extend(format!("\n\t{f}").chars()));
        Ok(HttpResponse::internal_server_error(error_string))
    }
}

async fn get_reports(ctx: &Context, req: GetReportsRequest) -> RpcResult<HttpResponse> {
    let report_writer: ReportWriterSender<_> = ReportWriterSender::to_actor(REPORT_WRITER_ACTOR);
    info!("{req:#?}");
    match report_writer.get_reports(ctx, &req).await {
        Ok(reports_result) => match reports_result.result() {
            Ok(reports) => {
                let reports = reports.to_owned().unwrap_or(vec![]);
                match serde_json::to_vec(&reports) {
                    Ok(reports) => {
                        Ok(HttpResponse::json(reports, 200).unwrap_or(HttpResponse::not_found()))
                    }
                    Err(e) => {
                        error!("Error serialising reports: {e}");
                        Ok(HttpResponse::not_found())
                    }
                }
            }
            Err(e) => {
                error!("Error retrieving reports: {e}");
                Ok(HttpResponse::not_found())
            }
        },
        Err(e) => {
            error!("Error retrieving reports: {e}");
            Ok(HttpResponse::not_found())
        }
    }
}

impl From<HttpRequest> for RequestType {
    fn from(req: HttpRequest) -> Self {
        let path = req.path.trim_matches(|c| c == ' ' || c == '/');
        let method = req.method.to_ascii_uppercase();
        match (method.as_str(), path) {
            ("POST", "scan") => match serde_json::from_slice::<ScanRequest>(&req.body) {
                Ok(mut scan_request) => {
                    if let Some(jwt) = auth::get_jwt_from_headers(&req.header) {
                        scan_request.jwt = jwt;
                        Self::Scan(scan_request)
                    } else {
                        Self::Unauthorised
                    }
                }
                Err(e) => Self::Invalid(anyhow!("Invalid body for scan request: {e}")),
            },
            ("GET", "reports") => match serde_json::from_slice::<GetReportsRequest>(&req.body) {
                Ok(mut reports_request) => {
                    if let Some(jwt) = auth::get_jwt_from_headers(&req.header) {
                        reports_request.jwt = jwt;
                        Self::GetReports(reports_request)
                    } else {
                        Self::Unauthorised
                    }
                }
                Err(e) => Self::Invalid(anyhow!("Invalid body for reports request: {e}")),
            },
            ("POST", "sign_in") => match serde_json::from_slice::<AuthParams>(&req.body) {
                Ok(credentials) => Self::SignIn(credentials),
                Err(e) => Self::Invalid(anyhow!("Invalid body for sign_in request: {e}")),
            },
            ("POST", "sign_up") => match serde_json::from_slice::<AuthParams>(&req.body) {
                Ok(credentials) => Self::SignUp(credentials),
                Err(e) => Self::Invalid(anyhow!("Invalid body for sign_up request: {e}")),
            },
            // ("POST", "auth") => Self::Auth(req.header),
            // ("GET", "sign_in") => Self::LoginPage,
            _ => Self::Invalid(anyhow!("Invalid method or path {method}: {path}")),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::collections::HashMap;
    use wasmbus_rpc::Timestamp;
    use webassembly_test::webassembly_test;

    #[webassembly_test]
    fn test_parse_req_type_scan() {
        let valid_scan_req = ScanRequest {
            targets: vec!["www.google.com", "www.github.com", "www.cosmonic.com"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            jwt: "test".to_string(),
            user_agent_tag: None,
        };
        let valid_http_req = HttpRequest {
            body: serde_json::to_vec(&valid_scan_req).unwrap(),
            method: "post".to_string(),
            path: "/scan".to_string(),
            header: HashMap::new(),
            query_string: String::new(),
        };

        let parsed_req = RequestType::from(valid_http_req);
        if let RequestType::Scan(req) = parsed_req {
            assert_eq!(valid_scan_req, req);
        } else {
            panic!();
        }
    }

    #[webassembly_test]
    fn test_parse_req_type_get_reports() {
        let valid_get_reports_req = GetReportsRequest {
            jwt: "test".to_string(),
            target: vec!["www.google.com".to_string()],
            start_timestamp: None,
            end_timestamp: Some(Timestamp::new(420, 69).unwrap()),
        };

        let valid_http_req = HttpRequest {
            body: serde_json::to_vec(&valid_get_reports_req).unwrap(),
            method: "post".to_string(),
            path: "/reports".to_string(),
            header: HashMap::new(),
            query_string: String::new(),
        };

        let parsed_req = RequestType::from(valid_http_req);
        if let RequestType::GetReports(req) = parsed_req {
            assert_eq!(valid_get_reports_req, req);
        } else {
            panic!();
        }
    }

    #[webassembly_test]
    fn test_parse_req_type_invalid() {
        let valid_scan_req = ScanRequest {
            targets: vec!["www.google.com", "www.github.com", "www.cosmonic.com"]
                .into_iter()
                .map(|s| s.to_string())
                .collect(),
            jwt: "test".to_string(),
            user_agent_tag: None,
        };
        let valid_get_reports_req = GetReportsRequest {
            jwt: "test".to_string(),
            target: vec!["www.google.com".to_string()],
            start_timestamp: None,
            end_timestamp: Some(Timestamp::new(420, 69).unwrap()),
        };
        let invalid_http_requests = vec![
            HttpRequest {
                body: serde_json::to_vec(&valid_get_reports_req).unwrap(),
                method: "post".to_string(),
                path: "/scan".to_string(),
                header: HashMap::new(),
                query_string: String::new(),
            },
            HttpRequest {
                body: serde_json::to_vec(&valid_scan_req).unwrap(),
                method: "get".to_string(),
                path: "/scan".to_string(),
                header: HashMap::new(),
                query_string: String::new(),
            },
            HttpRequest {
                body: serde_json::to_vec(&valid_scan_req).unwrap(),
                method: "post".to_string(),
                path: "/reports".to_string(),
                header: HashMap::new(),
                query_string: String::new(),
            },
            HttpRequest {
                body: serde_json::to_vec(&valid_scan_req).unwrap(),
                method: "post".to_string(),
                path: "/reports".to_string(),
                header: HashMap::new(),
                query_string: String::new(),
            },
            HttpRequest {
                body: serde_json::to_vec(&valid_scan_req).unwrap(),
                method: "postsdfs".to_string(),
                path: "/invalid_path".to_string(),
                header: HashMap::new(),
                query_string: String::new(),
            },
        ];

        for req in invalid_http_requests {
            match RequestType::from(req) {
                RequestType::Invalid(_) => {}
                _ => panic!(),
            }
        }
    }
}

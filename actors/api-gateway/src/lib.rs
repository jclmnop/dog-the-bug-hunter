use anyhow::{anyhow, Error};
use dtbh_interface::api_gateway_prelude::*;
use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_httpserver::{
    HttpRequest, HttpResponse, HttpServer, HttpServerReceiver,
};

#[allow(dead_code)]
const CALL_ALIAS: &str = "dtbh/api-gateway";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, HttpServer)]
struct ApiGatewayActor {}

//TODO: auth
//TODO: better error handling
#[async_trait]
impl HttpServer for ApiGatewayActor {
    async fn handle_request(
        &self,
        ctx: &Context,
        req: &HttpRequest,
    ) -> RpcResult<HttpResponse> {
        match RequestType::from(req.to_owned()) {
            RequestType::GetReports(reports_request) => {
                Ok(get_reports(ctx, reports_request)
                    .await
                    .unwrap_or(HttpResponse::not_found()))
            }
            RequestType::Scan(scan_request) => {
                todo!()
            }
            RequestType::Invalid(e) => {
                error!("{e}");
                Ok(HttpResponse::not_found())
            }
        }
    }
}

async fn get_reports(
    ctx: &Context,
    req: GetReportsRequest,
) -> RpcResult<HttpResponse> {
    let report_writer: ReportWriterSender<_> =
        ReportWriterSender::to_actor("dtbh/report-writer");
    match report_writer.get_reports(ctx, &req).await {
        Ok(reports_result) => match reports_result.result() {
            Ok(reports) => {
                let reports = reports.to_owned().unwrap_or(vec![]);
                match serde_json::to_vec(&reports) {
                    Ok(reports) => Ok(HttpResponse::json(reports, 200)
                        .unwrap_or(HttpResponse::not_found())),
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

//TODO: UI request for '/' path
enum RequestType {
    GetReports(GetReportsRequest),
    Scan(ScanRequest),
    Invalid(Error),
}

impl From<HttpRequest> for RequestType {
    fn from(req: HttpRequest) -> Self {
        let path = req.path.trim_matches(|c| c == ' ' || c == '/');
        let method = req.method.to_ascii_uppercase();
        match (method.as_str(), path) {
            ("POST", "scan") => {
                match serde_json::from_slice::<ScanRequest>(&req.body) {
                    Ok(scan_request) => Self::Scan(scan_request),
                    Err(e) => Self::Invalid(anyhow!(
                        "Invalid body for scan request: {e}"
                    )),
                }
            }
            ("POST", "reports") => {
                match serde_json::from_slice::<GetReportsRequest>(&req.body) {
                    Ok(reports_request) => Self::GetReports(reports_request),
                    Err(e) => Self::Invalid(anyhow!(
                        "Invalid body for reports request: {e}"
                    )),
                }
            }
            _ => Self::Invalid(anyhow!(
                "Invalid method or path {method}: {path}"
            )),
        }
    }
}

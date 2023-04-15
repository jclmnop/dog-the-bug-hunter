use dtbh_interface::endpoint_enumerator::{
    EndpointEnumerator, EndpointEnumeratorCallbackReceiver,
    EndpointEnumeratorCallbackReceiverReceiver, EndpointEnumeratorSender,
    EnumerateEndpointsResponse,
};
use dtbh_interface::orchestrator::RunScansRequest;
use dtbh_interface::orchestrator_prelude::*;
use dtbh_interface::scanner_prelude::ScanEndpointParams;
use dtbh_interface::{ReportWriter, WriteReportRequest, REPORT_WRITER_ACTOR, TASKS_TOPIC};
use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_messaging::{Messaging, MessagingSender, PubMessage};

#[allow(dead_code)]
const CALL_ALIAS: &str = "dtbh/orchestrator";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, Orchestrator, EndpointEnumeratorCallbackReceiver)]
struct OrchestratorActor {}

#[async_trait]
impl Orchestrator for OrchestratorActor {
    async fn run_scans(&self, ctx: &Context, req: &RunScansRequest) -> RpcResult<bool> {
        info!("Requesting endpoint enumeration");
        let enumerator = EndpointEnumeratorSender::new();
        enumerator.enumerate_endpoints(ctx, &req).await?;

        Ok(true)
    }
}

#[async_trait]
impl EndpointEnumeratorCallbackReceiver for OrchestratorActor {
    async fn enumerate_endpoints_callback(
        &self,
        ctx: &Context,
        resp: &EnumerateEndpointsResponse,
    ) -> RpcResult<()> {
        info!(
            "Received callback from endpoint enumerator: {:?}",
            resp.target
        );
        if let Some(subdomains) = &resp.subdomains {
            let report_writer_sender: ReportWriterSender<WasmHost> =
                ReportWriterSender::to_actor(REPORT_WRITER_ACTOR);
            match report_writer_sender
                .write_report(
                    ctx,
                    &WriteReportRequest {
                        jwt: resp.jwt.to_string(),
                        report: Report {
                            subdomains: subdomains.to_owned(),
                            target: resp.target.to_owned(),
                            timestamp: resp.timestamp.to_owned(),
                            user_id: "".to_string(), //TODO: remove this field,
                        },
                    },
                )
                .await
            {
                Ok(_) => {
                    info!("Succesfully created new report for {}", resp.target);
                }
                Err(RpcError::Timeout(s) | RpcError::DeadlineExceeded(s)) => {
                    info!(
                        "Timed out creating report for {}, continuing with scans.",
                        resp.target
                    );
                }
                Err(e) => {
                    error!("Error create report for {}: {e}", resp.target);
                    return Ok(());
                }
            }

            for subdomain in subdomains {
                let params = ScanEndpointParams {
                    subdomain: subdomain.clone(),
                    user_agent_tag: None,
                    jwt: resp.jwt.clone(),
                    timestamp: resp.timestamp.clone(),
                    target: resp.target.clone(),
                };
                match serde_json::to_vec(&params) {
                    Ok(body) => {
                        let pub_msg = PubMessage {
                            subject: TASKS_TOPIC.to_string(),
                            reply_to: None,
                            body,
                        };
                        match MessagingSender::new().publish(ctx, &pub_msg).await {
                            Ok(_) => {
                                debug!("Sent message to scanners");
                            }
                            Err(e) => {
                                error!("Failed to publish message: {e}");
                            }
                        };
                    }
                    Err(e) => {
                        error!("Failed to serialise scan params: {e}");
                    }
                };
            }
        }
        Ok(())
    }
}

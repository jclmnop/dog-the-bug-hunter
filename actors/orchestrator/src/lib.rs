use dtbh_interface::endpoint_enumerator::{
    EndpointEnumerator, EndpointEnumeratorCallbackReceiver,
    EndpointEnumeratorCallbackReceiverReceiver, EndpointEnumeratorSender,
    EnumerateEndpointsResponse,
};
use dtbh_interface::orchestrator::RunScansRequest;
use dtbh_interface::orchestrator_prelude::*;
use dtbh_interface::scanner_prelude::ScanEndpointParams;
use dtbh_interface::TASKS_TOPIC;
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
        req: &EnumerateEndpointsResponse,
    ) -> RpcResult<()> {
        info!("Received callback from endpoint enumerator: {:?}", req);
        if let Some(subdomains) = &req.subdomains {
            for subdomain in subdomains {
                let params = ScanEndpointParams {
                    subdomain: subdomain.clone(),
                    user_agent_tag: None,
                    user_id: req.user_id.clone(),
                    timestamp: req.timestamp.clone(),
                    target: req.target.clone(),
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

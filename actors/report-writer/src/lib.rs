use anyhow::Result;
use dtbh_interface::scanner_prelude::*;
use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_httpclient::*;
use wasmcloud_interface_logging::{error, info};

const CALL_ALIAS: &str = "dtb/scanner/<template>";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, HttpEndpointScanner)]
struct TemplateActor {}

#[async_trait]
impl HttpEndpointScanner for TemplateActor {
    async fn scan_endpoint(
        &self,
        ctx: &Context,
        params: &ScanEndpointParams,
    ) -> RpcResult<ScanEndpointResult> {
        todo!("Implement scan_endpoint for this specific vulnerability scanner")
    }
}

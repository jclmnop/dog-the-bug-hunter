use dtbh_interface::scanner_prelude::*;
use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_logging::{error, info};

const CALL_ALIAS: &str = "dtb/scanner/<template>";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, MessageSubscriber)]
struct TemplateActor {}

#[async_trait]
impl ScannerModule for TemplateActor {
    fn name() -> &'static str {
        "template"
    }

    async fn scan(&self, ctx: &Context, params: ScanEndpointParams) -> Result<ScanEndpointResult> {
        todo!()
    }
}

// Wrap `ScannerModule::process_message()` in `MessageSubscriber::handle_message()`, nothing
// else needs to be done for message handling.
#[async_trait]
impl MessageSubscriber for TemplateActor {
    async fn handle_message(&self, ctx: &Context, msg: &SubMessage) -> RpcResult<()> {
        self.process_message(ctx, msg).await
    }
}


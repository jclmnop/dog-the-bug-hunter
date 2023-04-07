use dtbh_interface::scanner_prelude::*;
use wasmbus_rpc::actor::prelude::*;
use once_cell::sync::Lazy;

const CALL_ALIAS: &str = "dtb/scanner/<template>";
static HTTP_CLIENT: Lazy<HttpClientSender<WasmHost>> = Lazy::new(|| HttpClientSender::new());

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, MessageSubscriber)]
struct TemplateActor {}

#[async_trait]
impl ScannerModule for TemplateActor {
    fn name() -> &'static str {
        "template"
    }

    async fn scan(&self, ctx: &Context, target_endpoint: String, user_agent_tag: &Option<String>) -> RpcResult<Option<Finding>> {
        todo!()
    }
}

// Wrap `ScannerModule::process_message()` in `MessageSubscriber::handle_message()`, nothing
// else needs to be done for message handling.
//TODO: derive macro?
#[async_trait]
impl MessageSubscriber for TemplateActor {
    async fn handle_message(
        &self,
        ctx: &Context,
        msg: &SubMessage,
    ) -> RpcResult<()> {
        self.process_message(ctx, msg).await
    }
}

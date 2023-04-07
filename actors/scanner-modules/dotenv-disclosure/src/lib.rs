use std::time::Duration;
use once_cell::sync::Lazy;
use dtbh_interface::scanner_prelude::*;

#[allow(dead_code)]
const CALL_ALIAS: &str = "dtb/scanner/dotenv-disclosure";

static HTTP_CLIENT: Lazy<HttpClientSender<WasmHost>> = Lazy::new(|| {
    let http_client: HttpClientSender<WasmHost> = HttpClientSender::new();
    http_client.set_timeout(Duration::from_secs(1));
    http_client
});

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, MessageSubscriber)]
struct DotEnvActor {}

#[async_trait]
impl ScannerModule for DotEnvActor {
    fn name() -> &'static str {
        "dotenv-disclosure"
    }

    async fn scan(&self, ctx: &Context, target_endpoint: String, user_agent_tag: &Option<String>) -> RpcResult<Option<Finding>> {
        let url = format!("{target_endpoint}/.env");
        let mut req = HttpRequest::get(&url);
        if let Some(tag) = user_agent_tag {
            req.headers.insert("User-Agent".to_string(), vec![tag.to_owned()]);
        }

        let resp = HTTP_CLIENT.request(ctx, &req).await?;
        if resp.status_code >= 200 && resp.status_code <= 299 {
            let finding = Finding {
                finding_type: Self::name().to_string(),
                url
            };
            info!("{finding:#?}");
            Ok(Some(finding))
        } else {
            Ok(None)
        }
    }
}

// Wrap `ScannerModule::process_message()` in `MessageSubscriber::handle_message()`, nothing
// else needs to be done for message handling.
#[async_trait]
impl MessageSubscriber for DotEnvActor {
    async fn handle_message(
        &self,
        ctx: &Context,
        msg: &SubMessage,
    ) -> RpcResult<()> {
        self.process_message(ctx, msg).await
    }
}

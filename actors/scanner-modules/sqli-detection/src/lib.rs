use dtbh_interface::scanner_prelude::*;
use once_cell::sync::Lazy;
use std::borrow::Cow;
use std::time::Duration;

const CALL_ALIAS: &str = "dtb/scanner/sqli-detection";
//TODO: configurable wordlists in KV storage
static HTTP_CLIENT: Lazy<HttpClientSender<WasmHost>> = Lazy::new(|| {
    let http_client: HttpClientSender<WasmHost> = HttpClientSender::new();
    http_client.set_timeout(Duration::from_secs(1));
    http_client
});
static PAYLOADS: Lazy<Vec<String>> = Lazy::new(|| {
    include_str!("../../../../wordlists/url_params_common.txt")
        .lines()
        .map(|param| {
            include_str!("../../../../wordlists/sqli.txt")
                .lines()
                .map(move |sqli_string| {
                    urlencoding::encode(&*format!("?{param}={sqli_string}")).to_string()
                })
        })
        .flatten()
        .collect()
});

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, MessageSubscriber)]
struct SqliDetectionActor {}

#[async_trait]
impl ScannerModule for SqliDetectionActor {
    fn name() -> &'static str {
        "sqli-detection"
    }

    async fn scan(
        &self,
        ctx: &Context,
        target_endpoint: String,
        user_agent_tag: &Option<String>,
    ) -> RpcResult<Option<Finding>> {
        //TODO: stream::iter?
        info!("Scanning {target_endpoint} for SQLi vulnerabilities");
        let mut finding = Finding {
            finding_type: Self::name().to_string(),
            url: String::new(),
        };
        for payload in PAYLOADS.iter() {
            let url = format!("{target_endpoint}/{payload}").to_string();
            let request = HttpRequest {
                method: "GET".to_string(),
                url: url.clone(),
                headers: Default::default(),
                body: vec![],
            };
            let response = HTTP_CLIENT.request(ctx, &request).await;
            if let Ok(response) = response {
                if analyse_response(&response) {
                    finding.url.extend(url.chars());
                }
            }
        }

        if !finding.url.is_empty() {
            Ok(Some(finding))
        } else {
            Ok(None)
        }
    }
}

// Currently just checks for *very* low hanging fruit
fn analyse_response(resp: &HttpResponse) -> bool {
    let body = serde_json::from_slice::<String>(&resp.body);

    if let Ok(body) = body {
        let body = body.to_ascii_lowercase();
        body.contains("sql") || body.contains("syntax") || body.contains("line")
    } else {
        false
    }
}

// Wrap `ScannerModule::process_message()` in `MessageSubscriber::handle_message()`, nothing
// else needs to be done for message handling.
//TODO: derive macro?
#[async_trait]
impl MessageSubscriber for SqliDetectionActor {
    async fn handle_message(&self, ctx: &Context, msg: &SubMessage) -> RpcResult<()> {
        self.process_message(ctx, msg).await
    }
}

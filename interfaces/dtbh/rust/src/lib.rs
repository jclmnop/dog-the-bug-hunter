//! dtbh Interface
pub mod common;
pub use common::*;
pub mod http_endpoint_scanner;
pub use http_endpoint_scanner::*;
pub mod orchestrator;
pub use orchestrator::*;
pub mod report_writer;
pub use report_writer::*;
pub mod endpoint_enumerator;
pub use endpoint_enumerator::*;
pub mod api;
pub use api::*;

pub const TASKS_TOPIC: &str = "dtbh.tasks";
pub const PUB_RESULTS_TOPIC: &str = "dtbh.reports.in";

// #[cfg(all(feature = "actor", target_arch = "wasm32"))]
#[cfg(feature = "actor")]
pub mod scanner_prelude {
    pub use crate::common::*;
    pub use crate::http_endpoint_scanner::{
        HttpEndpointScanner, HttpEndpointScannerReceiver, ScanEndpointParams, ScanEndpointResult,
    };
    use crate::{Report, PUB_RESULTS_TOPIC, TASKS_TOPIC};
    pub use anyhow::Result;
    pub use async_trait::async_trait;
    pub use futures::{stream, StreamExt};
    pub use wasmbus_rpc::actor::prelude::*;
    pub use wasmbus_rpc::common::Context;
    pub use wasmbus_rpc::error::RpcResult;
    pub use wasmcloud_interface_httpclient::*;
    pub use wasmcloud_interface_logging::{debug, error, info};
    pub use wasmcloud_interface_messaging::{
        MessageSubscriber, MessageSubscriberReceiver, Messaging, MessagingSender, PubMessage,
        SubMessage,
    };

    /// Scanner module actor to be wrapped by `MessageHandler`. Only the `scan()`
    /// method is required to be implemented.
    ///
    /// Message handling is already implemented, the `MessageSubscriber` trait
    /// just needs to be implemented, with `MessageSubscriber::handle_message()`
    /// simply wrapping `ScannerModule::process_message()`.
    #[async_trait]
    pub trait ScannerModule: Send + Sync + Default + 'static {
        /// The name of the scanner module
        fn name() -> &'static str;

        /// The topic for publishing results, (default: dtbh.results)
        fn pub_topic() -> &'static str {
            PUB_RESULTS_TOPIC
        }

        #[allow(dead_code)]
        fn sub_topic() -> &'static str {
            TASKS_TOPIC
        }

        /// Process the message received by `handle_message()`
        async fn process_message(&self, ctx: &Context, msg: &SubMessage) -> RpcResult<()> {
            let params: ScanEndpointParams =
                serde_json::from_slice(&msg.body).map_err(|e| RpcError::Deser(e.to_string()))?;
            let (target, jwt, timestamp) = (
                params.target.clone(),
                params.jwt.clone(),
                params.timestamp.clone(),
            );
            let result = match self.scan_all(ctx, params).await {
                Ok(result) => result,
                Err(e) => ScanEndpointResult {
                    subdomain: None,
                    reason: Some(format!("{} failed: {}", Self::name(), e.to_string())),
                    success: false,
                    target,
                    timestamp,
                    jwt,
                },
            };
            match self.publish_result(ctx, result).await {
                Ok(_) => {}
                Err(e) => {
                    error!("Failed to publish result: {}", e.to_string());
                }
            }
            Ok(())
        }

        /// Publish results from a scan to Self::pub_topic()
        async fn publish_result(&self, ctx: &Context, result: ScanEndpointResult) -> Result<()> {
            let topic = Self::pub_topic();
            if result.success {
                let report = Report {
                    subdomains: if let Some(subdomain) = result.subdomain {
                        vec![subdomain]
                    } else {
                        vec![]
                    },
                    target: result.target,
                    timestamp: result.timestamp,
                    user_id: result.jwt,
                };
                let msg = PubMessage {
                    subject: topic.to_string(),
                    body: serde_json::to_vec(&report)?,
                    reply_to: None,
                };
                MessagingSender::new().publish(ctx, &msg).await?;
            }
            Ok(())
        }

        /// Scan endpoints for the vulnerability this scanner specialises in
        async fn scan_all(
            &self,
            ctx: &Context,
            mut params: ScanEndpointParams,
        ) -> Result<ScanEndpointResult> {
            // let user_agent_tag = params.user_agent_tag.unwrap_or("".to_string());
            let url = params.subdomain.subdomain.to_owned();
            params.subdomain.open_ports = stream::iter(
                params
                    .subdomain
                    .open_ports
                    .into_iter()
                    .filter(|p| p.is_open),
            )
            .map(|mut p| {
                let url = format!("http://{url}:{}", p.port);
                async {
                    if let Ok(Some(finding)) = self.scan(ctx, url, &params.user_agent_tag).await {
                        p.findings.push(finding);
                    } //TODO: log error?
                    p
                }
            })
            .buffer_unordered(1)
            .collect()
            .await;

            params.subdomain.open_ports = params
                .subdomain
                .open_ports
                .into_iter()
                .filter(|p| !p.findings.is_empty())
                .collect();

            Ok(ScanEndpointResult {
                reason: None,
                subdomain: Some(params.subdomain),
                success: true,
                target: params.target,
                timestamp: params.timestamp,
                jwt: params.jwt,
            })
        }

        async fn scan(
            &self,
            ctx: &Context,
            target_endpoint: String,
            user_agent_tag: &Option<String>,
        ) -> RpcResult<Option<Finding>>;
    }
}

#[cfg(feature = "actor")]
pub mod orchestrator_prelude {
    pub use crate::common::*;
    pub use crate::http_endpoint_scanner::{
        HttpEndpointScannerSender, ScanEndpointParams, ScanEndpointResult,
    };
    pub use crate::orchestrator::{Orchestrator, OrchestratorReceiver, OrchestratorSender};
    pub use crate::report_writer::{Report, ReportWriterSender, WriteReportResult};
    pub use anyhow::Result;
    pub use async_trait::async_trait;
    pub use futures::{stream, Future, FutureExt};
    pub use wasmcloud_interface_logging::{debug, error, info};
}

#[cfg(feature = "actor")]
pub mod report_writer_prelude {
    pub use crate::common::*;
    pub use crate::report_writer::{Report, ReportWriter, ReportWriterReceiver, WriteReportResult};
    pub use anyhow::Result;
    pub use async_trait::async_trait;
    pub use wasmcloud_interface_logging::{debug, error, info};
}

#[cfg(feature = "actor")]
pub mod api_gateway_prelude {
    pub use crate::api::ScanRequest;
    pub use crate::common::*;
    pub use crate::orchestrator::{Orchestrator, OrchestratorSender, RunScansRequest};
    pub use crate::report_writer::{
        GetReportsRequest, GetReportsResult, ReportWriter, ReportWriterSender,
    };
    pub use crate::Reports;
    use anyhow::anyhow;
    pub use anyhow::Result;
    pub use wasmcloud_interface_logging::{debug, error, info};

    //TODO: make a trait for this and impl for other "result" types?
    impl GetReportsResult {
        pub fn result(&self) -> Result<&Option<Reports>> {
            if self.success {
                Ok(&self.reports)
            } else {
                Err(anyhow!(
                    "Error fetching reports: {}",
                    self.reason.clone().unwrap_or("unknown".to_string())
                ))
            }
        }
    }
}

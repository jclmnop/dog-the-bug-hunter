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
pub const RESULTS_TOPIC: &str = "dtbh.results";

#[cfg(feature = "actor")]
pub mod scanner_prelude {
    pub use crate::common::*;
    pub use crate::http_endpoint_scanner::{
        HttpEndpointScanner, HttpEndpointScannerReceiver, ScanEndpointParams,
        ScanEndpointResult,
    };
    pub use wasmbus_rpc::actor::prelude::*;
    pub use async_trait::async_trait;
    pub use anyhow::Result;
    pub use wasmcloud_interface_logging::{debug, error, info};
    use wasmbus_rpc::common::Context;
    use wasmbus_rpc::error::RpcResult;
    pub use wasmcloud_interface_messaging::{
        MessageSubscriber, MessagingSender, Messaging, PubMessage, SubMessage, MessageSubscriberReceiver,
    };
    pub use wasmcloud_interface_httpclient::*;
    use crate::{RESULTS_TOPIC, TASKS_TOPIC};

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
            RESULTS_TOPIC
        }

        #[allow(dead_code)]
        fn sub_topic() -> &'static str {
            TASKS_TOPIC
        }

        /// Process the message received by `handle_message()`
        async fn process_message(&self, ctx: &Context, msg: &SubMessage) -> RpcResult<()> {
            let params: ScanEndpointParams = serde_json::from_slice(&msg.body).map_err(|e| RpcError::Deser(e.to_string()))?;
            let result = match self.scan(ctx, params).await {
                Ok(result) => result,
                Err(e) => ScanEndpointResult {
                    subdomain: None,
                    reason: Some(format!("{} failed: {}", Self::name(), e.to_string())),
                    success: false,
                }
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
            let msg = PubMessage {
                subject: topic.to_string(),
                body: serde_json::to_vec(&result)?,
                reply_to: None,
            };
            MessagingSender::new().publish(ctx, &msg).await?;
            Ok(())
        }

        /// Scan endpoints for the vulnerability this scanner specialises in
        async fn scan(&self, ctx: &Context, params: ScanEndpointParams) -> Result<ScanEndpointResult>;
    }
}

#[cfg(feature = "actor")]
pub mod orchestrator_prelude {
    pub use anyhow::Result;
    pub use crate::common::*;
    pub use crate::http_endpoint_scanner::{
        HttpEndpointScannerSender, ScanEndpointParams, ScanEndpointResult,
    };
    pub use crate::orchestrator::{
        Orchestrator, OrchestratorReceiver, OrchestratorSender,
    };
    pub use crate::report_writer::{
        Report, ReportWriterSender, WriteReportResult,
    };
    pub use async_trait::async_trait;
    pub use futures::{stream, Future, FutureExt};
    pub use wasmcloud_interface_logging::{debug, error, info};
}

#[cfg(feature = "actor")]
pub mod report_writer_prelude {
    pub use anyhow::Result;
    pub use crate::common::*;
    pub use crate::report_writer::{
        Report, ReportWriter, ReportWriterReceiver, WriteReportResult,
    };
    pub use async_trait::async_trait;
    pub use wasmcloud_interface_logging::{debug, error, info};
}

#[cfg(feature = "actor")]
pub mod api_gateway_prelude {
    use anyhow::anyhow;
    pub use anyhow::Result;
    pub use wasmcloud_interface_logging::{debug, error, info};
    pub use crate::common::*;
    pub use crate::report_writer::{GetReportsRequest, GetReportsResult, ReportWriter, ReportWriterSender};
    pub use crate::orchestrator::{RunScansRequest, OrchestratorSender, Orchestrator};
    pub use crate::api::ScanRequest;
    pub use crate::Reports;

    //TODO: make a trait for this and impl for other "result" types?
    impl GetReportsResult {
        pub fn result<'a>(&'a self) -> Result<&'a Option<Reports>> {
            if self.success {
                Ok(&self.reports)
            } else {
                Err(anyhow!("Error fetching reports: {}", self.reason.clone().unwrap_or("unknown".to_string())))
            }
        }
    }
}

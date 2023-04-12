use anyhow::anyhow;
use dtbh_interface::report_writer_prelude::*;
use dtbh_interface::scanner_prelude::*;
use dtbh_interface::{GetReportsRequest, GetReportsResult, WriteReportRequest};
use serde_json::json;
use wasmcloud_interface_messaging::{
    MessageSubscriber, MessageSubscriberReceiver, Messaging, MessagingReceiver,
};
use wasmcloud_interface_surrealdb::{QueryRequest, RequestScope, SurrealDb, SurrealDbSender};

const CALL_ALIAS: &str = "dtbh/report-writer";
const PUB_TOPIC: &str = "dtbh.reports.out";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, MessageSubscriber, ReportWriter)]
struct ReportActor {}

#[async_trait]
impl ReportWriter for ReportActor {
    async fn write_report(&self, ctx: &Context, req: &WriteReportRequest) -> RpcResult<WriteReportResult> {
        match new_report(ctx, req).await {
            Ok(write_report_result) => Ok(write_report_result),
            Err(e) => Ok(WriteReportResult {
                message: Some(format!("Error creating new report: {e}")),
                success: false,
            }),
        }
    }

    async fn get_reports(
        &self,
        ctx: &Context,
        arg: &GetReportsRequest,
    ) -> RpcResult<GetReportsResult> {
        todo!()
    }
}

#[async_trait]
impl MessageSubscriber for ReportActor {
    /// Topic: `dtbh.reports.in`
    async fn handle_message(&self, ctx: &Context, msg: &SubMessage) -> RpcResult<()> {
        let report_req: WriteReportRequest =
            serde_json::from_slice(&msg.body).map_err(|e| RpcError::Deser(e.to_string()))?;
        let report_json = serde_json::to_string_pretty(&report_req.report)
            .map_err(|e| RpcError::Ser(e.to_string()))?;
        let pub_msg = PubMessage {
            subject: PUB_TOPIC.to_string(),
            reply_to: None,
            body: serde_json::to_vec(&report_json).map_err(|e| RpcError::Ser(e.to_string()))?,
        };
        let publisher: MessagingSender<_> = MessagingSender::new();
        publisher.publish(ctx, &pub_msg).await

        //TODO: write to db
    }
}

async fn new_report(ctx: &Context, req: &WriteReportRequest) -> Result<WriteReportResult> {
    todo!()
}

async fn update_report(
    ctx: &Context,
    WriteReportRequest { report, jwt }: WriteReportRequest,
) -> Result<WriteReportResult> {
    let surreal_db = SurrealDbSender::new();
    //TODO: decode user_id from JWT (in orchestrator)
    let scope = RequestScope {jwt: Some(jwt), ..Default::default()};
    let report_id = format!("{}{}{}", report.user_id, report.target, report.timestamp.as_nanos());
    let sql = r#"
        LET $id = type::thing("reports", crypto::md5($report_id));
        UPDATE $id MERGE timestamp = $timestamp, user_id = $token.ID;
    "#.to_string();

    todo!()
}

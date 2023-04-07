use dtbh_interface::report_writer_prelude::*;
use dtbh_interface::scanner_prelude::*;
use dtbh_interface::{GetReportsRequest, GetReportsResult};
use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_messaging::{
    MessageSubscriber, MessageSubscriberReceiver, Messaging, MessagingReceiver,
};
use wasmcloud_interface_sqldb::{
    ExecuteResult, QueryResult, SqlDbError, SqlDbSender, Statement,
};

const CALL_ALIAS: &str = "dtbh/report-writer";
const PUB_TOPIC: &str = "dtbh.reports.out";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, MessageSubscriber, ReportWriter)]
struct ReportActor {}

#[async_trait]
impl ReportWriter for ReportActor {
    async fn write_report(
        &self,
        ctx: &Context,
        arg: &Report,
    ) -> RpcResult<WriteReportResult> {
        todo!()
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
    async fn handle_message(
        &self,
        ctx: &Context,
        msg: &SubMessage,
    ) -> RpcResult<()> {
        let report: Report = serde_json::from_slice(&msg.body).map_err(|e| RpcError::Deser(e.to_string()))?;
        let report_json = serde_json::to_string_pretty(&report).map_err(|e| RpcError::Ser(e.to_string()))?;
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

async fn write_report_to_db(
    ctx: &Context,
    report: &Report,
) -> Result<WriteReportResult> {
    todo!()
}

//TODO: open a PR for wasmcloud postgres provider to implement prepared statements
fn escape_and_quote(value: &str) -> String {
    // Enough capacity to escape an entire string of `'` or `\` characters and surround it
    // with single quotes
    let mut escaped = String::with_capacity(value.len() * 2 + 2);
    escaped.push('\'');
    for c in value.chars() {
        if c == '\'' || c == '\\' {
            escaped.push('\\');
        }
        escaped.push(c);
    }
    escaped.push('\'');
    escaped
}

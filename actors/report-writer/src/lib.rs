use anyhow::anyhow;
use dtbh_interface::report_writer_prelude::*;
use dtbh_interface::scanner_prelude::*;
use dtbh_interface::{GetReportsRequest, GetReportsResult, WriteReportRequest};
use serde_json::json;
use wasmcloud_interface_messaging::{
    MessageSubscriber, MessageSubscriberReceiver, Messaging, MessagingReceiver,
};
use wasmcloud_interface_surrealdb::{Bindings, Queries, QueryRequest, QueryResponse, RequestScope, SurrealDb, SurrealDbSender};

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

const SQL_CREATE_REPORT: &str = r#"
    BEGIN TRANSACTION;
    LET $report_id = fn::report_id($auth.id, $timestamp, $target);
    CREATE $report_id CONTENT {
        user: $auth.id,
        timestamp: <datetime> $timestamp,
        subdomains: []
    };
"#;

const SQL_CREATE_SUBDOMAIN: &str = r#"
    LET $report_id = fn::report_id($auth.id, $timestamp, $target);
    CREATE subdomain CONTENT {
        subdomain: $subdomain.subdomain,
        report = $report_id,
        open_ports = []
    };
    LET $subdomain_id =
        (SELECT id FROM subdomain
        WHERE subdomain = $subdomain.subdomain and report = $report_id).id;
    UPDATE $report_id MERGE {
        subdomains: array::append($report_id.subdomains, $subdomain_id)
    };
"#;

const SQL_CREATE_PORT: &str = r#"
    LET $report_id = fn::report_id($auth.id, $timestamp, $target);
    LET $subdomain_id =
        (SELECT id FROM subdomain
        WHERE subdomain = $subdomain.subdomain and report = $report_id).id;
    CREATE port CONTENT {
        subdomain: $subdomain_id,
        port: $port.port,
    };
    LET $port_id =
        (SELECT id FROM port
        WHERE subdomain = $subdomain_id AND port = $port.port).id;
    UPDATE $subdomain_id MERGE {
        open_ports: array::append($subdomain_id.open_ports, $port_id)
    };
"#;

const SQL_COMMIT: &str = r#"COMMIT TRANSACTION;"#;
async fn new_report(ctx: &Context, req: &WriteReportRequest) -> Result<WriteReportResult> {
    let surreal_client: SurrealDbSender<WasmHost> = SurrealDbSender::new();
    let mut queries: Queries = vec![];
    let mut bindings: Bindings = vec![];
    let scope = RequestScope { jwt: Some(req.jwt.clone()), ..Default::default() };
    queries.push(SQL_CREATE_REPORT.to_owned());
    bindings.push(
        json!({
            "timestamp": req.report.timestamp.as_nanos(),
            "target": req.report.target
        }).to_string()
    );

    for subdomain in &req.report.subdomains {
        queries.push(SQL_CREATE_SUBDOMAIN.to_owned());
        bindings.push(
            json!({
                "timestamp": req.report.timestamp.as_nanos(),
                "target": req.report.target,
                "subdomain": subdomain
            }).to_string()
        );

        for port in &subdomain.open_ports {
            queries.push(SQL_CREATE_PORT.to_owned());
            bindings.push(
                json!({
                    "timestamp": req.report.timestamp.as_nanos(),
                    "target": req.report.target,
                    "subdomain": subdomain,
                    "port": port
                }).to_string()
            );
        }
    }

    queries.push(SQL_COMMIT.to_string());
    bindings.push("{}".to_string());

    let results = surreal_client.query(ctx, &QueryRequest {
        bindings,
        queries,
        scope: Some(scope),
    }).await?;

    if results.iter().any(|r| !r.errors.is_empty()) {
        Ok(WriteReportResult {
            message: Some("Failed to write all to database".to_string()),
            success: false,
        })
    } else {
        Ok(WriteReportResult {
            message: None,
            success: true,
        })
    }
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

use anyhow::anyhow;
use dtbh_interface::report_writer_prelude::*;
use dtbh_interface::scanner_prelude::*;
use dtbh_interface::{GetReportsRequest, GetReportsResult, WriteReportRequest};
use serde_json::json;
use wasmbus_rpc::Timestamp;
use wasmcloud_interface_logging::error;
use wasmcloud_interface_messaging::{
    MessageSubscriber, MessageSubscriberReceiver, Messaging, MessagingReceiver,
};
use wasmcloud_interface_surrealdb::{
    Bindings, Queries, QueryRequest, QueryResponse, RequestScope, SurrealDb, SurrealDbErrors,
    SurrealDbSender,
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
        req: &WriteReportRequest,
    ) -> RpcResult<WriteReportResult> {
        info!("Writing new report for {}", req.report.target);
        // info!("{:#?}", req.report);
        new_report(ctx, req)
            .await
            .map_err(|e| RpcError::Other(format!("Failed to create new report: {e}")))
        // match new_report(ctx, req).await {
        //     Ok(write_report_result) => Ok(write_report_result),
        //     Err(e) => Ok(WriteReportResult {
        //         message: Some(format!("Error creating new report: {e}")),
        //         success: false,
        //     }),
        // }
    }

    async fn get_reports(
        &self,
        ctx: &Context,
        req: &GetReportsRequest,
    ) -> RpcResult<GetReportsResult> {
        let client: SurrealDbSender<WasmHost> = SurrealDbSender::new();
        let scope = RequestScope {
            jwt: Some(req.jwt.clone()),
            ..Default::default()
        };
        let mut sql = r#"
            SELECT *, fn::to_timestamp(timestamp) AS timestamp FROM report WHERE user = $auth.id AND
        "#
        .to_string();
        if !req.target.is_empty() {
            sql.extend("target INSIDE $targets AND ".chars());
        } else {
            info!("No targets selected, fetching all");
        }
        sql.extend("timestamp >= fn::datetime_from_ns($start) AND ".chars());
        if req.end_timestamp.is_some() {
            sql.extend("timestamp <= fn::datetime_from_ns($end) ".chars());
        } else {
            sql.extend("timestamp <= time::now() ".chars());
        }
        sql.extend("FETCH subdomains, subdomains.open_ports;".chars());
        let start_timestamp = req
            .start_timestamp
            .unwrap_or(Timestamp::new(1681490325, 0)?)
            .as_nanos();
        let end_timestamp = if req.end_timestamp.is_some() {
            req.end_timestamp.unwrap().as_nanos()
        } else {
            u128::default()
        };
        let bindings = vec![json!({
            "start": start_timestamp,
            "end": end_timestamp,
            "targets": req.target
        })
        .to_string()];
        let result = client
            .query(
                ctx,
                &QueryRequest {
                    bindings,
                    queries: vec![sql],
                    scope: Some(scope),
                },
            )
            .await;

        Ok(match result {
            Ok(result) => {
                info!("{} responses", result.len());
                if result.iter().any(|r| !r.errors.is_empty()) {
                    GetReportsResult {
                        reason: Some("Error retrieving report(s)".to_string()),
                        reports: None,
                        success: false,
                    }
                } else {
                    let default_reponse = QueryResponse {
                        errors: vec![],
                        response: vec![],
                    };
                    let response_ser = result.first().unwrap_or(&default_reponse);
                    let mut reports: Vec<Report> = vec![];
                    for report_ser in &response_ser.response {
                        let report_utf8 = std::str::from_utf8(report_ser).unwrap_or("{}");
                        info!("{report_utf8:#?}");
                        if report_utf8 != "[]" {
                            match serde_json::from_slice::<Vec<Report>>(&report_ser) {
                                Ok(report) => {
                                    reports.extend_from_slice(&report);
                                }
                                Err(e) => {
                                    return Ok(GetReportsResult {
                                        reason: Some(format!("Failed to deserialise reports: {e}")),
                                        reports: None,
                                        success: false,
                                    })
                                }
                            }
                        }
                    }
                    info!("{reports:#?}");
                    GetReportsResult {
                        reason: None,
                        reports: Some(reports),
                        success: true,
                    }
                }
            }
            Err(e) => GetReportsResult {
                reason: Some(format!("Failed to fetch reports: {e}")),
                reports: None,
                success: false,
            },
        })
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

        if let Err(e) = update_report(ctx, report_req).await {
            error!("Failed to write report: {e}")
        }
        publisher.publish(ctx, &pub_msg).await?;

        Ok(())
    }
}

async fn new_report(ctx: &Context, req: &WriteReportRequest) -> Result<WriteReportResult> {
    let surreal_client: SurrealDbSender<WasmHost> = SurrealDbSender::new();
    let mut query_string = String::new();
    let scope = RequestScope {
        jwt: Some(req.jwt.clone()),
        ..Default::default()
    };

    //TODO: query builder function
    // Build the query as one string so it can be executed as a transaction
    query_string.extend(SQL_CREATE_REPORT.chars());
    for (i, subdomain) in req.report.subdomains.iter().enumerate() {
        let sql_create_subdomain = SQL_CREATE_SUBDOMAIN.replace("<i>", &i.to_string());
        query_string.extend(sql_create_subdomain.chars());
        for (j, port) in subdomain.open_ports.iter().enumerate() {
            let sql_create_port = SQL_CREATE_PORT.replace("<j>", &j.to_string());
            query_string.extend(sql_create_port.chars());
        }
    }
    //TODO: setup events
    query_string.extend(SQL_COMMIT.chars());

    let bindings = vec![json!({
        "timestamp": req.report.timestamp.as_nanos(),
        "target": req.report.target,
        "subdomains": req.report.subdomains
    })
    .to_string()];
    let queries = vec![query_string];

    let results = surreal_client
        .query(
            ctx,
            &QueryRequest {
                bindings,
                queries,
                scope: Some(scope),
            },
        )
        .await?;

    if results.iter().any(|r| !r.errors.is_empty()) {
        Err(anyhow!(
            "{:#?}",
            results
                .iter()
                .map(|r| r.errors.clone())
                .collect::<Vec<SurrealDbErrors>>()
        ))
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
    let surreal_client = SurrealDbSender::new();
    //TODO: decode user_id from JWT (in orchestrator)
    let scope = RequestScope {
        jwt: Some(jwt),
        ..Default::default()
    };
    let mut query_string = String::new();
    query_string.extend(SQL_BEGIN_UPDATE_REPORT.chars());
    for (i, subdomain) in report.subdomains.iter().enumerate() {
        let sql_update_subdomains = SQL_UPDATE_SUBDOMAIN.replace("<i>", &i.to_string());
        query_string.extend(sql_update_subdomains.chars());
        for (j, port) in subdomain.open_ports.iter().enumerate() {
            if !port.findings.is_empty() {
                let sql_update_port = SQL_UPDATE_PORT.replace("<j>", &j.to_string());
                query_string.extend(sql_update_port.chars());
            }
        }
    }
    query_string.extend(SQL_COMMIT.chars());
    let bindings = vec![json!({
        "timestamp": report.timestamp.as_nanos(),
        "target": report.target,
        "subdomains": report.subdomains
    })
    .to_string()];
    let queries = vec![query_string];

    let results = surreal_client
        .query(
            ctx,
            &QueryRequest {
                bindings,
                queries,
                scope: Some(scope),
            },
        )
        .await?;

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

const SQL_CREATE_REPORT: &str = r#"
    BEGIN;
    LET $report_id = fn::report_id($auth.id, $timestamp, $target);
    CREATE $report_id CONTENT {
        user: $auth.id,
        timestamp: fn::datetime_from_ns($timestamp),
        target: $target,
        subdomains: []
    };
"#;

const SQL_CREATE_SUBDOMAIN: &str = r#"
    LET $subdomain = $subdomains[<i>];
    CREATE subdomain CONTENT {
        subdomain: $subdomain.subdomain,
        report: $report_id,
        openPorts: []
    };
    LET $subdomain_id =
        SELECT VALUE id FROM subdomain
        WHERE subdomain = $subdomain.subdomain and report = $report_id;
    UPDATE $report_id SET subdomains += $subdomain_id[0];
"#;

const SQL_CREATE_PORT: &str = r#"
    LET $port = $subdomain.openPorts[<j>];
    CREATE port CONTENT {
        subdomain: $subdomain_id[0],
        port: $port.port,
        findings: []
    };
    LET $port_id =
        SELECT VALUE id FROM port
        WHERE subdomain = $subdomain_id[0] AND port = $port.port;
    UPDATE $subdomain_id SET openPorts += $port_id[0];
"#;

const SQL_BEGIN_UPDATE_REPORT: &str = r#"
    BEGIN;
    LET $report_id = fn::report_id($auth.id, $timestamp, $target);
"#;
const SQL_UPDATE_SUBDOMAIN: &str = r#"
    LET $subdomain = $subdomains[<i>];
    LET $subdomain_id =
        SELECT VALUE id FROM subdomain
        WHERE subdomain = $subdomain.subdomain and report = $report_id;
"#;
const SQL_UPDATE_PORT: &str = r#"
    LET $port = $subdomain.openPorts[<j>];
    LET $port_id =
        SELECT VALUE id FROM port
        WHERE subdomain = $subdomain_id[0] AND port = $port.port;
    UPDATE $port_id SET findings += $port.findings;
"#;

const SQL_COMMIT: &str = r#"COMMIT;"#;

// async fn new_report(ctx: &Context, req: &WriteReportRequest) -> Result<WriteReportResult> {
//     let surreal_client: SurrealDbSender<WasmHost> = SurrealDbSender::new();
//     let mut queries: Queries = vec![];
//     let mut bindings: Bindings = vec![];
//     let scope = RequestScope { jwt: Some(req.jwt.clone()), ..Default::default() };
//     queries.push(SQL_CREATE_REPORT.to_owned());
//     bindings.push(
//         json!({
//             "timestamp": req.report.timestamp.as_nanos(),
//             "target": req.report.target
//         }).to_string()
//     );
//
//     for subdomain in &req.report.subdomains {
//         queries.push(SQL_CREATE_SUBDOMAIN.to_owned());
//         bindings.push(
//             json!({
//                 "timestamp": req.report.timestamp.as_nanos(),
//                 "target": req.report.target,
//                 "subdomain": subdomain
//             }).to_string()
//         );
//
//         for port in &subdomain.open_ports {
//             queries.push(SQL_CREATE_PORT.to_owned());
//             bindings.push(
//                 json!({
//                     "timestamp": req.report.timestamp.as_nanos(),
//                     "target": req.report.target,
//                     "subdomain": subdomain,
//                     "port": port
//                 }).to_string()
//             );
//         }
//     }
//
//     queries.push(SQL_COMMIT.to_string());
//     bindings.push("{}".to_string());
//
//     let results = surreal_client.query(ctx, &QueryRequest {
//         bindings,
//         queries,
//         scope: Some(scope),
//     }).await?;
//
//     if results.iter().any(|r| !r.errors.is_empty()) {
//         Ok(WriteReportResult {
//             message: Some("Failed to write all to database".to_string()),
//             success: false,
//         })
//     } else {
//         Ok(WriteReportResult {
//             message: None,
//             success: true,
//         })
//     }
// }
// const SQL_CREATE_SUBDOMAIN: &str = r#"
//     LET $report_id = fn::report_id($auth.id, $timestamp, $target);
//     CREATE subdomain CONTENT {
//         subdomain: $subdomain.subdomain,
//         report = $report_id,
//         open_ports = []
//     };
//     LET $subdomain_id =
//         (SELECT id FROM subdomain
//         WHERE subdomain = $subdomain.subdomain and report = $report_id).id;
//     UPDATE $report_id MERGE {
//         subdomains: array::append($report_id.subdomains, $subdomain_id)
//     };
// "#;
//
// const SQL_CREATE_PORT: &str = r#"
//     LET $report_id = fn::report_id($auth.id, $timestamp, $target);
//     LET $subdomain_id =
//         (SELECT id FROM subdomain
//         WHERE subdomain = $subdomain.subdomain and report = $report_id).id;
//     CREATE port CONTENT {
//         subdomain: $subdomain_id,
//         port: $port.port,
//     };
//     LET $port_id =
//         (SELECT id FROM port
//         WHERE subdomain = $subdomain_id AND port = $port.port).id;
//     UPDATE $subdomain_id MERGE {
//         open_ports: array::append($subdomain_id.open_ports, $port_id)
//     };
// "#;

// report_writer.smithy
//

// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "jclmnop.dtbh.interface.report_writer", crate: "crate::report_writer" } ]

namespace jclmnop.dtbh.interface.report_writer

use jclmnop.dtbh.interface.common#Findings
use jclmnop.dtbh.interface.common#Subdomains
use jclmnop.dtbh.interface.api#Targets
use org.wasmcloud.model#wasmbus
use org.wasmcloud.model#U32


/// Write and retrieve reports to keyvalue storage
@wasmbus(
  contractId: "dtbh:reportwriter",
  actorReceive: true
)
service ReportWriter {
  version: "0.1",
  operations: [ WriteReport, GetReports ]
}

/// Create the initial table for the report
operation WriteReport {
  input: WriteReportRequest,
  output: WriteReportResult,
}

structure WriteReportResult {
  success: Boolean,
  message: String,
}

operation GetReports {
  input: GetReportsRequest,
  output: GetReportsResult,
}

structure GetReportsRequest {
  @required
  jwt: String,
  /// If no target is specified, all target reports for the given time range will be returned
  @required
  target: Targets,
  /// If not specified, defaults to earliest report
  startTimestamp: Timestamp,
  /// If not specified, defaults to latest report
  endTimestamp: Timestamp,
}

structure GetReportsResult {
  @required
  success: Boolean,
  reason: String,
  reports: Reports,
}

structure WriteReportRequest {
  @required
  @sensitive
  jwt: String,
  @required
  report: Report,
}

structure Report {
  @required
  timestamp: Timestamp,
  /// The user id is used to identify the user and must be passed in by the api
  @required
  userId: String,
  @required
  target: String,
  /// The findings for the report are stored in the open
  /// ports for each subdomain.
  @required
  subdomains: Subdomains,
}

list Reports {
  member: Report
}

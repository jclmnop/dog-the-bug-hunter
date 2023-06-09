// http_endpoint_scanner.smithy
//

// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "jclmnop.dtbh.interface.http_endpoint_scanner", crate: "crate::http_endpoint_scanner" } ]

namespace jclmnop.dtbh.interface.http_endpoint_scanner

use jclmnop.dtbh.interface.common#Finding
use jclmnop.dtbh.interface.common#Subdomain
use org.wasmcloud.model#wasmbus

/// Scans a target domain for vulnerabilities and generates a report if any
/// are found. Can be triggered by actor to actor calls or message subscriptions.
@wasmbus(
  contractId: "dtbh:scanner",
  actorReceive: true
)
service HttpEndpointScanner {
  version: "0.1",
  operations: [ ScanEndpoint ],
}

/// Scan an endpoint for a vulnerability
operation ScanEndpoint {
  input: ScanEndpointParams,
  output: ScanEndpointResult,
}

/// Params for the scan. This schema will most likely change
structure ScanEndpointParams {
  /// The subdomain (which contains all its open ports) for scanning
  @required
  subdomain: Subdomain,
  /// User ID is passed from the API, currently only used for logging purposes
  @required
  jwt: String,
  @required
  timestamp: Timestamp,
  @required
  target: String
  /// Optional string to be appended to user agent string, usually so the target
  /// is aware of the purpose of requests (an example would be <username>@wearehackerone)
  userAgentTag: String,
}

structure ScanEndpointResult {
  /// False if there was an issue scanning the endpoint
  @required
  success: Boolean,
  reason: String,
  subdomain: Subdomain,
  @required
  jwt: String,
  @required
  timestamp: Timestamp,
  @required
  target: String,
}

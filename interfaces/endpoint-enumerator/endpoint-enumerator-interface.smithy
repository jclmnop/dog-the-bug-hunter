// endpoint-enumerator-interface.smithy

// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "jclmnop.provider.endpoint_enumerator", crate: "wasmcloud-interface-endpoint_enumerator" } ]

namespace jclmnop.provider.endpoint_enumerator

use org.wasmcloud.model#wasmbus
use org.wasmcloud.model#U16

// TODO: move this whole interface into dtbh

@wasmbus(
  contractId: "jclmnop:endpoint_enumerator",
  providerReceive: true,
)
service EndpointEnumerator {
  version: "0.1.0",
  operations: [ EnumerateEndpoints ],
}

@wasmbus(
  contractId: "jclmnop:endpoint_enumerator_callback",
  actorReceive: true,
)
service EndpointEnumeratorCallbackReceiver {
  version: "0.1.0",
  operations: [ EnumerateEndpointsCallback ],
}


/// Takes a target URL, enumerates the endpoints, and eventually calls back with the results.
operation EnumerateEndpoints {
  input: RunScansRequest,
}

/// Receives the results of the EnumerateEndpoints operation.
operation EnumerateEndpointsCallback {
  input: EnumerateEndpointsResponse,
}

structure EnumerateEndpointsResponse {
  /// The list of endpoints that can be scanned for vulnerabilities.
  @required
  success: Boolean,
  /// Timestamp of when the request was received, used later for logs.
  @required
  timestamp: Timestamp,
  @required
  userId: String,
  reason: String,
  subdomains: Subdomains,
}


structure Port {
  @required
  port: U16,
  isOpen: Boolean,
  findings: Findings,
}

list Ports {
  member: Port,
}

structure Subdomain {
  @required
  subdomain: String,
  @required
  openPorts: Ports
}

list Subdomains {
  member: Subdomain,
}

structure Finding {
  @required
  url: String,
  @required
  findingType: String,
}

list Findings {
  member: Finding,
}

// NOTE: from orchestrator
structure RunScansRequest {
  @required
  userId: String,
  /// The target to scan
  @required
  target: String,
}




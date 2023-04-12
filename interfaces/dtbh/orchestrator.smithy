// report_writer.smithy
//

// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "jclmnop.dtbh.interface.orchestrator", crate: "crate::orchestrator" } ]

namespace jclmnop.dtbh.interface.orchestrator

use org.wasmcloud.model#wasmbus
use org.wasmcloud.model#U32


/// Handle the entire process of scanning a target for vulnerabilities
@wasmbus(
  contractId: "dtbh:orchestrator",
  actorReceive: true
)
service Orchestrator {
  version: "0.1",
  operations: [ RunScans ]
}

/// Run scans for a given target
operation RunScans {
  input: RunScansRequest,
  output: Boolean,
}

structure RunScansRequest {
  @required
  jwt: String,
  /// The target to scan
  @required
  target: String,
  //TODO: add user agent tag
}


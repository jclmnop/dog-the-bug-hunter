// report_writer.smithy
//

// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "jclmnop.dtbh.interface.common", crate: "crate::common" } ]

namespace jclmnop.dtbh.interface.common

use org.wasmcloud.model#wasmbus
use org.wasmcloud.model#U16

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

structure Subdomains {
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

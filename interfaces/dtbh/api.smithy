// Tell the code generator how to reference symbols defined in this namespace
metadata package = [ { namespace: "jclmnop.dtbh.interface.api", crate: "crate::api" } ]

namespace jclmnop.dtbh.interface.api

use org.wasmcloud.model#wasmbus
use org.wasmcloud.model#U16

structure ScanRequest {
    @required
    userId: String,
    @required,
    targets: Targets,
    userAgentTag: String,
    // TODO: config for optional fuzzing + specific params?
}

list Targets {
    member: String
}
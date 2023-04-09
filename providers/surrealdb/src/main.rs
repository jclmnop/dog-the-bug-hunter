mod config;
mod error;
mod response;

use async_trait::async_trait;
use wasmbus_rpc::common::Context;
use wasmbus_rpc::error::{RpcError, RpcResult};
use wasmbus_rpc::provider::prelude::*;
use wasmcloud_interface_surrealdb::surrealdb::*;


// main (via provider_main) initializes the threaded tokio executor,
// listens to lattice rpcs, handles actor links,
// and returns only when it receives a shutdown message
//
fn main() -> Result<(), Box<dyn std::error::Error>> {
    provider_main(
        SurrealDbProvider::default(),
        Some("SurrealDb Provider".to_string()),
    )?;

    eprintln!("SurrealDb provider exiting");
    Ok(())
}

/// SurrealDB Capability Provider
/// contractId: "wasmcloud:surrealdb"
#[derive(Default, Clone, Provider)]
#[services(SurrealDb)]
struct SurrealDbProvider {}

/// use default implementations of provider message handlers
impl ProviderDispatch for SurrealDbProvider {}

#[async_trait]
impl ProviderHandler for SurrealDbProvider {
}

#[async_trait]
impl SurrealDb for SurrealDbProvider {
    async fn sign_up(&self, ctx: &Context, arg: &Scope) -> RpcResult<SignUpResponse> {
        todo!()
    }

    async fn query(&self, ctx: &Context, arg: &QueryRequest) -> RpcResult<QueryResponses> {
        todo!()
    }
}

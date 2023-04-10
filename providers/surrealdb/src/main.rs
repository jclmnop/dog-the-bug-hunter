mod config;
mod error;
mod response;

use async_trait::async_trait;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use surrealdb::engine::any::Any;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::{Root, Scope, Signin, Signup};
use surrealdb::Surreal;
use tokio::sync::RwLock;
use tracing::instrument;
use wasmbus_rpc::common::Context;
use wasmbus_rpc::core::{LinkDefinition, HostData};
use wasmbus_rpc::error::{RpcError, RpcResult};
use wasmbus_rpc::provider::prelude::*;
use wasmcloud_interface_surrealdb::surrealdb::Scope as RequestScope;
use wasmcloud_interface_surrealdb::surrealdb::*;

type SurrealClient = Surreal<Client>;

// main (via provider_main) initializes the threaded tokio executor,
// listens to lattice rpcs, handles actor links,
// and returns only when it receives a shutdown message
//
fn main() -> Result<(), Box<dyn std::error::Error>> {
    let host_data: HostData = load_host_data()?;
    provider_main(
        SurrealDbProvider::init(host_data),
        Some("SurrealDb Provider".to_string()),
    )?;

    eprintln!("SurrealDb provider exiting");
    Ok(())
}

/// SurrealDB Capability Provider
/// contractId: "wasmcloud:surrealdb"
#[derive(Default, Clone, Provider)]
#[services(SurrealDb)]
struct SurrealDbProvider {
    actors: Arc<RwLock<HashMap<String, SurrealClient>>>,
}

impl SurrealDbProvider {
    fn init(host_data: HostData) -> Self {
        //TODO: "global" config values
        Self::default()
    }
}

/// use default implementations of provider message handlers
impl ProviderDispatch for SurrealDbProvider {}

#[async_trait]
impl ProviderHandler for SurrealDbProvider {
    /// Provider should perform any operations needed for a new link,
    /// including setting up per-actor resources, and checking authorization.
    /// If the link is allowed, return true, otherwise return false to deny the link.
    #[instrument(level = "debug", skip(self, ld), fields(actor_id = %ld.actor_id))]
    async fn put_link(&self, ld: &LinkDefinition) -> RpcResult<bool> {
        let config = config::LinkConfig::load_config(ld)?;
        let client = Surreal::new::<Ws>(config.get_url())
            .with_capacity(config.concurrency)
            .await
            .map_err(|e| RpcError::ProviderInit(format!("Error connecting to SurrealDB: {e}")))?;

        client
            .signin(Root {
                username: config.user.as_str(),
                password: config.pass.as_str(),
            })
            .await
            .map_err(|e| {
                RpcError::ProviderInit(format!("Error authenticating Root for SurrealDB: {e}"))
            })?;

        let mut update_map = self.actors.write().await;
        update_map.insert(ld.actor_id.to_string(), client);
        Ok(true)
    }

    /// Handle notification that a link is dropped - close the connection
    #[instrument(level = "debug", skip(self))]
    async fn delete_link(&self, actor_id: &str) {
        let mut actors = self.actors.write().await;
        if let Some(client) = actors.remove(actor_id) {
            // drop client for this actor
            drop(client);
        }
    }

    /// Handle shutdown request by closing all connections
    async fn shutdown(&self) -> Result<(), Infallible> {
        let mut actors = self.actors.write().await;
        // close all connections
        for (_, client) in actors.drain() {
            drop(client);
        }
        Ok(())
    }
}

#[async_trait]
impl SurrealDb for SurrealDbProvider {
    async fn sign_up(&self, ctx: &Context, req: &RequestScope) -> RpcResult<SignUpResponse> {
        todo!()
    }

    async fn query(&self, ctx: &Context, req: &QueryRequest) -> RpcResult<QueryResponses> {
        todo!()
    }
}

// async fn get_client(ctx: &Context) ->
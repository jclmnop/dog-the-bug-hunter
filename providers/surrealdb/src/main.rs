mod auth;
mod config;
mod error;
mod response;

use crate::auth::{sign_in, to_scope};
use crate::config::LinkConfig;
use crate::response::Response;
use anyhow::Result;
use async_trait::async_trait;
use std::collections::HashMap;
use std::convert::Infallible;
use std::sync::Arc;
use surrealdb::engine::remote::ws::{Client, Ws};
use surrealdb::opt::auth::Root;
use surrealdb::sql::Value;
use surrealdb::Response as SurrealResponse;
use surrealdb::Surreal;
use tokio::sync::RwLock;
use tracing::instrument;
use wasmbus_rpc::common::Context;
use wasmbus_rpc::core::{HostData, LinkDefinition};
use wasmbus_rpc::error::{RpcError, RpcResult};
use wasmbus_rpc::provider::prelude::*;
use wasmcloud_interface_surrealdb::*;

//TODO: tracing

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
    configs: Arc<RwLock<HashMap<String, LinkConfig>>>,
}

impl SurrealDbProvider {
    fn init(_host_data: HostData) -> Self {
        //TODO: "global" config values
        Self::default()
    }

    async fn get_client(&self, ctx: &Context) -> RpcResult<SurrealClient> {
        let actor_id = actor_id(ctx)?;
        let actors = self.actors.read().await;
        let client = actors.get(actor_id).ok_or_else(|| {
            RpcError::InvalidParameter(format!("No client defined for actor: {actor_id}"))
        })?;
        Ok(client.clone())
    }

    async fn get_config(&self, ctx: &Context) -> RpcResult<LinkConfig> {
        let actor_id = actor_id(ctx)?;
        let configs = self.configs.read().await;
        let config = configs.get(actor_id).ok_or_else(|| {
            RpcError::InvalidParameter(format!("No client defined for actor: {actor_id}"))
        })?;
        Ok(config.clone())
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

        let mut actors = self.actors.write().await;
        actors.insert(ld.actor_id.to_string(), client);
        let mut configs = self.configs.write().await;
        configs.insert(ld.actor_id.to_string(), config);
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
    async fn query(&self, ctx: &Context, req: &QueryRequest) -> RpcResult<QueryResponses> {
        let mut client = self.get_client(ctx).await?;
        let config = self.get_config(ctx).await?;
        sign_in(&req.scope, &config, &mut client)
            .await
            .map_err(|_| RpcError::InvalidParameter("Failed to sign in".into()))?;
        let queries = &req.queries;
        let bindings = parse_bindings(&req.bindings)
            .map_err(|e| RpcError::InvalidParameter(format!("Unable to parse bindings: {e}")))?;
        Ok(send_queries(&client, queries, bindings).await)
    }

    async fn sign_up(&self, ctx: &Context, req: &RequestScope) -> RpcResult<SignUpResponse> {
        let client = self.get_client(ctx).await?;
        let config = self.get_config(ctx).await?;
        let scope = to_scope(req, &config).map_err(|e| RpcError::InvalidParameter(e.to_string()))?;
        Ok(match client.signup(scope).await {
            Ok(token) => match serde_json::to_string(&token) {
                Ok(token) => SignUpResponse {
                    error: None,
                    jwt: Some(token.trim_matches('"').to_string()),
                    success: true,
                },
                Err(e) => SignUpResponse {
                    error: Some(SurrealDbError {
                        message: e.to_string(),
                        name: "JWT_SER_ERROR".to_string(),
                    }),
                    jwt: None,
                    success: false,
                },
            },
            Err(e) => SignUpResponse {
                error: Some(SurrealDbError {
                    message: e.to_string(),
                    name: "SIGNUP_ERROR".to_string(),
                }),
                jwt: None,
                success: false,
            },
        })
    }

    async fn sign_in(&self, ctx: &Context, req: &RequestScope) -> RpcResult<SignInResponse> {
        let client = self.get_client(ctx).await?;
        let config = self.get_config(ctx).await?;
        let scope = to_scope(req, &config).map_err(|e| RpcError::InvalidParameter(e.to_string()))?;
        Ok(match client.signin(scope).await {
            Ok(token) => match serde_json::to_string(&token) {
                Ok(token) => SignInResponse {
                    error: None,
                    jwt: Some(token.trim_matches('"').to_string()),
                    success: true,
                },
                Err(e) => SignInResponse {
                    error: Some(SurrealDbError {
                        message: e.to_string(),
                        name: "JWT_SER_ERROR".to_string(),
                    }),
                    jwt: None,
                    success: false,
                },
            },
            Err(e) => SignInResponse {
                error: Some(SurrealDbError {
                    message: e.to_string(),
                    name: "SIGNIN_ERROR".to_string(),
                }),
                jwt: None,
                success: false,
            },
        })
    }

    async fn authenticate<TS: ToString + ?Sized + Sync>(
        &self,
        ctx: &Context,
        jwt: &TS,
    ) -> RpcResult<AuthenticateResponse> {
        let client = self.get_client(ctx).await?;
        let jwt = jwt.to_string();
        Ok(match client.authenticate(jwt.to_owned()).await {
            Ok(_) => AuthenticateResponse {
                error: None,
                success: true,
            },
            Err(e) => AuthenticateResponse {
                error: Some(SurrealDbError {
                    message: e.to_string(),
                    name: "JWT_AUTH_FAIL".to_string(),
                }),
                success: false,
            },
        })
    }
}

async fn send_queries(
    client: &SurrealClient,
    queries: &Queries,
    bindings: Vec<Value>,
) -> QueryResponses {
    let mut results: Vec<surrealdb::Result<SurrealResponse>> = vec![];
    let iter = queries.iter().zip(bindings.into_iter());
    for (q, b) in iter {
        let r = client.query(q).bind(b).await;
        results.push(r);
    }
    results
        .into_iter()
        .map(|result| match result {
            Ok(response) => QueryResponse::from(Response::from(response)),
            Err(err) => QueryResponse {
                errors: vec![SurrealDbError {
                    message: err.to_string(),
                    name: "QUERY_SEND_ERROR".to_string(),
                }],
                response: vec![],
            },
        })
        .collect()
}

fn parse_bindings(bindings: &[String]) -> surrealdb::Result<Vec<Value>> {
    let parsed = bindings
        .iter()
        .flat_map(|b| Ok::<Value, surrealdb::Error>(surrealdb::sql::json(b)?));
    Ok(parsed.collect())
}

fn actor_id(ctx: &Context) -> RpcResult<&String> {
    ctx.actor
        .as_ref()
        .ok_or_else(|| RpcError::InvalidParameter("no actor in request".into()))
}

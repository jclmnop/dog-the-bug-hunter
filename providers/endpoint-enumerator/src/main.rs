mod common_ports;
mod dns_resolver;
mod port_scanner;
mod subdomains;

use crate::subdomains::enumerate_subdomains;
use anyhow::Result;
use async_trait::async_trait;
use dtbh_interface::common::*;
use dtbh_interface::endpoint_enumerator::*;
use dtbh_interface::orchestrator::RunScansRequest;
use futures::{stream, StreamExt};
use std::collections::HashMap;
use std::str::FromStr;
use std::sync::Arc;
use tokio::sync::{RwLock, Semaphore};
use tracing::{debug, error, info, instrument, trace};
use wasmbus_rpc::common::Context;
use wasmbus_rpc::error::{RpcError, RpcResult};
use wasmbus_rpc::provider::prelude::*;
use wasmbus_rpc::Timestamp;

type ActorId = String;
type WorkPermits = RwLock<HashMap<ActorId, Arc<Semaphore>>>;

// main (via provider_main) initializes the threaded tokio executor,
// listens to lattice rpcs, handles actor links,
// and returns only when it receives a shutdown message
//
fn main() -> Result<(), Box<dyn std::error::Error>> {
    provider_main(
        EndpointEnumeratorProvider::default(),
        Some("endpoint_enumerator Provider".to_string()),
    )?;

    eprintln!("endpoint_enumerator provider exiting");
    Ok(())
}

/// Endpoint enumerator provider
/// contractId: "jclmnop:endpoint_enumerator"
#[derive(Default, Clone, Provider)]
#[services(EndpointEnumerator)]
struct EndpointEnumeratorProvider {
    pub inner: Inner,
}

#[derive(Clone)]
struct Inner {
    /// Semaphore to limit the number of concurrent tasks
    pub work_permits: Arc<WorkPermits>,
}

impl Default for Inner {
    fn default() -> Self {
        Self {
            work_permits: Arc::new(RwLock::new(HashMap::new())),
        }
    }
}

impl Inner {
    pub async fn add_permit(&self, actor_id: String, concurrency: usize) {
        let mut work_permits = self.work_permits.write().await;
        work_permits.insert(actor_id, Arc::new(Semaphore::new(concurrency)));
    }

    pub async fn remove_permit(&self, actor_id: &str) {
        let mut work_permits = self.work_permits.write().await;
        work_permits.remove(actor_id);
    }
}

/// use default implementations of provider message handlers
impl ProviderDispatch for EndpointEnumeratorProvider {}

#[async_trait]
impl ProviderHandler for EndpointEnumeratorProvider {
    /// Add a permit for the actor to perform work with the given concurrency value (default: 1)
    async fn put_link(&self, ld: &LinkDefinition) -> RpcResult<bool> {
        let link_values = &ld.values;
        let actor_id = ld.actor_id.clone();
        let concurrency = link_values
            .get("concurrency")
            .and_then(|v| usize::from_str(v).ok())
            .unwrap_or(1);
        self.inner.add_permit(actor_id.clone(), concurrency).await;
        info!("A maximum of {concurrency} jobs will be processed concurrently for the link to {actor_id}.");
        Ok(true)
    }

    /// Remove the permit for the actor when a link is deleted
    async fn delete_link(&self, actor_id: &str) {
        self.inner.remove_permit(actor_id).await;
        info!("Removed permits for actor {actor_id}.");
    }
}

#[async_trait]
impl EndpointEnumerator for EndpointEnumeratorProvider {
    #[instrument(name = "enumerate_endpoints", skip(self, ctx, req))]
    async fn enumerate_endpoints(
        &self,
        ctx: &Context,
        req: &RunScansRequest,
    ) -> RpcResult<()> {
        let ld = {
            let host_bridge = get_host_bridge();
            let actor_id = ctx.actor.as_ref().ok_or(RpcError::Other(
                "Unable to find actor ID".to_string(),
            ))?;
            host_bridge.get_link(actor_id).await.ok_or(RpcError::Other(
                "Unable to find link definition".to_string(),
            ))?
        };

        let ctx = ctx.to_owned();
        let permit = self.inner.work_permits.clone();
        let req = req.to_owned();
        tokio::task::spawn(async move {
            Self::handle_callback(ctx, req, ld, permit).await
        });
        Ok(())
    }
}

impl EndpointEnumeratorProvider {
    pub const DNS_CONCURRENCY: usize = 100;
    pub const PORT_CONCURRENCY: usize = 100;

    async fn handle_callback(
        ctx: Context,
        req: RunScansRequest,
        link_def: LinkDefinition,
        work_permits: Arc<WorkPermits>,
    ) -> Result<()> {
        let actor =
            EndpointEnumeratorCallbackReceiverSender::for_actor(&link_def);

        // Only allow one request to be processed at a time
        let response = {
            let actor_id = link_def.actor_id.clone();
            let permit = {
                let permits = work_permits.read().await;
                permits
                    .get(&actor_id)
                    .ok_or(RpcError::Other(
                        "Unable to find permit".to_string(),
                    ))?
                    .clone()
            };
            let _permit = permit.acquire().await?;
            Self::process_request(&req).await
        };

        actor.enumerate_endpoints_callback(&ctx, &response).await?;
        Ok(())
    }

    async fn process_request(
        req: &RunScansRequest,
    ) -> EnumerateEndpointsResponse {
        let url = req.target.as_str();
        let user_id = req.user_id.to_owned();
        info!("Enumerating endpoints for {}", url);
        let timestamp = Timestamp::now();
        let subdomains = match Self::enumerate_subdomains(url).await {
            Ok(subdomains) => subdomains,
            Err(e) => {
                error!("Error enumerating subdomains: {}", e);
                return EnumerateEndpointsResponse {
                    reason: Some(e.to_string()),
                    subdomains: None,
                    success: false,
                    timestamp,
                    user_id,
                };
            }
        };

        let subdomains =
            match Self::filter_unresolvable_domains(subdomains).await {
                Ok(subdomains) => subdomains,
                Err(e) => {
                    error!("Error filtering unresolvable domains: {}", e);
                    return EnumerateEndpointsResponse {
                        reason: Some(e.to_string()),
                        subdomains: None,
                        success: false,
                        timestamp,
                        user_id,
                    };
                }
            };

        let subdomains = Self::scan_ports_for_subdomains(subdomains).await;

        EnumerateEndpointsResponse {
            reason: None,
            subdomains: Some(subdomains),
            success: true,
            timestamp,
            user_id,
        }
    }

    #[instrument(name = "enumerate_subdomains")]
    async fn enumerate_subdomains(url: &str) -> Result<Subdomains> {
        info!("Enumerating subdomains for {}", url);
        let mut subdomains = enumerate_subdomains(url).await?;
        let original_subdomain = Subdomain {
            open_ports: vec![],
            subdomain: url.to_string(),
        };
        if !subdomains.contains(&original_subdomain) {
            subdomains.push(original_subdomain);
        }
        Ok(subdomains)
    }

    #[instrument(name = "filter_unresolvable_domains", skip(subdomains))]
    async fn filter_unresolvable_domains(
        subdomains: Subdomains,
    ) -> Result<Subdomains> {
        info!("Filtering unresolvable domains");
        let resolver = dns_resolver::new_resolver()?;
        Ok(stream::iter(subdomains.into_iter())
            .map(|subdomain| dns_resolver::resolves(&resolver, subdomain))
            .buffer_unordered(Self::DNS_CONCURRENCY)
            .filter_map(|subdomain| async move { subdomain })
            .collect()
            .await)
    }

    #[instrument(name = "scan_ports_for_subdomains", skip(subdomains))]
    async fn scan_ports_for_subdomains(subdomains: Subdomains) -> Subdomains {
        info!("Scanning ports for subdomains...");
        debug!("Scanning ports for {} subdomains", subdomains.len());
        trace!("Subdomains: {:#?}", subdomains);
        stream::iter(subdomains.into_iter())
            .map(|subdomain| {
                port_scanner::scan_ports(Self::PORT_CONCURRENCY, subdomain)
            })
            .buffer_unordered(1)
            .filter_map(|subdomain| async move {
                if let Err(e) = &subdomain {
                    error!("Error scanning ports: {}", e);
                };
                subdomain.ok()
            })
            .collect()
            .await
    }
}

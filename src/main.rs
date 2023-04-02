//! sleepy capability provider
//!
//!
mod common_ports;
mod dns_resolver;
mod port_scanner;
mod subdomains;

use crate::subdomains::enumerate_subdomains;
use anyhow::Result;
use async_trait::async_trait;
use futures::{stream, StreamExt};
use tracing::{debug, error, info, instrument, trace};
use wasmbus_rpc::common::Context;
use wasmbus_rpc::error::{RpcError, RpcResult};
use wasmbus_rpc::provider::prelude::*;
use wasmcloud_interface_endpoint_enumerator::*;

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

/// Sleepy capability provider implementation
/// contractId: "jclmnop:sleepy"
#[derive(Default, Clone, Provider)]
#[services(EndpointEnumerator)]
struct EndpointEnumeratorProvider {}

/// use default implementations of provider message handlers
impl ProviderDispatch for EndpointEnumeratorProvider {}
impl ProviderHandler for EndpointEnumeratorProvider {}

#[async_trait]
impl EndpointEnumerator for EndpointEnumeratorProvider {
    #[instrument(name = "enumerate_endpoints", skip(self, ctx, url))]
    async fn enumerate_endpoints<TS: ToString + ?Sized + Sync>(
        &self,
        ctx: &Context,
        url: &TS,
    ) -> RpcResult<()> {
        let url = url.to_string();

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
        tokio::task::spawn(
            async move { Self::handle_callback(ctx, url, ld).await },
        );
        Ok(())
    }
}

impl EndpointEnumeratorProvider {
    pub const DNS_CONCURRENCY: usize = 100;
    pub const PORT_CONCURRENCY: usize = 100;

    async fn handle_callback(
        ctx: Context,
        url: String,
        link_def: LinkDefinition,
    ) -> Result<()> {
        let actor =
            EndpointEnumeratorCallbackReceiverSender::for_actor(&link_def);
        let response = Self::process_request(url.as_str()).await;
        actor.enumerate_endpoints_callback(&ctx, &response).await?;
        Ok(())
    }

    async fn process_request(url: &str) -> EnumerateEndpointsResponse {
        info!("Enumerating endpoints for {}", url);
        let subdomains = match Self::enumerate_subdomains(url).await {
            Ok(subdomains) => subdomains,
            Err(e) => {
                error!("Error enumerating subdomains: {}", e);
                return EnumerateEndpointsResponse {
                    reason: Some(e.to_string()),
                    subdomains: None,
                    success: false,
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
                    };
                }
            };

        let subdomains = Self::scan_ports_for_subdomains(subdomains).await;

        EnumerateEndpointsResponse {
            reason: None,
            subdomains: Some(subdomains),
            success: true,
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

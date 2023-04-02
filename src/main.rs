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
use futures::{stream, FutureExt, StreamExt};
use tracing::{error, info};
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
    async fn enumerate_endpoints<TS: ToString + ?Sized + Sync>(
        &self,
        ctx: &Context,
        url: &TS,
    ) -> RpcResult<EnumerateEndpointsResponse> {
        let url = &*url.to_string();
        let subdomains = match self.enumerate_subdomains(url).await {
            Ok(subdomains) => subdomains,
            Err(e) => {
                error!("Error enumerating subdomains: {}", e);
                return Ok(EnumerateEndpointsResponse {
                    reason: Some(e.to_string()),
                    subdomains: None,
                    success: false,
                });
            }
        };

        let subdomains =
            match self.filter_unresolvable_domains(subdomains).await {
                Ok(subdomains) => subdomains,
                Err(e) => {
                    error!("Error filtering unresolvable domains: {}", e);
                    return Ok(EnumerateEndpointsResponse {
                        reason: Some(e.to_string()),
                        subdomains: None,
                        success: false,
                    });
                }
            };

        let subdomains = self.scan_ports_for_subdomains(subdomains).await;

        Ok(EnumerateEndpointsResponse {
            reason: None,
            subdomains: Some(subdomains),
            success: true,
        })
    }
}

impl EndpointEnumeratorProvider {
    pub const DNS_CONCURRENCY: usize = 100;
    pub const PORT_CONCURRENCY: usize = 100;
    async fn enumerate_subdomains(&self, url: &str) -> Result<Subdomains> {
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

    async fn filter_unresolvable_domains(
        &self,
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

    async fn scan_ports_for_subdomains(
        &self,
        subdomains: Subdomains,
    ) -> Subdomains {
        info!("Scanning ports for subdomains");
        stream::iter(subdomains.into_iter())
            .map(|subdomain| {
                port_scanner::scan_ports(Self::PORT_CONCURRENCY, subdomain)
            })
            .buffer_unordered(1)
            .filter_map(|subdomain| async move {
                if let Err(e) = &subdomain {
                    error!("Error scanning ports for {}", e);
                };
                subdomain.ok()
            })
            .collect()
            .await
    }
}

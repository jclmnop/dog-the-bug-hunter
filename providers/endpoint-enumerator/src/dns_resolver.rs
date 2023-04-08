use crate::Subdomain;
use anyhow::Result;
use std::sync::Arc;
use std::time::Duration;
use trust_dns_resolver::config::{ResolverConfig, ResolverOpts};
use trust_dns_resolver::name_server::{GenericConnection, GenericConnectionProvider, TokioRuntime};
use trust_dns_resolver::AsyncResolver;

pub type Resolver = Arc<AsyncResolver<GenericConnection, GenericConnectionProvider<TokioRuntime>>>;

pub async fn resolves(resolver: &Resolver, subdomain: Subdomain) -> Option<Subdomain> {
    if resolver
        .lookup_ip(subdomain.subdomain.as_str())
        .await
        .is_ok()
    {
        Some(subdomain)
    } else {
        None
    }
}

pub fn new_resolver() -> Result<Resolver> {
    let config = ResolverConfig::default();
    let mut opts = ResolverOpts::default();
    opts.timeout = Duration::from_secs(5);

    let resolver = AsyncResolver::tokio(config, opts)?;

    Ok(Arc::new(resolver))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::Subdomain;

    #[tokio::test]
    async fn test_resolves() {
        let subdomain = Subdomain {
            open_ports: vec![],
            subdomain: "github.com".to_string(),
        };
        let resolver = new_resolver().unwrap();
        let subdomain = resolves(&resolver, subdomain).await;
        assert!(subdomain.is_some());
    }
}

use crate::common_ports::COMMON_PORTS;
use anyhow::{anyhow, Result};
use dtbh_interface::common::{Port, Subdomain};
use futures::{stream, StreamExt};
use std::net::{SocketAddr, ToSocketAddrs};
use std::time::Duration;
use tokio::net::TcpStream;
use tokio::time::timeout;
use tracing::{info, instrument, trace, warn};

#[instrument(level = "info", name = "scan_ports", fields(subdomain = %subdomain.subdomain))]
pub async fn scan_ports(concurrency: usize, mut subdomain: Subdomain) -> Result<Subdomain> {
    info!("Scanning ports for {}", subdomain.subdomain);
    let socket_addresses: Vec<SocketAddr> = format!("{}:1024", subdomain.subdomain)
        .to_socket_addrs()
        .map_err(|e| anyhow!("\nsubdomain:{}\n{e}", subdomain.subdomain))?
        .collect();

    if socket_addresses.is_empty() {
        warn!("No socket addresses found for {}", subdomain.subdomain);
        return Ok(subdomain);
    }

    let socket_address = socket_addresses[0];
    subdomain.open_ports = stream::iter(COMMON_PORTS.into_iter())
        .map(|port| async move {
            let port = scan_port(socket_address, port).await;
            if port.is_open {
                Some(port)
            } else {
                None
            }
        })
        .buffer_unordered(concurrency)
        .filter_map(|port| async { port })
        .collect()
        .await;

    Ok(subdomain)
}

#[instrument(level = "trace", name = "scan_port", fields(port = %port))]
pub async fn scan_port(mut socket_address: SocketAddr, port: u16) -> Port {
    trace!("Scanning port {}", port);
    let timeout_limit = Duration::from_secs(3);
    socket_address.set_port(port);

    if let Ok(Ok(_)) = timeout(timeout_limit, TcpStream::connect(&socket_address)).await {
        Port {
            findings: vec![],
            is_open: true,
            port,
        }
    } else {
        Port {
            findings: vec![],
            is_open: false,
            port,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::net::Ipv4Addr;

    #[tokio::test]
    async fn test_scan_port() {
        let socket_address = SocketAddr::new(Ipv4Addr::new(127, 0, 0, 1).into(), 1024);
        let port = scan_port(socket_address, 80).await;
        assert_eq!(port.port, 80);
        assert!(!port.is_open);
    }

    // NOTE: moved to integration tests
    // #[tokio::test]
    // async fn test_scan_ports() {
    //     start_logger();
    //
    //     // Run the docker-compose up -d command
    //     // Guard ensures that the docker-compose down command is run when the test
    //     // completes, even if it panics
    //     let _docker_guard = start_docker();
    //
    //     // Run the port scan and hold onto the result
    //     let subdomain = Subdomain {
    //         open_ports: vec![],
    //         subdomain: "localhost".to_string(),
    //     };
    //
    //     let result = scan_ports(1, subdomain).await;
    //
    //     // Assert
    //     assert!(result.is_ok());
    //     let subdomain = result.unwrap();
    //     info!("subdomain: {:?}", subdomain);
    //
    //     assert_eq!(subdomain.subdomain, "localhost");
    //     assert!(subdomain.open_ports.len() >= 3);
    //
    //     assert!(subdomain.open_ports.contains(&Port {
    //         findings: None,
    //         is_open: true,
    //         port: 8000
    //     }));
    //
    //     assert!(subdomain.open_ports.contains(&Port {
    //         findings: None,
    //         is_open: true,
    //         port: 8001
    //     }));
    //
    //     assert!(subdomain.open_ports.contains(&Port {
    //         findings: None,
    //         is_open: true,
    //         port: 8002
    //     }));
    // }
}

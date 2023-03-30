//! sleepy capability provider
//!
//!
use async_trait::async_trait;
use std::time::SystemTime;
use tokio::time::{sleep, Duration};
use wasmbus_rpc::common::Context;
use wasmbus_rpc::error::{RpcError, RpcResult};
use wasmbus_rpc::provider::prelude::*;
use wasmbus_rpc::Timestamp;
use wasmcloud_interface_sleepy::{Sleepy, SleepyReceiver};

// main (via provider_main) initializes the threaded tokio executor,
// listens to lattice rpcs, handles actor links,
// and returns only when it receives a shutdown message
//
fn main() -> Result<(), Box<dyn std::error::Error>> {
    provider_main(
        SleepyProvider::default(),
        Some("sleepy Provider".to_string()),
    )?;

    eprintln!("sleepy provider exiting");
    Ok(())
}

/// Sleepy capability provider implementation
/// contractId: "jclmnop:sleepy"
#[derive(Default, Clone, Provider)]
#[services(Sleepy)]
struct SleepyProvider {}

/// use default implementations of provider message handlers
impl ProviderDispatch for SleepyProvider {}
impl ProviderHandler for SleepyProvider {}

#[async_trait]
impl Sleepy for SleepyProvider {
    async fn sleep(&self, _ctx: &Context, duration_ms: &u32) -> RpcResult<()> {
        let duration = Duration::from_millis(*duration_ms as u64);
        sleep(duration).await;
        Ok(())
    }

    async fn sleep_until(
        &self,
        _ctx: &Context,
        timestamp: &Timestamp,
    ) -> RpcResult<()> {
        let now_duration = SystemTime::now()
            .duration_since(SystemTime::UNIX_EPOCH)
            .map_err(|e| RpcError::from(format!("System time error: {e}")))?;
        let end_duration = Duration::new(timestamp.sec as u64, timestamp.nsec);
        let sleep_duration = end_duration - now_duration;
        sleep(sleep_duration).await;
        Ok(())
    }
}

// /// Handle Factorial methods
// #[async_trait]
// impl Factorial for sleepyProvider {
//     /// accepts a number and calculates its factorial
//     async fn calculate(&self, _ctx: &Context, req: &u32) -> RpcResult<u64> {
//         debug!("processing request calculate ({})", *req);
//         Ok(n_factorial(*req))
//     }
// }
//
// /// calculate n factorial
// fn n_factorial(n: u32) -> u64 {
//     match n {
//         0 => 1,
//         1 => 1,
//         _ => {
//             let mut result = 1u64;
//             // add 1 because rust ranges exclude upper bound
//             for v in 2..(n + 1) {
//                 result *= v as u64;
//             }
//             result
//         }
//     }
// }

mod utils;

use futures::StreamExt;
use tracing::info;
use tracing::log::error;
use utils::*;
use wasmbus_rpc::common::deserialize;
use wasmbus_rpc::core::Invocation;
use wasmbus_rpc::error::RpcResult;
use wasmbus_rpc::provider::prelude::*;
use wasmcloud_interface_endpoint_enumerator::{EndpointEnumerator, EndpointEnumeratorSender, EnumerateEndpointsResponse, Port, Subdomains};
use wasmcloud_test_util::{
    check,
    cli::print_test_results,
    provider_test::test_provider,
    testing::TestOptions,
};
#[allow(unused_imports)]
use wasmcloud_test_util::{run_selected, run_selected_spawn};

#[tokio::test]
async fn run_all() {
    // start_logger();
    let opts = TestOptions::default();
    let res =
        run_selected_spawn!(&opts, test_health_check, test_enumerate_endpoints);
    print_test_results(&res);

    let passed = res.iter().filter(|tr| tr.passed).count();
    let total = res.len();
    assert_eq!(passed, total, "{} passed out of {}", passed, total);

    // try to let the provider shut down gracefully
    let provider = test_provider().await;
    let _ = provider.shutdown().await;
}

/// A mock actor to receive the callback from the provider once endpoints are enumerated
async fn mock_callback_actor(
) -> tokio::task::JoinHandle<RpcResult<Option<EnumerateEndpointsResponse>>> {
    let handle = tokio::runtime::Handle::current();

    handle.spawn(async move {
        let prov = test_provider().await;
        let topic = prov.mock_actor_rpc_topic();
        let mut sub = prov
            .nats_client
            .subscribe(topic)
            .await
            .map_err(|e| RpcError::Nats(e.to_string()))?;
        let mut response = None;
        while let Some(msg) = sub.next().await {
            let inv: Invocation = deserialize(&msg.payload)
                .map_err(|e| RpcError::Deser(e.to_string()))?;
            if &inv.operation != "EndpointEnumeratorCallbackReceiver.EnumerateEndpointsCallback" {
                error!("unexpected invocation: {:?}", &inv);
            } else {
                info!("Callback received!");
                response = deserialize(&inv.msg)
                    .map_err(|e| RpcError::Deser(e.to_string()))?;
                break;
            }
        }
        Ok(response)
    })
}

/// test that health check returns healthy
async fn test_health_check(_opt: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;

    // health check
    let hc = prov.health_check().await;
    check!(hc.is_ok())?;
    Ok(())
}

/// test that endpoints are enumerated and callback is successful with expected results
async fn test_enumerate_endpoints(_opt: &TestOptions) -> RpcResult<Subdomains> {
    let prov = test_provider().await;
    let actor = mock_callback_actor().await;
    let _guard = start_docker();

    let client = EndpointEnumeratorSender::via(prov);
    let ctx = Context::default();

    let url = "127.0.0.1";
    // let url = "github.com";
    client.enumerate_endpoints(&ctx, &url).await?;
    let res = actor
        .await
        .map_err(|e| RpcError::Other(e.to_string()))??
        .expect("expected a response from the mock actor");

    check!(res.success)?;
    check!(res.reason.is_none())?;
    check!(res.subdomains.is_some())?;
    let subdomains = res.subdomains.unwrap();
    info!("subdomains: {:#?}", subdomains);

    let subdomain = subdomains.first().unwrap();
    check!(subdomain.open_ports.len() >= 3)?;

    check!(subdomain.open_ports.contains(&Port {
        findings: None,
        is_open: true,
        port: 8000
    }))?;

    check!(subdomain.open_ports.contains(&Port {
        findings: None,
        is_open: true,
        port: 8001
    }))?;

    check!(subdomain.open_ports.contains(&Port {
        findings: None,
        is_open: true,
        port: 8002
    }))?;

    Ok(subdomains)
}
//
// /// test that `SleepySender::sleep()` works correctly
// async fn test_sleep(_opt: &TestOptions) -> RpcResult<()> {
//     let prov = test_provider().await;
//
//     let client = SleepySender::via(prov);
//     let ctx = Context::default();
//
//     let start = tokio::time::Instant::now();
//     let sleep_time_ms = 100;
//     let _ = client.sleep(&ctx, &sleep_time_ms).await?;
//     let actual_time_slept = start.elapsed();
//
//     check!(
//         actual_time_slept >= Duration::from_millis(sleep_time_ms as u64)
//     )?;
//
//     Ok(())
// }
//
// /// test that `SleepySender::sleep_until()` works correctly
// async fn test_sleep_until(_opt: &TestOptions) -> RpcResult<()> {
//     let prov = test_provider().await;
//
//     let client = SleepySender::via(prov);
//     let ctx = Context::default();
//
//     let start = tokio::time::Instant::now();
//     let sleep_duration = Duration::from_millis(100);
//     let sys_timestamp = SystemTime::now();
//     let sleep_until = Timestamp::from(sys_timestamp.add(sleep_duration));
//     let _ = client.sleep_until(&ctx, &sleep_until).await?;
//     let actual_time_slept = start.elapsed();
//
//     check!(
//         actual_time_slept >= sleep_duration
//     )?;
//
//     Ok(())
// }
//
// /// test that `SleepySender::now()` works correctly
// async fn test_now(_opt: &TestOptions) -> RpcResult<()> {
//     let prov = test_provider().await;
//
//     let client = SleepySender::via(prov);
//     let ctx = Context::default();
//
//     let start = client.now(&ctx).await?;
//     let sleep_duration = Duration::from_millis(100);
//     tokio::time::sleep(sleep_duration).await;
//     let end = client.now(&ctx).await?;
//
//     // check that the difference between the start and end times is within 10ms of the sleep duration
//     check!((end.as_nanos() - start.as_nanos()).abs_diff(sleep_duration.as_nanos()) < 10_000_000)?;
//
//     Ok(())
// }

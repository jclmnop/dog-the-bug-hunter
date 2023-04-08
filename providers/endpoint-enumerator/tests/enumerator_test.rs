mod utils;

use dtbh_interface::common::{Port, Subdomains};
use dtbh_interface::endpoint_enumerator::{
    EndpointEnumerator, EndpointEnumeratorSender, EnumerateEndpointsResponse,
};
use dtbh_interface::orchestrator::RunScansRequest;
use futures::StreamExt;
use std::collections::HashMap;
use tracing::info;
use tracing::log::error;
use utils::*;
use wasmbus_rpc::common::deserialize;
use wasmbus_rpc::core::Invocation;
use wasmbus_rpc::error::RpcResult;
use wasmbus_rpc::provider::prelude::*;
use wasmcloud_test_util::{
    check, cli::print_test_results, provider_test::test_provider, testing::TestOptions,
};
#[allow(unused_imports)]
use wasmcloud_test_util::{run_selected, run_selected_spawn};

//TODO: fix these tests once some of the wasmcloud interface crates update to wasmbus 0.12.0
//      until then, i can't use this provider in the orchestrator actor with wasmbus 0.12.0
//      and these tests don't work with wasmbus 0.11.1 because wasmcloud_test_utils only
//      mock actor in the latest version, but the latest test utils version isn't compatible
//      with wasmbus 0.11.1

#[tokio::test]
async fn run_all() {
    let _guard = start_docker();

    start_logger();
    let opts = TestOptions::default();
    let res = run_selected_spawn!(
        &opts,
        test_health_check,
        test_enumerate_endpoints,
        test_jobs_queued_sequentially
    );
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
    mut n_requests: u32,
) -> tokio::task::JoinHandle<RpcResult<Vec<EnumerateEndpointsResponse>>> {
    let handle = tokio::runtime::Handle::current();

    handle.spawn(async move {
        let prov = test_provider().await;
        let topic = prov.mock_actor_rpc_topic();
        let mut sub = prov
            .nats_client
            .subscribe(topic)
            .await
            .map_err(|e| RpcError::Nats(e.to_string()))?;
        let mut responses = Vec::new();
        'callback: while let Some(msg) = sub.next().await {
            let inv: Invocation =
                deserialize(&msg.payload).map_err(|e| RpcError::Deser(e.to_string()))?;
            if &inv.operation != "EndpointEnumeratorCallbackReceiver.EnumerateEndpointsCallback" {
                error!("unexpected invocation: {:?}", &inv);
            } else {
                info!("Callback received!");
                let response = deserialize(&inv.msg).map_err(|e| RpcError::Deser(e.to_string()))?;
                responses.push(response);
                n_requests -= 1;
                if n_requests == 0 {
                    break 'callback;
                }
            }
        }
        Ok(responses)
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
    let actor = mock_callback_actor(1).await;

    let client = EndpointEnumeratorSender::via(prov);
    let ctx = Context::default();

    let url = "127.0.0.1";
    let req = RunScansRequest {
        target: url.to_string(),
        user_id: "test".to_string(),
    };
    // let url = "github.com";
    client.enumerate_endpoints(&ctx, &req).await?;
    let res = actor.await.map_err(|e| RpcError::Other(e.to_string()))??;
    let res = res
        .first()
        .expect("expected a response from the mock actor")
        .clone();

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

async fn test_jobs_queued_sequentially(_opt: &TestOptions) -> RpcResult<()> {
    let n_requests = 3;
    let prov = test_provider().await;
    let actor = mock_callback_actor(n_requests).await;

    let client = EndpointEnumeratorSender::via(prov);
    let ctx = Context::default();

    // Send requests in sequential order, without waiting for the response
    let urls: Vec<_> = (1..n_requests + 1)
        .map(|i| format!("127.0.0.{}", i))
        .collect();
    let mut requests = Vec::new();
    for url in &urls {
        info!("sending request for {}", url);
        let req = client.enumerate_endpoints(&ctx, url);
        requests.push(req);
    }

    for req in requests {
        req.await?;
    }

    // Wait for the mock actor to receive and process the responses
    info!("waiting for mock actor to receive and process responses...");
    let responses = actor.await.map_err(|e| RpcError::Other(e.to_string()))??;

    check!(responses.len() == n_requests as usize)?;

    // Check that the responses are in the correct order
    info!("checking responses...");
    for i in 1..n_requests + 1 {
        let url = format!("127.0.0.{}", i);
        let res = responses[i as usize - 1].clone();
        check!(res.success)?;
        check!(res.reason.is_none())?;
        check!(res.subdomains.as_ref().unwrap().first().unwrap().subdomain == url)?;
    }

    Ok(())
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

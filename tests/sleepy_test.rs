use std::ops::Add;
use std::time::{Duration, SystemTime};
use wasmbus_rpc::error::RpcResult;
use wasmbus_rpc::provider::prelude::*;
use wasmbus_rpc::Timestamp;
use wasmcloud_test_util::{
    check,
    cli::print_test_results,
    provider_test::test_provider,
    testing::{TestOptions, TestResult},
};
#[allow(unused_imports)]
use wasmcloud_test_util::{run_selected, run_selected_spawn};
use wasmcloud_interface_sleepy::{Sleepy, SleepySender};

#[tokio::test]
async fn run_all() {
    let opts = TestOptions::default();
    let res = run_selected_spawn!(&opts, health_check, sleep, sleep_until);
    print_test_results(&res);

    let passed = res.iter().filter(|tr| tr.passed).count();
    let total = res.len();
    assert_eq!(passed, total, "{} passed out of {}", passed, total);

    // try to let the provider shut down gracefully
    let provider = test_provider().await;
    let _ = provider.shutdown().await;
}

/// test that health check returns healthy
async fn health_check(_opt: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;

    // health check
    let hc = prov.health_check().await;
    check!(hc.is_ok())?;
    Ok(())
}

/// test that `SleepySender::sleep()` works correctly
async fn sleep(_opt: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;

    let client = SleepySender::via(prov);
    let ctx = Context::default();

    let start = tokio::time::Instant::now();
    let sleep_time_ms = 100;
    let _ = client.sleep(&ctx, &sleep_time_ms).await?;
    let actual_time_slept = start.elapsed();

    assert!(
        actual_time_slept >= Duration::from_millis(sleep_time_ms as u64),
        "Expected: {}, Actual: {}", sleep_time_ms, actual_time_slept.as_millis()
    );

    Ok(())
}

/// test that `SleepySender::sleep_until()` works correctly
async fn sleep_until(_opt: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;

    let client = SleepySender::via(prov);
    let ctx = Context::default();

    let start = tokio::time::Instant::now();
    let sleep_duration = Duration::from_millis(100);
    let sys_timestamp = SystemTime::now();
    let sleep_until = Timestamp::from(sys_timestamp.add(sleep_duration));
    let _ = client.sleep_until(&ctx, &sleep_until).await?;
    let actual_time_slept = start.elapsed();

    assert!(
        actual_time_slept >= sleep_duration,
        "Expected: {}, Actual: {}", sleep_duration.as_millis(), actual_time_slept.as_millis()
    );

    Ok(())
}

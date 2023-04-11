use std::collections::HashMap;
use std::ops::Add;
use std::time::{Duration, SystemTime};
use serde::{Deserialize, Serialize};
use tracing::info;
use wasmbus_rpc::error::RpcResult;
use wasmbus_rpc::provider::prelude::*;
use wasmbus_rpc::Timestamp;
use wasmcloud_interface_surrealdb::{QueryRequest, SurrealDb, SurrealDbSender};
use wasmcloud_test_util::{
    check, check_eq,
    cli::print_test_results,
    provider_test::test_provider,
    testing::{TestOptions, TestResult},
};
#[allow(unused_imports)]
use wasmcloud_test_util::{run_selected, run_selected_spawn};

//TODO: docker container with surrealdb instance

#[tokio::test]
async fn run_all() {
    let opts = TestOptions::default();
    let res = run_selected_spawn!(&opts, test_health_check, test_single_query);
    print_test_results(&res);

    let passed = res.iter().filter(|tr| tr.passed).count();
    let total = res.len();
    assert_eq!(passed, total, "{} passed out of {}", passed, total);

    // try to let the provider shut down gracefully
    let provider = test_provider().await;
    let _ = provider.shutdown().await;
}

/// test that health check returns healthy
async fn test_health_check(_opt: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;

    // health check
    let hc = prov.health_check().await;
    check!(hc.is_ok())?;
    Ok(())
}

async fn test_single_query(_opt: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;
    let ctx = Context::default();
    let client = SurrealDbSender::via(prov);
    let test_struct = create_test_struct();

    let sql = "CREATE test2 SET field1 = $field1, field2 = $field2, field3 = $field3, field4 = $field4 ".to_string();
    let binding = serde_json::to_string(&test_struct).unwrap();
    let req = QueryRequest {
        bindings: vec![binding],
        queries: vec![sql],
        scope: None,
    };
    let results = client.query(&ctx, &req).await?;

    let result = results.first().unwrap().response.first().unwrap();

    let s = String::from_utf8(result.clone()).unwrap();
    println!("{s}");

    let deser_result: Vec<TestStruct> = serde_json::from_slice(result).unwrap();

    let deser_result = deser_result.first().unwrap();

    assert_eq!(&test_struct, deser_result);

    Ok(())
}

fn create_test_struct() -> TestStruct {
    TestStruct {
        field1: "testtesttest".to_string(),
        field2: 420,
        field3: vec![
            SubStruct {
                subfield1: "subtesttest".to_string(),
                subfield2: true,
                subfield3: vec![233, 120, 42],
            },
            SubStruct {
                subfield1: "subtest2".to_string(),
                subfield2: false,
                subfield3: vec![21, 21, 21],
            },
        ],
        field4: TestEnum::Enumfield3(
            SubStruct {
                subfield1: "enumsubstructtest".to_string(),
                subfield2: false,
                subfield3: vec![90, 31, 53, 78, 150],
            }
        ),
    }
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
struct TestStruct {
    field1: String,
    field2: u16,
    field3: Vec<SubStruct>,
    field4: TestEnum,
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
struct SubStruct {
    subfield1: String,
    subfield2: bool,
    subfield3: Vec<u8>,
}

#[derive(Serialize, Deserialize, Default, PartialEq, Debug)]
enum TestEnum {
    #[default]
    EnumField1,
    EnumField2,
    Enumfield3(SubStruct)
}

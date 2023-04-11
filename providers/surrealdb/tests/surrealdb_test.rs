mod utils;

use serde::{Deserialize, Serialize};
use std::time::Duration;
use tracing::debug;
use tracing::{info, warn};
use utils::*;
use wasmbus_rpc::error::RpcResult;
use wasmbus_rpc::provider::prelude::*;
use wasmcloud_interface_surrealdb::{
    AuthParams, QueryRequest, RequestScope, SurrealDb, SurrealDbSender,
};
use wasmcloud_test_util::{
    check, check_eq, cli::print_test_results, provider_test::test_provider, testing::TestOptions,
};
#[allow(unused_imports)]
use wasmcloud_test_util::{run_selected, run_selected_spawn};

//TODO: docker container with surrealdb instance

#[tokio::test]
async fn run_all() {
    start_logger();
    let _guard = start_docker();
    let opts = TestOptions::default();
    let res = run_selected_spawn!(
        &opts,
        test_health_check,
        test_single_query,
        test_signup,
        test_signin,
        test_auth,
        test_scoped_query,
    );
    print_test_results(&res);

    let passed = res.iter().filter(|tr| tr.passed).count();
    let total = res.len();
    assert_eq!(passed, total, "{} passed out of {}", passed, total);

    // try to let the provider shut down gracefully
    let provider = test_provider().await;
    let _ = provider.shutdown().await;
    tokio::time::sleep(Duration::from_secs(1)).await;
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

    let sql =
        "CREATE test2 SET field1 = $field1, field2 = $field2, field3 = $field3, field4 = $field4 "
            .to_string();
    let binding = serde_json::to_string(&test_struct).unwrap();
    let req = QueryRequest {
        bindings: vec![binding],
        queries: vec![sql],
        scope: None,
    };
    let results = client.query(&ctx, &req).await?;

    let result = results.first().unwrap().response.first().unwrap();

    let s = String::from_utf8(result.clone()).unwrap();
    debug!("{s}");

    let deser_result: Vec<TestStruct> = serde_json::from_slice(result).unwrap();

    let deser_result = deser_result.first().unwrap();

    check_eq!(&test_struct, deser_result)?;

    Ok(())
}

async fn test_signup(_opt: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;
    let ctx = Context::default();
    let client = SurrealDbSender::via(prov);

    let sql = r#"
        REMOVE TABLE user;
        DEFINE SCOPE test_scope
        SESSION 14d
        SIGNUP (
          CREATE type::thing("user", string::lowercase(string::trim($username)))
          SET pass = crypto::argon2::generate($password)
        )
        SIGNIN (
          SELECT * FROM type::thing("user", string::lowercase(string::trim($username)))
          WHERE crypto::argon2::compare(pass, $password)
        )
    "#
    .to_string();

    let res = client
        .query(
            &ctx,
            &QueryRequest {
                bindings: vec!["{}".to_string()],
                queries: vec![sql],
                scope: None,
            },
        )
        .await?;

    for res in res {
        for err in res.errors {
            warn!("{err:#?}");
        }
        for data in res.response {
            let s = String::from_utf8(data).unwrap();
            info!("{s}");
        }
    }

    let scope = RequestScope {
        database: Some("db".to_string()),
        namespace: Some("ns".to_string()),
        scope_name: Some("test_scope".to_string()),
        auth_params: Some(AuthParams {
            username: "test_username".to_string(),
            password: "test_password".to_string(),
        }),
        jwt: None,
    };

    let signup_resp = client.sign_up(&ctx, &scope).await?;

    if signup_resp.error.is_some() {
        warn!("{:?}", signup_resp.error.clone().unwrap());
    }

    check!(signup_resp.success)?;
    check!(signup_resp.error.is_none())?;
    check!(signup_resp.jwt.is_some())?;

    info!("JWT: {}", signup_resp.jwt.unwrap());

    Ok(())
}

async fn test_signin(_opts: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;
    let ctx = Context::default();
    let client = SurrealDbSender::via(prov);

    let sql = r#"
        REMOVE TABLE user_signin;
        DEFINE SCOPE test_signin_scope
        SESSION 14d
        SIGNUP (
          CREATE type::thing("user_signin", string::lowercase(string::trim($username)))
          SET pass = crypto::argon2::generate($password)
        )
        SIGNIN (
          SELECT * FROM type::thing("user_signin", string::lowercase(string::trim($username)))
          WHERE crypto::argon2::compare(pass, $password)
        );
    "#
    .to_string();

    let res = client
        .query(
            &ctx,
            &QueryRequest {
                bindings: vec!["{}".to_string()],
                queries: vec![sql],
                scope: None,
            },
        )
        .await?;

    for res in res {
        for err in res.errors {
            warn!("signin: {err:#?}");
        }
        for data in res.response {
            let s = String::from_utf8(data).unwrap();
            info!("signin: {s}");
        }
    }

    let scope = RequestScope {
        database: Some("db".to_string()),
        namespace: Some("ns".to_string()),
        scope_name: Some("test_signin_scope".to_string()),
        auth_params: Some(AuthParams {
            username: "test_signin_user".to_string(),
            password: "test_signin_pass".to_string(),
        }),
        jwt: None,
    };
    let signup_resp = client.sign_up(&ctx, &scope).await?;
    check!(signup_resp.success)?;

    let signin_resp = client.sign_in(&ctx, &scope).await?;

    if signin_resp.error.is_some() {
        warn!("{:?}", signin_resp.error.clone().unwrap());
    }
    check!(signin_resp.success)?;
    check!(signin_resp.error.is_none())?;
    check!(signin_resp.jwt.is_some())?;

    info!("JWT: {}", signin_resp.jwt.unwrap());

    let invalid_signin = RequestScope {
        database: Some("db".to_string()),
        namespace: Some("ns".to_string()),
        scope_name: Some("test_signin_scope".to_string()),
        auth_params: Some(AuthParams {
            username: "test_signin_user".to_string(),
            password: "invalid_pass".to_string(),
        }),
        jwt: None,
    };

    let signin_resp = client.sign_in(&ctx, &invalid_signin).await?;
    check!(!signin_resp.success)?;

    Ok(())
}

async fn test_auth(_opts: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;
    let ctx = Context::default();
    let client = SurrealDbSender::via(prov);

    let sql = r#"
        REMOVE TABLE user_jwt;
        DEFINE SCOPE test_jwt_scope
        SESSION 14d
        SIGNUP (
          CREATE type::thing("user_jwt", string::lowercase(string::trim($username)))
          SET pass = crypto::argon2::generate($password)
        )
        SIGNIN (
          SELECT * FROM type::thing("user_jwt", string::lowercase(string::trim($username)))
          WHERE crypto::argon2::compare(pass, $password)
        );
    "#
    .to_string();

    let res = client
        .query(
            &ctx,
            &QueryRequest {
                bindings: vec!["{}".to_string()],
                queries: vec![sql],
                scope: None,
            },
        )
        .await?;

    for res in res {
        for err in res.errors {
            warn!("signin: {err:#?}");
        }
        for data in res.response {
            let s = String::from_utf8(data).unwrap();
            info!("signin: {s}");
        }
    }

    let scope = RequestScope {
        database: Some("db".to_string()),
        namespace: Some("ns".to_string()),
        scope_name: Some("test_jwt_scope".to_string()),
        auth_params: Some(AuthParams {
            username: "test_jwt_user".to_string(),
            password: "test_jwt_pass".to_string(),
        }),
        jwt: None,
    };
    let signup_resp = client.sign_up(&ctx, &scope).await?;
    check!(signup_resp.success)?;
    let signin_resp = client.sign_in(&ctx, &scope).await?;
    check!(signin_resp.success)?;

    let valid_jwt = signin_resp.jwt.unwrap();
    let invalid_jwt = "eyJ0eXAiOiJKV1QiLCJhbGciOiJIUzUxMiJ9.eyJpYXQiOjE2ODEyMTEyMzUsIm5iZiI6MTY4MTIxMTIzNSwiZXhwIjoxNjgyNDIwODM1LCJpc3MiOiJTdXJyZWFsREIiLCJOUyI6Im5zIiwiREIiOiJkYiIsIlNDIjoidGVzdF9zaWduaW5fc2NvcGUiLCJJRCI6InVzZXJfc2lnbmluOnRlc3Rfc2lnbmluX3VzZXIifQ.0TwNCLK-IW7NxRkr4ReNhxH3rsBUCK3W0Gdb22AVgkCM5HcDxfZjLQeeMtv5rkHZZk1nkm0Ew2A3chxZ3fcL-f".to_string();

    let invalid_auth = client.authenticate(&ctx, &invalid_jwt).await?;
    let valid_auth = client.authenticate(&ctx, &valid_jwt).await?;

    if valid_auth.error.is_some() {
        warn!("jwt-test: {:#?}", valid_auth.error.unwrap());
    }

    check!(valid_auth.success)?;
    check!(!invalid_auth.success)?;

    Ok(())
}

async fn test_scoped_query(_opts: &TestOptions) -> RpcResult<()> {
    let prov = test_provider().await;
    let ctx = Context::default();
    let client = SurrealDbSender::via(prov);

    let sql = r#"
        REMOVE TABLE user_query;
        REMOVE TABLE user_wrong;
        DEFINE SCOPE test_query_scope
        SESSION 14d
        SIGNUP (
          CREATE type::thing("user_query", string::lowercase(string::trim($username)))
          SET pass = crypto::argon2::generate($password)
        )
        SIGNIN (
          SELECT * FROM type::thing("user_query", string::lowercase(string::trim($username)))
          WHERE crypto::argon2::compare(pass, $password)
        );

        DEFINE TABLE scoped_table SCHEMALESS
        PERMISSIONS
            FOR select, update, delete WHERE user = $token.ID AND $scope = "test_query_scope"
            FOR create WHERE $scope = "test_query_scope";

        DEFINE SCOPE wrong_scope
        SESSION 14d
        SIGNUP (
          CREATE type::thing("user_wrong", string::lowercase(string::trim($username)))
          SET pass = crypto::argon2::generate($password)
        )
        SIGNIN (
          SELECT * FROM type::thing("user_wrong", string::lowercase(string::trim($username)))
          WHERE crypto::argon2::compare(pass, $password)
        );
    "#
        .to_string();

    let res = client
        .query(
            &ctx,
            &QueryRequest {
                bindings: vec!["{}".to_string()],
                queries: vec![sql],
                scope: None,
            },
        )
        .await?;

    for res in res {
        for err in res.errors {
            warn!("scoped_query: {err:#?}");
        }
        for data in res.response {
            let s = String::from_utf8(data).unwrap();
            info!("scoped_query: {s}");
        }
    }

    let scope = RequestScope {
        database: Some("db".to_string()),
        namespace: Some("ns".to_string()),
        scope_name: Some("test_query_scope".to_string()),
        auth_params: Some(AuthParams {
            username: "test_query_user".to_string(),
            password: "test_query_pass".to_string(),
        }),
        jwt: None,
    };
    let signup_resp = client.sign_up(&ctx, &scope).await?;
    check!(signup_resp.success)?;

    let jwt = signup_resp.jwt.unwrap();

    let sql =
        r#"CREATE scoped_table SET user = $token.ID, field1 = "poop" "#
            .to_string();

    // Wrong scope, should fail
    let wrong_scope = RequestScope {
        database: Some("db".to_string()),
        namespace: Some("ns".to_string()),
        scope_name: Some("wrong_scope".to_string()),
        auth_params: Some(AuthParams {
            username: "test_query_user".to_string(),
            password: "test_query_pass".to_string(),
        }),
        jwt: None,
    };
    let req = QueryRequest {
        bindings: vec!["{}".to_string()],
        queries: vec![sql.clone()],
        scope: Some(wrong_scope.clone()),
    };

    // Not signed up, should return error
    let result = client.query(&ctx, &req).await;
    check!(result.is_err())?;

    // Sign user up to wrong scope
    let sign_up_wrong_scope = client.sign_up(&ctx, &wrong_scope).await?;
    let jwt_wrong_scope = sign_up_wrong_scope.jwt.unwrap();

    let req = QueryRequest {
        bindings: vec!["{}".to_string()],
        queries: vec![sql.clone()],
        scope: Some(RequestScope { jwt: Some(jwt_wrong_scope), ..Default::default()}),
    };

    let results = client.query(&ctx, &req).await?;

    for result in results {
        info!("scope-query: {:#?}", result.errors);
        for response in result.response {
            if !response.is_empty() {
                check_eq!("[]".to_string(), String::from_utf8(response.clone()).unwrap())?;
            }
        }
        // check!(!result.errors.is_empty())?;
    }

    // Correct scope (using jwt), should succeed
    let req = QueryRequest {
        bindings: vec!["{}".to_string()],
        queries: vec![sql],
        scope: Some(RequestScope {
            jwt: Some(jwt.clone()),
            ..Default::default()
        }),
    };
    let results = client.query(&ctx, &req).await?;

    for result in results {
        info!("scope-query: {:#?}", result.errors);
        check!(result.errors.is_empty())?;

        for response in result.response {
            let s = String::from_utf8(response).unwrap();
        }
    }



    // Sign up new user in same scope to ensure they can't access
    let scope = RequestScope {
        database: Some("db".to_string()),
        namespace: Some("ns".to_string()),
        scope_name: Some("test_query_scope".to_string()),
        auth_params: Some(AuthParams {
            username: "new_user".to_string(),
            password: "password123".to_string(),
        }),
        jwt: None,
    };
    let signup_resp = client.sign_up(&ctx, &scope).await?;
    check!(signup_resp.success)?;
    let other_jwt = signup_resp.jwt.unwrap();

    let sql = r#"SELECT * FROM scoped_table WHERE field1 = "poop""#.to_string();
    let req = QueryRequest {
        bindings: vec!["{}".to_string()],
        queries: vec![sql.clone()],
        scope: Some(RequestScope{
            jwt: Some(other_jwt),
            ..Default::default()
        }),
    };
    let results = client.query(&ctx, &req).await?;

    for result in results {
        info!("scope-query: {:#?}", result.errors);
        for response in result.response {
            if !response.is_empty() {
                let s = String::from_utf8(response.clone()).unwrap();
                check_eq!("[]".to_string(), s)?;
            }
        }
    }

    // Correct scope (using jwt), should succeed
    let req = QueryRequest {
        bindings: vec!["{}".to_string()],
        queries: vec![sql],
        scope: Some(RequestScope {
            jwt: Some(jwt),
            ..Default::default()
        }),
    };
    let results = client.query(&ctx, &req).await?;

    for result in results {
        info!("scope-query: {:#?}", result.errors);
        check!(result.errors.is_empty())?;

        for response in result.response {
            let s = String::from_utf8(response).unwrap();
            check!(!(s == "[]".to_string()))?;
            warn!("scope-query: {s}");
        }
    }

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
        field4: TestEnum::Enumfield3(SubStruct {
            subfield1: "enumsubstructtest".to_string(),
            subfield2: false,
            subfield3: vec![90, 31, 53, 78, 150],
        }),
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
    Enumfield3(SubStruct),
}

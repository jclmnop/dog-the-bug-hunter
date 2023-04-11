use crate::config::LinkConfig;
use crate::SurrealClient;
use anyhow::{anyhow, Result};
use serde::Serialize;
use surrealdb::opt::auth::*;
use wasmcloud_interface_surrealdb::{AuthParams, RequestScope};

//TODO: tidy this up it's disgusting
pub async fn sign_in(
    request_scope: &Option<RequestScope>,
    conf: &LinkConfig,
    client: &mut SurrealClient,
) -> Result<()> {
    if let Some(request_scope) = request_scope {
        match request_scope {
            RequestScope {
                auth_params: Some(params),
                database: Some(database),
                namespace: Some(namespace),
                scope_name: Some(scope),
            } => {
                let credentials = Scope {
                    namespace,
                    database,
                    scope,
                    params,
                };
                // client.use_ns(namespace).use_db(database).await?;
                client.signin(credentials).await?;
            }
            RequestScope {
                auth_params: Some(AuthParams { username, password }),
                database: Some(database),
                namespace: Some(namespace),
                ..
            } => {
                let credentials = Database {
                    namespace,
                    database,
                    username,
                    password,
                };
                // client.use_ns(namespace).use_db(database).await?;
                client.signin(credentials).await?;
            }
            RequestScope {
                auth_params: Some(AuthParams { username, password }),
                namespace: Some(namespace),
                ..
            } => {
                let credentials = Namespace {
                    namespace,
                    username,
                    password,
                };
                // client.use_ns(namespace).use_db(database).await?;
                client.signin(credentials).await?;
            }
            RequestScope {
                auth_params: Some(AuthParams { username, password }),
                ..
            } => {
                let credentials = Root { username, password };
                // client.use_ns(namespace).use_db(database).await?;
                client.signin(credentials).await?;
            }
            RequestScope {
                auth_params: None,
                namespace: Some(namespace),
                database,
                ..
            } => {
                client.signin(root_credentials(conf)).await?;
                if let Some(database) = database {
                    client.use_ns(namespace).use_db(database).await?;
                } else {
                    return Err(anyhow!("Invalid combination of scope params"));
                }
            }
            _ => return Err(anyhow!("Invalid combination of scope params")),
        }
    } else {
        let root = root_credentials(conf);
        client.signin(root).await?;
        let ns = &conf.default_namespace;
        let db = &conf.default_database;
        client.use_ns(ns).use_db(db).await?;
    }
    Ok(())
}

pub fn to_scope(req_scope: &RequestScope) -> Result<Scope<AuthParams>> {
    match req_scope {
        RequestScope {
            auth_params: Some(params),
            database: Some(database),
            namespace: Some(namespace),
            scope_name: Some(scope),
        } => Ok(Scope {
            namespace,
            database,
            scope,
            params: params.to_owned(),
        }),
        _ => Err(anyhow!("Invalid user scope.")),
    }
}

fn root_credentials(conf: &LinkConfig) -> Root {
    let root_user = &conf.user;
    let root_password = &conf.pass;
    Root {
        username: root_user,
        password: root_password,
    }
}

// #[derive(Serialize)]
// struct UserPass<'a> {
//     #[serde(rename = "user")]
//     username: &'a String,
//     #[serde(rename = "pass")]
//     password: &'a String,
// }

use base64::engine::general_purpose::GeneralPurpose;
use base64::Engine;
use serde::{Deserialize, Serialize};
use wasmbus_rpc::core::LinkDefinition;
use wasmbus_rpc::error::{RpcError, RpcResult};

// TODO: "global" config?

/// Per-actor config for each link definition
#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct LinkConfig {
    //TODO: connection/client type (ws, wss, http, embedded/in-memory, etc)
    /// Host address for the SurrealDB instance. Defaults to `localhost`
    #[serde(default = "default_host")]
    pub host: String,
    /// Port for the SurrealDB instance. Defaults to `8000`
    #[serde(default = "default_port")]
    pub port: u16,
    /// Username for Root scope. Defaults to `root`
    #[serde(default = "default_user")]
    pub user: String,
    /// Password for Root scope. Defaults to `root`
    #[serde(default = "default_pass")]
    pub pass: String,
    /// Concurrency limit for built-in connection pool. `0` is unbounded. Defaults to `100_000`
    #[serde(default = "default_concurrency")]
    pub concurrency: usize,
    /// Default namespace to be used when one isn't specified. Defaults to `ns`
    #[serde(default = "default_default_namespace")]
    pub default_namespace: String,
    /// Default database to be used when one isn't specified. Defaults to `db`
    #[serde(default = "default_default_database")]
    pub default_database: String,
}

//TODO: connect()
impl LinkConfig {
    pub fn get_url(&self) -> String {
        format!("{}:{}", self.host, self.port)
    }

    // Mostly stolen from https://github.com/wasmCloud/capability-providers/blob/main/sqldb-postgres/src/config.rs
    /// Load configuration from 'values' field of LinkDefinition.
    /// Support a variety of configuration possibilities:
    ///  'config_json' - json string
    ///  'config_b64' - base64-encoded json
    pub fn load_config(ld: &LinkDefinition) -> RpcResult<Self> {
        let b64_engine = GeneralPurpose::new(
            &base64::alphabet::STANDARD,
            base64::engine::GeneralPurposeConfig::default(),
        );
        if let Some(cj) = ld.values.get("config_b64") {
            serde_json::from_slice(
                &b64_engine
                    .decode(cj)
                    .map_err(|_| RpcError::ProviderInit("invalid config_base64 encoding".into()))?,
            )
                .map_err(|e| RpcError::ProviderInit(format!("invalid json config: {e}")))
        } else if let Some(cj) = ld.values.get("config_json") {
            serde_json::from_str(cj.as_str())
                .map_err(|e| RpcError::ProviderInit(format!("invalid json config: {e}")))
        } else {
            serde_json::from_str("{}") // Should just load all the defaults
                .map_err(|e| RpcError::ProviderInit(format!("can't deserialise empty config: {e}")))
        }
    }
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub struct ProviderConfig {
    #[serde(default)]
    pub provider_type: ProviderType,
}

#[derive(Debug, Default, Deserialize, Serialize, PartialEq)]
pub enum ProviderType {
    #[serde(rename = "static")]
    Static(LinkConfig),
    #[default]
    Dynamic
}

#[derive(Debug, Default, Deserialize, Serialize)]
pub enum ClientType {
    #[default]
    #[serde(rename = "ws")]
    Ws,
    #[serde(rename = "wss")]
    Wss,
    #[serde(rename = "http")]
    Http,
    #[serde(rename = "embedded")]
    Embedded,
}

fn default_host() -> String {
    "localhost".into()
}

fn default_port() -> u16 {
    8000
}

fn default_user() -> String {
    "root".into()
}

fn default_pass() -> String {
    "root".into()
}

fn default_concurrency() -> usize {
    //TODO: figure out a sensible value
    100_000
}

fn default_default_namespace() -> String {
    "ns".into()
}

fn default_default_database() -> String {
    "db".into()
}

//TODO: basic unit tests
#[cfg(test)]
mod tests {
    use super::{ProviderConfig, ProviderType, ClientType, LinkConfig, Deserialize};

    #[test]
    fn test_defaults() {
        let default_str = "{}";
        let link_conf: LinkConfig = serde_json::from_str(default_str).expect("Failed to deser default link config");
        let provider_conf: ProviderConfig = serde_json::from_str(default_str).expect("Failed to deser default link config");

        println!("{}", serde_json::to_string_pretty(&link_conf).unwrap());
        println!("{}", serde_json::to_string_pretty(&provider_conf).unwrap());

        assert_eq!(link_conf,
            LinkConfig {
                host: "localhost".into(),
                port: 8000,
                user: "root".into(),
                pass: "root".into(),
                concurrency: 100000,
                default_namespace: "ns".into(),
                default_database: "db".into(),
            }
        );
        assert_eq!(provider_conf, ProviderConfig{ provider_type: ProviderType::Dynamic });
    }
}

/*
{
  "host": "localhost",
  "port": 8000,
  "user": "root",
  "pass": "root",
  "concurrency": 100000,
  "default_namespace": "ns",
  "default_database": "db"
}
{
  "provider_type": "Dynamic"
}

 */
use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Config {
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

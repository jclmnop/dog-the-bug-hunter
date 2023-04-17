use anyhow::anyhow;
use chrono::prelude::*;
use clap::{Args, Parser, Subcommand};
use colored_json::prelude::*;
use serde_json::json;
use std::env;
use std::path::PathBuf;

const APP_NAME: &str = "Dog, The Bug Hunter";
const URL: &str = "https://divine-ocean-2029.cosmonic.app";
const DB_WS: &str = "ws://dtbh-surrealdb.fly.dev";
const REPORTS: &str = "reports";
const SCAN: &str = "scan";
const SIGN_UP: &str = "sign_up";
const SIGN_IN: &str = "sign_in";
const ENV_JWT: &str = "DTBH_JWT";
const JWT_DIR: &str = ".dtbh";

type CliResult<T> = Result<T, anyhow::Error>;

fn main() {
    let cli = Cli::parse();

    match cli.process_command() {
        Ok(_) => (),
        Err(e) => {
            eprintln!("{e}");
            std::process::exit(1);
        }
    }
}

/// CLI for `Dog, The Bug Hunter`, a web application for vulnerability scanning
/// built on Cosmonic
#[derive(Parser, Debug)]
#[clap(name = APP_NAME, version = "0.1.0", author = "jclmnop")]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Optional JWT override for authentication, by default it will be read from
    /// the `$DTBH_ENV` environment variable, or the `jwt_path` file if that hasn't
    /// been set
    #[arg(long)]
    jwt: Option<String>,
    /// Path to file containing JWT for authentication, or for the JWT to be written to when signing up/in.
    /// Default: `~/.dtbh/jwt`
    #[arg(long)]
    jwt_path: Option<String>,
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand, Debug)]
enum Commands {
    /// Begin scanning target URL(s) for potential vulnerabilities
    Scan {
        /// Target URL to scan, can be passed more than once
        #[arg(short, long)]
        target: Vec<String>,
    },
    /// Sign up with a new username and password to create a new account
    SignUp(AuthArgs),
    /// Sign in to an existing account with a username and password
    SignIn(AuthArgs),
    /// View reports for the current user. If no arguments are passed, then all
    /// reports for the current user will be returned.
    ///
    /// Timestamps can be passed in either ISO 8601 format or as seconds since
    /// epoch. If both types are passed for one of the timestamps, then the
    /// seconds since epoch will be used.
    Reports {
        /// Target to fetch report for. Can be passed more than once. If not passed,
        /// then reports for all targets will be returned
        #[arg(short, long)]
        target: Vec<String>,
        /// Start time for the report, in ISO 8601 format (e.g. 2021-01-01 00:00:00)
        #[arg(short, long)]
        start_time: Option<String>,
        /// End time for the report, in ISO 8601 format (e.g. 2021-01-01 00:00:00)
        #[arg(short, long)]
        end_time: Option<String>,
        /// Start timestamp for the report, in seconds since epoch
        #[arg(long)]
        start_timestamp: Option<i64>,
        /// End timestamp for the report, in seconds since epoch
        #[arg(long)]
        end_timestamp: Option<i64>,
    },
}

#[derive(Args, Debug)]
struct AuthArgs {
    /// Username for the account
    #[arg(short, long)]
    username: String,
    /// Password for the account
    #[arg(short, long)]
    password: String,
    /// Print returned JWT to stdout instead of writing it to a file
    #[arg(long)]
    print_jwt: bool,
}

impl Cli {
    fn get_jwt(&self) -> Option<String> {
        if self.jwt.is_some() {
            self.jwt.clone()
        } else if let Ok(jwt) = env::var(ENV_JWT) {
            if jwt.is_empty() {
                None
            } else {
                Some(jwt)
            }
        } else {
            let jwt_path = self.get_jwt_path();
            let jwt = std::fs::read_to_string(jwt_path).ok();
            jwt
        }
    }

    fn write_jwt(&self, jwt: Option<String>) -> CliResult<()> {
        if let Some(jwt) = jwt {
            let jwt_path = self.get_jwt_path();
            let parent_dir_exists = if let Some(parent_dir) = jwt_path.parent() {
                parent_dir.exists()
            } else {
                false
            };
            if !parent_dir_exists {
                std::fs::create_dir_all(jwt_path.parent().ok_or(anyhow!("Invalid JWT path"))?)?;
            }
            std::fs::write(&jwt_path, jwt)?;
            println!("JWT written to {}", jwt_path.display());
            Ok(())
        } else {
            Ok(())
        }
    }

    fn get_jwt_path(&self) -> PathBuf {
        if let Some(jwt_path) = &self.jwt_path {
            PathBuf::from(jwt_path)
        } else {
            let home_dir = dirs::home_dir().unwrap_or(PathBuf::from("/"));
            home_dir.join(JWT_DIR).join("jwt")
        }
    }

    fn process_command(&self) -> CliResult<()> {
        let jwt = self.get_jwt();
        match &self.command {
            Commands::Scan { target } => scan(jwt, target)?,
            Commands::SignUp(AuthArgs {
                username,
                password,
                print_jwt: output_jwt,
            }) => self.write_jwt(sign_up(username, password, *output_jwt)?)?,
            Commands::SignIn(AuthArgs {
                username,
                password,
                print_jwt: output_jwt,
            }) => self.write_jwt(sign_in(username, password, *output_jwt)?)?,
            Commands::Reports {
                target,
                start_time,
                end_time,
                start_timestamp,
                end_timestamp,
            } => reports(
                jwt,
                target,
                start_time.clone(),
                end_time.clone(),
                start_timestamp.clone(),
                end_timestamp.clone(),
            )?,
        }

        Ok(())
    }
}

fn scan(jwt: Option<String>, targets: &Vec<String>) -> CliResult<()> {
    if jwt.is_none() {
        anyhow::bail!("No JWT provided, please sign in or sign up");
    }
    let jwt = jwt.unwrap();
    if targets.is_empty() {
        anyhow::bail!("No targets provided, please provide at least one target")
    }
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(format!("{}/{}", URL, SCAN))
        .json(&json!({
            "targets": targets,
        }))
        .bearer_auth(jwt)
        .send()?;

    if res.status().is_success() {
        println!("Scan started successfully");
    } else {
        anyhow::bail!("Scan failed to start: {}", res.status());
    }

    // TODO: option to subscribe to websocket for updates?

    Ok(())
}

fn sign_up(username: &String, password: &String, output_jwt: bool) -> CliResult<Option<String>> {
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(format!("{}/{}", URL, SIGN_UP))
        .json(&json!({
            "username": username,
            "password": password,
        }))
        .send()?;

    handle_auth_response(res, output_jwt)
}

fn sign_in(username: &String, password: &String, output_jwt: bool) -> CliResult<Option<String>> {
    let client = reqwest::blocking::Client::new();
    let res = client
        .post(format!("{}/{}", URL, SIGN_IN))
        .json(&json!({
            "username": username,
            "password": password,
        }))
        .send()?;

    handle_auth_response(res, output_jwt)
}

fn reports(
    jwt: Option<String>,
    targets: &Vec<String>,
    start_time: Option<String>,
    end_time: Option<String>,
    start_timestamp: Option<i64>,
    end_timestamp: Option<i64>,
) -> CliResult<()> {
    if jwt.is_none() {
        anyhow::bail!("No JWT provided, please sign in or sign up");
    }
    let jwt = jwt.unwrap();

    let start = if let Some(start_time) = start_timestamp {
        wasmbus_rpc::Timestamp::new(start_time, 0)
            .map_err(|e| anyhow!("Error parsing start timestamp: {e}"))?
    } else if let Some(start_time) = start_time {
        let start_time = Local.datetime_from_str(&start_time, "%Y-%m-%d %H:%M:%S")?;
        wasmbus_rpc::Timestamp::new(start_time.timestamp(), start_time.timestamp_subsec_nanos())
            .map_err(|e| anyhow!("Error parsing start time: {e}"))?
    } else {
        wasmbus_rpc::Timestamp::new(1681490325, 0).map_err(|e| anyhow!("It's all my fault"))?
    };

    let end = if let Some(end_time) = end_timestamp {
        wasmbus_rpc::Timestamp::new(end_time, 0)
            .map_err(|e| anyhow!("Error parsing end timestamp: {e}"))?
    } else if let Some(end_time) = end_time {
        let end_time = Local.datetime_from_str(&end_time, "%Y-%m-%d %H:%M:%S")?;
        wasmbus_rpc::Timestamp::new(end_time.timestamp(), end_time.timestamp_subsec_nanos())
            .map_err(|e| anyhow!("Error parsing end time: {e}"))?
    } else {
        wasmbus_rpc::Timestamp::now()
    };
    let client = reqwest::blocking::Client::new();
    let res = client
        .get(format!("{}/{}", URL, REPORTS))
        .bearer_auth(jwt)
        .json(&json!({
            "target": targets,
            "startTimestamp": start,
            "endTimestamp": end,
        }))
        .send()?;

    if !res.status().is_success() {
        anyhow::bail!("Failed to get reports: {}", res.status());
    } else {
        let body: Vec<u8> = res.json()?;
        let s = std::str::from_utf8(&body)?;
        let reports: serde_json::Value = serde_json::from_str(s)?;
        println!(
            "{}",
            serde_json::to_string_pretty(&reports)?.to_colored_json_auto()?
        );
    }

    // TODO: option to write to file

    Ok(())
}

fn handle_auth_response(
    res: reqwest::blocking::Response,
    output_jwt: bool,
) -> CliResult<Option<String>> {
    if !res.status().is_success() {
        anyhow::bail!("Authentication failed: {}", res.text()?);
    }

    let jwt = res
        .cookies()
        .find(|c| c.name() == "jwt")
        .ok_or(anyhow!("No JWT found in response"))?
        .value()
        .to_string();

    if output_jwt {
        println!("JWT:\n{}", jwt);
        Ok(None)
    } else {
        Ok(Some(jwt))
    }
}

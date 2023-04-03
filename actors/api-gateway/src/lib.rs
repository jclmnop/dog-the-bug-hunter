use anyhow::Result;
use wasmbus_rpc::actor::prelude::*;
use wasmcloud_interface_httpserver::{
    HttpRequest, HttpResponse, HttpServer, Method,
};
use wasmcloud_interface_logging::{error, info};

const CALL_ALIAS: &str = "dtb/scanner/api-gateway";

#[derive(Debug, Default, Actor, HealthResponder)]
#[services(Actor, HttpServer)]
struct TemplateActor {}

use indexmap::IndexMap;
use surrealdb::sql::Value;
use surrealdb::Response as SurrealResponse;
use surrealdb::Result as SurrealResult;
use wasmcloud_interface_surrealdb::{QueryResponse, SurrealDbError};

pub struct Response(pub IndexMap<usize, SurrealResult<Vec<Value>>>);

//TODO: get rid of this unsafe block if they ever make it easier to serialise surrealdb::Response
impl From<SurrealResponse> for Response {
    fn from(value: SurrealResponse) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

//TODO: get rid of this unsafe block if they ever make it easier to deserialise surrealdb::Response
impl From<Response> for SurrealResponse {
    fn from(value: Response) -> Self {
        unsafe { std::mem::transmute(value) }
    }
}

impl From<Response> for QueryResponse {
    fn from(response: Response) -> Self {
        let mut query_response = QueryResponse::default();

        for (_k, v) in response.0 {
            if let Ok(values) = v {
                match serde_json::to_vec(&values) {
                    Ok(bytes) => query_response.response.push(bytes),
                    Err(e) => {
                        query_response.response.push(vec![]);
                        query_response.errors.push(SurrealDbError {
                            message: e.to_string(),
                            name: "serde_json_error".to_string(),
                        });
                    }
                }
            } else if let Err(e) = v {
                query_response.response.push(vec![]);
                query_response.errors.push(SurrealDbError {
                    message: e.to_string(),
                    name: "query_error".to_string(),
                })
            }
        }

        query_response
    }
}

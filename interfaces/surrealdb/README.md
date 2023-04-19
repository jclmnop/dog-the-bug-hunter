# wasmcloud-interface-surrealdb

[![crates.io](https://img.shields.io/crates/v/wasmcloud-interface-surrealdb.svg)](https://crates.io/crates/wasmcloud-interface-timing)
[![Documentation](https://docs.rs/wasmcloud-interface-surrealdb/badge.svg)](https://docs.rs/wasmcloud-interface-timing)

Interface definition for `wasmcloud:surrealdb` capability provider. This is a 
_very early_ version of the interface, and there will be breaking changes in the 
near future.

Currently only provides four methods: `query`, `sign_up`, `sign_in`, and `authenticate`.

## Query
The `QueryRequest` struct has currently 3 fields, but will be revised in the next version:
- `queries`: A vector of Strings, each of which is a query to be executed.
- `bindings`: A vector of serialised binding variables. Each query needs to have
   a corresponding string in this vector, if there are no bindings for a query,
   then `"{}"` should be used. 
- `scope`: An optional `Scope` struct (slightly different to the one from the surrealdb library),
   if not provided then the root login will be used. Before a query is executed, the 
   connection will be signed in to the scope provided (or as root). 

Each query in the `queries` vector will be executed in order, and the results will be returned 
as a vector of `QueryResponse` structs. Because each query can actually be a collection of 
multiple queries, and each query can return multiple objects, the `response` field will 
deserialise into a vector of vectors. I plan to make this more ergonomic in `v0.2.0`.

The easiest way to serialise the bindings for a query if you don't want to define a new struct, 
or if you just want more fine grained control, is to use the `serde_json::json!` macro:
```rust
let surrealdb = SurrealDbSender::new();

// We have a `Person` struct which corresponds to our person table in the DB, 
// but we only want to update a few fields. 
let dave = Person {
    name: "Dave".to_string(),
    age: 32,
    hobbies: vec!["Programming".to_string(), "Cooking".to_string()],
    occupation: Occupation {
        title: "Software Engineer".to_string(),
        company: "ACME".to_string(),
        salary: 100_000,
    },
};

let query_dave = r#"
    UPDATE person:dave MERGE {
        hobbies: $hobbies,
        occupation.salary: $salary,
    };
"#.to_string();

let bindings_dave = json!({
    "hobbies": dave.hobbies,
    "salary": dave.occupation.salary,
}).to_string();

let response = surrealdb.query(
    ctx,
    &QueryRequest {
        queries: vec![query_dave],
        bindings: vec![bindings_dave],
        scope: None,
    },
).await?;
```
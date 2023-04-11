# TODO
## Essential
- [ ] unit tests for serde etc
- [ ] integration tests
  - [x] single query schemaless
  - [x] signin
  - [x] signup
  - [x] jwt auth
  - [x] scoped query
  - [ ] transaction
  - [x] docker
- [x] basic interface
- [x] figure out a way of se/deserialising queries, bindings and responses
    - the way i'm currently handling serde of responses isn't ideal but i don't 
      have time to implement something perfect right now
        - at the moment, for responses, i'm just (unsafely) transmuting the surrealdb `Response`
          struct into a struct with a public field so the inner indexmap can be accessed.
          before serialising i have to convert all results to their Ok values (and 
          put any Errors in a separate field) because surrealdb errors don't impl 
          serialise. a more ideal solution would be to open a PR for surrealdb, and 
          impl serde for their errors + serde for the entire `Response` struct, then 
          we can just return a serialised version of the actual `Response` struct to 
          the actor and it can be deserialised while retaining all the original surrealdb 
          types etc
        - cbor enc/decoding would be even better (or maybe bung?)
- [x] basic per-actor config
- [x] basic operations
  - [x] query
  - [x] signup
  - [x] signin/auth?

## Nice to have
- [ ] basic global config
    - [ ] configure whether to use separate DBs/configs for each actor, or whether 
      to use one database connection with a static client
    - [ ] connect to remote DB or run an in-memory DB (good for dev + cache)
- [ ] extra operations (basically wrap *all* the surrealdb client methods)
  - [ ] select
  - [ ] update
  - [ ] etc
- [ ] [bung](https://docs.rs/bung/latest/bung/index.html) encdec
- [ ] TLS

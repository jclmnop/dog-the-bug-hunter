# Sleepy Capability Provider

This third-party capability provider implements the third-party 
["jclmnop:endpoint_enumerator"]() capability contract.

Build with 'make'. Test with 'make test'.

# TODO
- [x] Implement a queue for incoming requests   
  - [x] `tokio::sync::Semaphore` with 1 permit that must be acquired to run a job (or a mutex?) 
  - [ ] ~~add job to queue if job is currently running~~
  - [ ] ~~`Arc<Mutex<VecDeque<EnumeratorRequest>>>`~~
- [ ] Figure out optimal concurrency values
- [ ] Trim down the list of ports, or add a config
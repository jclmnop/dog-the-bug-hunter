![img_1.png](img_1.png)
# Dog The Bug Hunter
# Overview
> Note: This is a work in progress for the Cosmonic Hackathon. It is not yet ready for production use, and probably never will be.
> 
> I'm not a security expert, this tool will most likely yield a lot of false positives/false negatives.

A distributed, modular, cloud-native automated vulnerability scanner for bug bounty hunting. The user pushes a queue of
target domains and their endpoints (subdomains + open ports) are enumerated sequentially (or in parallel if this ever 
becomes a paid service), which are then passed to vulnerability scanners to be scanned for common types of vulnerabilities. 

New types of vulnerability scanners can be added without any configuration, thanks to the actor model and 
NATS pub/sub messaging. All types of vulnerability scans run in parallel, and the results are aggregated and written to a
report which is only accessible to the user who submitted the request to scan the target.

Although it's intended to be hosted on a [Cosmonic](https://cosmonic.com/) managed wasmCloud host, 
using the [super-constellation](https://cosmonic.com/docs/user_guide/superconstellations) functionality for
extra scalability, the same can be achieved with self-managed wasmCloud hosts 
[bridged via NGS](https://wasmcloud.com/docs/reference/lattice/ngs/). 

## Architecture
![img.png](img.png)

Capability providers are shown in blue, and actors in purple. 

Once a request is received by the API Gateway it's passed to the orchestrator actor, which then calls the endpoint 
enumerator. To get around the 2 second timeout on RPC calls the endpoint enumerator spawns a task in the background and 
instantly returns a response to the orchestrator. Once the endpoint enumerator has finished enumerating the endpoints, it
will callback to the orchestrator with the results. 

The orchestrator then publishes a message using NATS to a channel which all vulnerability scanners are subscribed to
(again, to get around the 2 second timeout on RPC calls). Once a vulnerability scanner has finished scanning the
endpoints, it will publish the results to a NATS channel which the report-writer actor is subscribed to. 

The report writer writes these results into the report for the specified job (composed of the UserID, Target and 
RequestTimestamp) using the KV storage provider.

The system can be easily scaled by adding more vulnerability scanner actors and more endpoint enumerator + Http-Client 
providers to the super-constellation, on any cloud provider.  

### NATS Topics
- `dtbh.tasks`
    - `pub`: `orchestrator`
    - `sub`: `scanner-modules`
- `dtbh.reports.in`
    - `pub`: `scanner-modules`
    - `sub`: `report-writer`
- `dtbh.reports.out`
    - `pub`: `report-writer`
    - `sub`: external

# TODO
## Necessary for Hackathon PoC
### General
- [x] Decide between KV/SQL/surrealDB for storing reports (leaning towards surrealDB)
  - [x] build a surrealDB provider
- [ ] Test on local
- [ ] Test on cosmonic
- [ ] Test scaling on Railway/Digital Ocean
- [ ] Refactor interfaces
  - [ ] remove some "services"
  - [x] merge endpoint-enumerator interface with dtbh interface
- [ ] CLI
  - [ ] wrappers for all api requests
    - [ ] get_reports
    - [ ] sign_up
    - [ ] sign_in
    - [ ] scan
  - [ ] connect to surrealDB websocket for new events

### Actors
#### api-gateway
- [x] handle POST request to /scan
- [x] handle POST request to /reports (POST bc it will require auth)
- [x] very basic auth for testing (surrealDB)
  - [x] sign up
  - [x] sign in
  - [x] JWT auth token
- [ ] improve error handling
#### report-writer
- [x] Implement message subscription for vulnerability scanner results 
- [x] write results to storage
- [x] Implement get_reports() rpc method
#### orchestrator
- [x] implement handling from API Gateway -> endpoint enumerator
- [x] implement handling from endpoint enumerator callback -> publish to vulnerability scanner NATS channel
#### vulnerability-scanners
- [x] message handling (sub + pub)
- [ ] Implement at least 4 different vulnerability scanners, can be fairly simple
  - [ ] elasticsearch unauthenticated access 
  - [x] dotenv disclosure
  - [x] basic SQLi?
  - [ ] a recent CVE
#### ui-actor
- [ ] only if everything else is finished first
- [ ] single page leptos app embedded in actor binary
- [ ] login/auth -> (view_reports_form | scan_form)

### Providers
#### Endpoint Enumerator
- [x] Implement task queue so multiple targets can be submitted at once, and the provider can work through them with a 
      configurable concurrency value in the link definition
#### SurrealDB
- [x] simple functions
  - [x] query
  - [x] signup
  - [x] signin
#### "wasmcloud:timing" interface + provider
- [x] publish interface to crate
- [x] publish provider to OCI
- [ ] add to wasmcloud examples repo

## Nice to haves
- [ ] Open telemetry
- [ ] proper session tokens rather than storing JWT in headers/cookies
- [ ] Front end for login + submitting targets
- [ ] Front end for viewing reports 
- [ ] notifications/emails when reports are ready for a specific vulnerability scanner
- [ ] deploy script
- [ ] Figure out optimal concurrency values for endpoint enumerator provider
- [ ] Trim down the list of ports, or add a config for endpoint enumerator provider
- [ ] Test scalability with a separate containerised deployment on Railway and Digital Ocean 
      (extra providers on Digital Ocean, extra actors on Railway?)
- [ ] Encrypt report data + store user private keys in vault
- [ ] customisable wordlists stored in user-specific KV storage
- [ ] blind TRUE/FALSE SQLi
- [ ] blind sleep/time based SQLi

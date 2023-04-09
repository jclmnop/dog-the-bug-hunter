# sleepy Makefile

CAPABILITY_ID = "wasmcloud:surrealdb"
NAME = "surrealdb"
VENDOR = "jclmnop"
PROJECT = wasmcloud-provider-surrealdb
VERSION = 0.1.0
REVISION = 0

include ./provider.mk

test::
	cargo clippy --all-targets --all-features
	cargo test -- --nocapture


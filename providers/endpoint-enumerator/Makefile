# sleepy Makefile

CAPABILITY_ID = "jclmnop:endpoint_enumerator"
NAME = "endpoint-enumerator"
VENDOR = "jclmnop"
PROJECT = wasmcloud-provider-endpoint-enumerator
VERSION = 0.1.0
REVISION = 0

include ./provider.mk

test::
	cargo clippy --all-targets --all-features
	cargo test -- --nocapture


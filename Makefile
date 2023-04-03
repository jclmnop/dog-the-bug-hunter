# sleepy Makefile

CAPABILITY_ID = "jclmnop:sleepy"
NAME = "sleepy"
VENDOR = "jclmnop"
PROJECT = wasmcloud-provider-sleepy
VERSION = 0.1.0
REVISION = 0

include ./provider.mk

test::
	cargo clippy --all-targets --all-features
	cargo test -- --nocapture


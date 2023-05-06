ERBOSE := $(if ${CI},--verbose,)

COMMIT := $(shell git rev-parse --short HEAD)

ifneq ("$(wildcard /usr/lib/librocksdb.so)","")
	SYS_LIB_DIR := /usr/lib
else ifneq ("$(wildcard /usr/lib64/librocksdb.so)","")
	SYS_LIB_DIR := /usr/lib64
else
	USE_SYS_ROCKSDB :=
endif

USE_SYS_ROCKSDB :=
SYS_ROCKSDB := $(if ${USE_SYS_ROCKSDB},ROCKSDB_LIB_DIR=${SYS_LIB_DIR},)

CARGO := env ${SYS_ROCKSDB} cargo

test:
	cargo build && ${CARGO} test ${VERBOSE} --all -- --skip trust_metric --nocapture

doc:
	cargo doc --all --no-deps

doc-deps:
	cargo doc --all

# generate GraphQL API documentation
doc-api:
	bash docs/build/gql_api.sh

check:
	${CARGO} check ${VERBOSE} --all

build:
	${CARGO} build ${VERBOSE} --release

check-fmt:
	cargo +nightly fmt ${VERBOSE} --all -- --check

fmt:
	cargo +nightly fmt ${VERBOSE} --all

sort:
	cargo sort -gw

check-sort:
	cargo sort -gwc

capsule-build:
	capsule build

ci: check-fmt clippy test

info:
	date
	pwd
	env

# For counting lines of code
stats:
	@cargo count --version || cargo +nightly install --git https://github.com/kbknapp/cargo-count
	@cargo count --separator , --unsafe-statistics

# Use cargo-audit to audit Cargo.lock for crates with security vulnerabilities
# expecting to see "Success No vulnerable packages found"
security-audit:
	@cargo audit --version || cargo install cargo-audit
	@cargo audit


unit-test: capsule-build

schema:
	make -C common/types/ schema

.PHONY: build prod prod-test
.PHONY: fmt test clippy doc doc-deps doc-api check stats
.PHONY: ci info security-audit

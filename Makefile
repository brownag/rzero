.PHONY: help build build-wasm test test-conformance test-gnu-r smoke lint format check run clean

.DEFAULT_GOAL := help
SHELL := /bin/bash

CARGO       ?= cargo
NODE        ?= node
WASM_TARGET ?= wasm32-unknown-unknown

help: ## Show this help message
	@awk 'BEGIN {FS = ":.*?## "} /^[a-zA-Z_-]+:.*?## / { printf "  \033[36m%-18s\033[0m %s\n", $$1, $$2 }' $(MAKEFILE_LIST)

build: ## Build native debug binary and library
	$(CARGO) build

build-wasm: ## Build WASM release target
	$(CARGO) build --target $(WASM_TARGET) --release

test: ## Run all Cargo tests
	$(CARGO) test

test-conformance: ## Run conformance test suite
	$(CARGO) test --test conformance

test-gnu-r: ## Run R0 vs GNU R equivalence harness (requires Rscript)
	@command -v Rscript >/dev/null 2>&1 || { echo "test-gnu-r: Rscript not found on PATH; install GNU R to run this harness" >&2; exit 1; }
	$(CARGO) test --test gnu_r

smoke: build-wasm ## Build WASM and run smoke test
	$(NODE) .development/smoke.mjs

lint: ## Run clippy linter
	$(CARGO) clippy

format: ## Format code with cargo fmt
	$(CARGO) fmt

check: ## Run cargo check
	$(CARGO) check

run: ## Run the CLI
	$(CARGO) run

clean: ## Clean build artifacts
	$(CARGO) clean

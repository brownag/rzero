# AGENTS.md - r0-wasm

Rust implementation of the `r0-wasm` language/VM (`cdylib` + `rlib`, edition 2021).

## Quick Start

Run `make help` to see all available `Makefile` targets, or use the commands directly:

* **Build (native)**: `make build` (`cargo build`)
* **Build (WASM release)**: `make build-wasm` (`cargo build --target wasm32-unknown-unknown --release`)
* **Test all**: `make test` (`cargo test`)
* **Test conformance**: `make test-conformance` (`cargo test --test conformance`)
* **Lint / Check**: `make check` / `make lint` (`cargo check` / `cargo clippy`)
* **Format**: `make format` (`cargo fmt`)
* **Run CLI**: `make run` (`cargo run`)
* **WASM smoke test**: `make smoke` (`node .development/smoke.mjs`)

## Find Things

* `src/` — Core crate (`lib.rs`, `main.rs`, `lexer.rs`, `parser.rs`, `ast.rs`, `isa.rs`, `value.rs`, `vm.rs`). Run `ls src` to see modules.
* `tests/` — Integration and conformance test suites (`conformance.rs`).
* `docs/` — Architecture, language specification, and adversarial oracle docs (`ARCHITECTURE.md`, `LANGUAGE_SPEC.md`, `ADVERSARIAL_ORACLE.md`).
* `planning/` — Execution plans, goals, state files, task specs (`planning/tasks/`), and `TODO_*.md` ledgers.
* `.development/` — Local development helpers and smoke scripts (`smoke.mjs`).
* `target/` — Cargo build artifacts (gitignored).

## Workflows

* **Planning & Execution**: Goals and wave plans live in `planning/` (`GOALS.md`, `EXECUTION_PLAN_*.md`, `STATE_*.md`, `TODO_*.md`). Update task status in `planning/` as work progresses.
* **Testing**: Run `cargo test` before committing changes; verify WASM target builds with `cargo build --target wasm32-unknown-unknown --release` when touching `lib.rs` or FFI boundaries.

## Infrastructure

* **Build**: `Makefile` (`make help` lists all targets)
* **Manifest**: `Cargo.toml` (`name = "r0-wasm"`, `crate-type = ["cdylib", "rlib"]`)
* **Release profile**: `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`
* **Entry points**: `src/lib.rs` (library / WASM `cdylib`), `src/main.rs` (binary)

## References

* [ARCHITECTURE.md](docs/ARCHITECTURE.md) — System architecture and VM design
* [LANGUAGE_SPEC.md](docs/LANGUAGE_SPEC.md) — Language specification
* [ADVERSARIAL_ORACLE.md](docs/ADVERSARIAL_ORACLE.md) — Adversarial testing and oracle notes

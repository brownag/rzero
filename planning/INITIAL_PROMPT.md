# Task: Scaffold, Extract, and Modularize the `r0-wasm` Project from Conversation Context

You have access to a design and architecture conversation transcript at `<PATH_TO_CONVO_TRANSCRIPT>` (or in your active context window). It contains three core artifacts for **R0 ("R-Zero")**, an embeddable, deterministic, strict subset of R targeting `wasm32-unknown-unknown`:

1. **R0 Language Specification v0.1** (types, `NA` representations, Kleene 3-valued logic, strict broadcasting, 1-based indexing rules, EBNF grammar, and the 48-intrinsic locked prelude).
2. **R0 Compiler & Wasm Execution Architecture Specification v0.1** (`R0Vec` 16-byte descriptor, dual sentinel/Arrow bitmap ABI, type/shape lattice, 256-element strip-mined L1 tile loop fusion, 32-bit fixed-width ISA, and `v128` SIMD kernels).
3. **Working Zero-Dependency Rust Prototype** (Newline-suppressing lexer, Pratt parser with `|>` and `%in%` desugaring, Kleene LUTs, vectorized evaluator, and pedon verification script).

## Objectives

Execute the following steps sequentially, committing cleanly after each milestone. Do not use external crates in the core library (`zero-dependency` pure Rust).

### Phase 1: Repository Initialization & Specification Extraction
1. Initialize a new Rust library + binary workspace/crate in the current directory (`Cargo.toml` configured with `edition = "2021"`, `[lib]` with `crate-type = ["cdylib", "rlib"]`, and `[profile.release]` set to `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`).
2. Extract the **R0 Language Specification v0.1** from the conversation into `docs/LANGUAGE_SPEC.md`. Preserve the full EBNF grammar, operator precedence table, broadcasting rules, and intrinsic definitions verbatim.
3. Extract the **R0 Compiler & Wasm Execution Architecture v0.1** into `docs/ARCHITECTURE.md`. Preserve the memory layout diagrams, `R0Vec` 16-byte struct spec, `FUSE_TILE` opcode sequence, and `v128` SIMD notes.

### Phase 2: Modularize the Working Prototype into the Target Crate Layout
Instead of leaving the prototype as a monolithic `main.rs`, refactor and split the working code from the transcript into clean, idiomatic modules matching the architecture specification:

```text
r0-wasm/
├── Cargo.toml
├── docs/
│   ├── LANGUAGE_SPEC.md
│   └── ARCHITECTURE.md
├── src/
│   ├── lib.rs             # Public API (`eval_script`, Wasm FFI exports, re-exports)
│   ├── main.rs            # CLI runner & self-contained pedon verification demo
│   ├── value.rs           # `R0Val`, `NA_REAL_BITS`, `NA_INT`, `Lgl`, Kleene 16-byte LUTs, `R0Vec` 16-byte ABI struct
│   ├── lexer.rs           # `Tok`, `tokenize()` with R newline-suppression state machine
│   ├── ast.rs             # `Expr`, `BinOp`, `UnaryOp`
│   ├── parser.rs          # Pratt `Parser`, `infix_bp`, `postfix_bp`, `desugar_pipe()`
│   ├── vm.rs              # `R0Vm`, fuel metering, strict broadcasting, 1-based indexing, prelude intrinsics
│   └── isa.rs             # Stub/scaffold 32-bit `Opcode` enum and `TileKernel` definitions from Architecture Spec Section 4
└── tests/
    └── conformance.rs     # Comprehensive integration test suite

```

### Phase 3: Expand Verification into `tests/conformance.rs`

Convert the inline checks from the prototype into a rigorous `cargo test` suite in `tests/conformance.rs` covering:

1. **R Newline Suppression:** Multiline arithmetic (`1 +\n 2`), multiline `|>` pipelines, and newlines inside `(...)` and `[...]` vs. statement-terminating newlines.
2. **Pipe Desugaring:** Implicit first-argument insertion (`x |> sum(na.rm = TRUE)`) and explicit `_` placeholder substitution (`x |> approx(y, xout = _)`).
3. **Kleene Three-Valued Logic:** Exhaustive truth table checks for `&`, `|`, and `!` across `TRUE`, `FALSE`, and `NA` (e.g., `FALSE & NA == FALSE`, `TRUE | NA == TRUE`, `TRUE & NA == NA`).
4. **Strict Broadcasting Trap:** Verify scalar-to-vector (`1 + c(10, 20)`) and equal-length (`c(1, 2) + c(3, 4)`) succeed, while mismatched vector lengths (`c(1, 2, 3, 4) + c(10, 20)`) return `ErrDimensionMismatch`.
5. **1-Based Indexing & OOB Semantics:** Positive indexing (`x[1]`), out-of-bounds `NA` padding (`x[99L]` -> `NA`), negative drop indexing (`x[-2L]`), mixed positive/negative trap (`ErrMixedSubscript`), logical mask filtering (`x[x > 20]`), scalar extraction (`x[[1L]]` and out-of-bounds trap), and in-place subset assignment (`x[x < 0] <- 0`).
6. **Domain Pedometrics Integration Test:** The 5-horizon `pedon` depth-weighted clay aggregation (`0–100 cm` returning `~25.661`) and `approx()` linear depth interpolation at `c(25, 50, 85)`.
7. **Deterministic Fuel Metering:** Verify an infinite loop (`while (TRUE) { x <- 1 }`) cleanly halts with `ErrFuelExhausted`.

### Phase 4: Wasm Compilation & Verification Gate

1. Add C-ABI/Wasm FFI entrypoints in `src/lib.rs` (`r0_eval_str_ptr` or a minimal linear-memory host harness) so `cargo build --target wasm32-unknown-unknown --release` produces a valid `.wasm` artifact.
2. Run and verify:
* `cargo test` (all unit and conformance tests pass)
* `cargo run --release` (runs the pedon verification script and prints output)
* `cargo clippy -- -D warnings` (clean with zero warnings)
* If `wasm32-unknown-unknown` target is installed, build the release `.wasm` binary and report its exact byte size.



## Constraints

* **Zero external runtime dependencies** in `Cargo.toml` (`std` is fine for the initial host/Wasm prototype, or `core`+`alloc` where practical).
* Do not water down or omit any semantics from the conversation prototype.
* Keep error messages deterministic and prefixed with trap codes (`ErrDimensionMismatch`, `ErrInvalidCondition`, `ErrMixedSubscript`, `ErrBoundsTrap`, `ErrFuelExhausted`).


# r0-wasm

`r0-wasm` is a deterministic, fuel-metered interpreter for R0, a strict subset of GNU R. It compiles to a native binary and a standalone WebAssembly module with zero third-party Rust dependencies.

## What R0 supports

R0 uses standard R syntax with optional `: type` parameter annotations, optional `-> type` return annotations, and `fn` as an alias for `function`.

- **Types:** `dbl`, `int`, `lgl`, `chr`, `range`, `df`, `list`, and `NULL`. Scalars are length-1 vectors.
- **Sandboxing:** Loops, vector operations, and range materializations consume a fuel budget (`DEFAULT_FUEL = 100_000` in `eval_script` / `r0_eval`).
- **Value semantics:** Subset assignment (`x[i] <- v`) rebinds the identifier without mutating aliased bindings.
- **Strictness traps:** Dimension mismatches, invalid conditions, mixed subscripts, out-of-bounds writes, integer overflow, and fuel exhaustion fail with explicit errors.

## Usage

Run a script from the CLI:

```bash
cargo run -- -e "sum(1:10)"
```

Evaluate R0 from Rust:

```rust
use r0_wasm::{eval_script, DEFAULT_FUEL};

let val = eval_script("x <- 1:5; sum(x)", DEFAULT_FUEL).unwrap();
```

For WebAssembly, build the `cdylib` target and call the exported `r0_eval` entry point.

## Documentation

- [Language Specification](docs/LANGUAGE_SPEC.md)
- [Architecture](docs/ARCHITECTURE.md)
- [Adversarial Oracle](docs/ADVERSARIAL_ORACLE.md)

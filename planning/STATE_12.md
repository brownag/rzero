# STATE_12: Anonymous Functions and Closures (G24, G25, G26, G27, G28, G30, G32, G33, G34)

Branch: `g12-transpiler` (local). Orchestration 12 is not started.

## Baseline (2026-11-15, before wave 1)
- `cargo test --lib`: 134 passed, 0 failed.
- `cargo test --test conformance`: 80 passed, 0 failed.
- `cargo test --test gnu_r -- --test-threads=1`: 4 passed, 0 failed (about 20s; needs `Rscript`).
- `cargo build --target wasm32-unknown-unknown --release`: exit 0 (verified in plan 11).
- `cargo fmt --check` and `cargo clippy`: clean at plan 11 completion; re-check at wave 6.

## Wave Board

| Wave | Task ID | Title | Status | Commit |
|---|---|---|---|---|
| 1 | `12.1.1` | `R0Val` function variant (G24) | pending | — |
| 1 | `12.1.2` | Lexer `\` introducer and transpiler test (G25, G30) | pending | — |
| 2 | `12.2.1` | Unnamed function expressions evaluate to values (G26) | pending | — |
| 2 | `12.2.2` | Function values are not printable (G34) | pending | — |
| 3 | `12.3.1` | `Expr::Call` takes a callee expression (G27, part 1) | pending | — |
| 4 | `12.3.2` | Calls through function values (G27, part 2) | pending | — |
| 5 | `12.4.1` | Closure scoping rule (G28) | pending | — |
| 6 | `12.5.1` | Corpus `g_anon_*` and gnu_r equivalence (G32) | pending | — |
| 6 | `12.5.2` | Spec for anonymous functions, closures, relaxed Axiom 1 (G33) | pending | — |

## Task Reports

## 12.1.2
- Status: done, commit c631c08 (`feat(lexer): lex \( as function introducer`).
- `src/lexer.rs`: guarded arm `'\\' if` next non-space/tab byte is `(` pushes `Tok::Fn`; otherwise the existing "Unexpected character" arm.
- Tests added: lexer `backslash_lambda_is_fn`, `bare_backslash_errors`; transpile `backslash_lambda`.
- `cargo test --lib lexer`: 13 passed, 0 failed.
- `cargo test --lib transpile`: 8 passed, 0 failed.
- `cargo fmt --check`, `cargo clippy`, wasm32 release build: clean.
- Diff: only src/lexer.rs (+16) and src/transpile.rs (+5, test module).

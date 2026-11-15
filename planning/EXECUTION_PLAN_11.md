# EXECUTION_PLAN_11: G12 (`.R0 -> .R` Transpiler)

Status: APPROVED by user 2026-11-15. Not started. Run wave 1 (11.1.1) when instructed.

## Goal & Scope

- **Goal**: Implement a pure-Rust `.R0 -> .R` transpiler (`transpile_to_r`) that mechanically strips R0-only syntax (`fn`, `: <type>`, `-> <type>`) so R0 sources run under GNU R (`Rscript`), and verify equivalence against `R0Vm` in `tests/gnu_r.rs`.
- **In scope**:
  - `src/transpile.rs` (`pub fn transpile_to_r(src: &str) -> Result<String, String>`) and `pub mod transpile;` in `src/lib.rs`.
  - Token-level rewrite on the `src/lexer.rs` stream: `fn` → `function`, strip `: <type>` inside `fn`/`function` parameter lists, strip `-> <type>` return annotations.
  - R0-only corpus additions in `tests/r0/` (including `PEDON_SCRIPT` and `fn` + default-arg + `1:10` cases) and equivalence test in `tests/gnu_r.rs`.
  - Clean `wasm32-unknown-unknown` `cdylib` build.
- **Out of scope (explicitly deferred)**:
  - Preserving original whitespace and comments (normalized token-stream output is acceptable).
  - CLI `--transpile` / `--emit-r` flag in `src/main.rs` (deferred until `tests/gnu_r.rs` passes).
  - Rightward assignment `->` (not supported in R0; `->` is reserved exclusively for return type annotations).
- **Verbatim constraints**:
  - "Read-only research" (satisfied in `RESEARCH_G12_TRANSPILER.md`).
  - "The crate is `cdylib` and `rlib`, and the WASM build must stay clean."

---

## Decision Log

- **2026-11-15 — Token-level rewrite**: Use a token-level rewrite on `src/lexer.rs` rather than AST pretty-printing (loses `|>` surface syntax) or regex (fragile on strings, comments, `1:10`).
- **2026-11-15 — Drop type hints**: Strip `: <type>` and `-> <type>` without emitting `stopifnot(...)`, matching R0's runtime behavior (`src/parser.rs:237, 254` discards them).
- **2026-11-15 — Normalized output**: Whitespace and comment preservation is not required.
- **2026-11-15 — Defer CLI**: Wait on `src/main.rs` CLI exposure until `tests/gnu_r.rs` equivalence passes.
- **2026-11-15 — No rightward assignment**: `->` is reserved exclusively for return types.
- **2026-11-15 — Placement**: `pub mod transpile;` in `src/lib.rs` (pure Rust, no `std::fs`/`std::process`) so `wasm32-unknown-unknown` stays clean.
- **2026-11-15 — Planning artifacts**: Tracked in Git (`docs(planning):` commits separate from code).
- **2026-11-15 — Workers**: `flash` (or `inherit`), escalating to `hydrogen` on retry failure; max 2 concurrent workers.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Files in Scope | Depends On |
|---|---|---|---|---|
| 1 | `11.1.1` | Implement `transpile_to_r` module & export | `src/transpile.rs` (create), `src/lib.rs` (edit) | — |
| 2 | `11.2.1` | Add R0-only corpus & `gnu_r.rs` equivalence test | `tests/gnu_r.rs` (edit), `tests/r0/g12_*.R0` (create) | `11.1.1` |

---

## Hazard Notes

- `src/lexer.rs` (`Tok` enum) does not carry byte spans; emit normalized tokens with spaces/newlines as needed so GNU R parses all constructs unambiguously (e.g. `function(...)`, `<-`, `|>`).
- `1:10` sequence expressions use `Tok::Colon` outside parameter lists; only strip `: <type>` when inside a `Tok::Fn` parameter list `(...)` at depth 1.
- `tests/gnu_r.rs` invokes `Rscript --vanilla`; run `cargo test --test gnu_r` serially at wave end.

---

## Task Specs

### Task 11.1.1: Implement `transpile_to_r` module & export
- **id**: `11.1.1`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *can create*: `src/transpile.rs`
  - *must edit*: `src/lib.rs`
  - *do not touch*: `src/vm.rs`, `src/parser.rs`, `src/main.rs`, `tests/`
- **depends-on**: none
- **anchors**:
  - `src/lexer.rs:239` (`"function" | "fn" => Tok::Fn`)
  - `src/lexer.rs:267-269` (`"->" => Tok::Arrow`)
  - `src/lexer.rs:305` (`':' => Tok::Colon`)
  - `src/lib.rs` (module exports)
- **spec**:
  Implement `pub fn transpile_to_r(src: &str) -> Result<String, String>` in `src/transpile.rs` using `crate::lexer::Lexer` (or `lex`). Rewrite `Tok::Fn` to `function`, strip `: <type>` inside `fn`/`function` parameter lists (depth 1), and strip `-> <type>` immediately after `)` of `fn`/`function`. Keep `1:10` and `|>` intact. Export `pub mod transpile;` in `src/lib.rs`. Include unit tests in `src/transpile.rs` covering `fn`, `: type`, `-> type`, default args, `1:10`, and `list(f = fn(x: dbl) -> dbl x)`.
- **acceptance**:
  - `cargo test --lib transpile` passes
  - `cargo build --target wasm32-unknown-unknown --release` exits 0

### Task 11.2.1: Add R0-only corpus & `gnu_r.rs` equivalence test
- **id**: `11.2.1`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *can create*: `tests/r0/g12_transpile.R0`
  - *must edit*: `tests/gnu_r.rs`
  - *do not touch*: `src/`, `tests/conformance.rs`
- **depends-on**: `11.1.1`
- **anchors**:
  - `tests/gnu_r.rs` (`r0_corpus_matches_gnu_r`)
  - `tests/conformance.rs:212-239` (`PEDON_SCRIPT` reference)
- **spec**:
  Add `tests/r0/g12_transpile.R0` exercising `fn`, `: <type>`, `-> <type>`, default arguments with type hints, `fn` inside `list(...)`, and `1:10`. In `tests/gnu_r.rs`, add a test `r0_transpiled_matches_gnu_r` that transpiles R0-only `.R0` files (including `g12_transpile.R0`) via `r0_wasm::transpile::transpile_to_r`, runs the transpiled `.R` under `Rscript --vanilla` with `tests/gnu_r/canon.R`, and asserts identical canonical output to `R0Vm::eval_script` on the original `.R0`.
- **acceptance**:
  - `cargo test --test gnu_r` passes

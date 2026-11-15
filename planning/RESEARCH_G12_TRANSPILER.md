# Research Memo: Direction for G12 (`.R0 -> .R` Transpiler)

**Date**: 2026-11-15
**Goal**: `planning/GOALS.md:8` (G12 — depends on G11, closed)
**Status**: Research / Direction Proposal

---

## 1. Exact R0-Only Syntax Set

Every construct in R0 that GNU R does not accept:

| Construct | Spec Citation | Lexer / Parser Citation | Classification | Notes |
|---|---|---|---|---|
| `fn` keyword alias for `function` | `docs/LANGUAGE_SPEC.md:5, 219, 330` | `src/lexer.rs:239` (`"function" \| "fn" => Tok::Fn`); `src/parser.rs:226-258` | **Pure rewrite** (`fn` → `function`) | Both `fn` and `function` lex to `Tok::Fn`. |
| Parameter type annotations (`: type`) | `docs/LANGUAGE_SPEC.md:5, 84, 330` | `src/lexer.rs:305` (`':' => Tok::Colon`); `src/parser.rs:234-238` | **Deletion** (strip `: <type>`) | Parsed inside `Tok::Fn` param list and immediately discarded (`_type_name`). |
| Return type annotations (`-> type`) | `docs/LANGUAGE_SPEC.md:5, 84, 219, 330` | `src/lexer.rs:267-269` (`"->" => Tok::Arrow`); `src/parser.rs:251-255` | **Deletion** (strip `-> <type>`) | Parsed immediately after `)` of `Tok::Fn` and discarded (`_ret_type`). |
| Rightward assignment (`->`) | `docs/LANGUAGE_SPEC.md:330` (mentioned in spec, not in R0) | `src/parser.rs:252, 299-301` | **Not implemented in R0** | `Tok::Arrow` is only handled for return types; `infix_bp` only handles `Tok::Assign` (`<-`, `=`). |

---

## 2. Rewrite Strategy & Type Hints

### Recommended Strategy: (a) Token-Level Rewrite on the Lexer Stream
- `src/lexer.rs` already tokenizes string literals, comments, `Tok::Fn`, `Tok::Colon`, `Tok::Arrow`, and parentheses/braces.
- A token-level pass tracking `Tok::Fn` and `(` ... `)` nesting depth can unambiguously distinguish:
  - `: type` inside a `fn`/`function` parameter list vs. `1:10` sequence expressions,
  - `-> type` immediately after the closing `)` of `fn`/`function`,
  - `fn` inside `list(...)` or nested calls,
  - without altering `|>` pipes or operator precedence.
- **Implementation detail**: `Tok` in `src/lexer.rs` does not currently carry byte spans. Preserving original whitespace and comments requires either adding byte spans to `Tok` or emitting formatted tokens.

### Rejected Options
- **(b) Parse to existing AST, then print R source — Rejected**: `src/ast.rs:18-43` has no source spans, comments, or formatting, and `src/parser.rs:316` desugars `|>` pipes into nested `Call`s during parsing, losing original surface syntax.
- **(c) Text-level regex — Rejected**: Fragile on nested expressions, string literals, comments, and `:` (`1:10` vs. `: type`).

### Type Hints: Drop vs. Runtime Checks
- **Current R0 behavior**: R0 enforces **nothing** at runtime. `src/parser.rs:237, 254` binds `_type_name` and `_ret_type` and discards them; `Expr::FnDef` (`src/ast.rs:35`) only stores `params: Vec<(String, Option<Expr>)>` and `body: Box<Expr>`, and `src/vm.rs:134` performs no type checking.
- **Recommendation**: **Drop type hints.** `docs/LANGUAGE_SPEC.md:13, 84, 330` explicitly states that type annotations are ignored at runtime and that the `.R0 -> .R` transpiler strips them. Emitting `stopifnot(...)` would diverge from R0's actual runtime behavior and violate GNU R equivalence.

---

## 3. Equivalence Criterion & Corpus Requirements

- **Existing harness**: `tests/gnu_r.rs` already compares `R0Vm` canonical output against `Rscript` (`tests/gnu_r/canon.R`) over `tests/r0/*.R0`.
- **Proposed equivalence test**: Transpile each R0-only `.R0` file to `.R`, execute it under `Rscript --vanilla` with `tests/gnu_r/canon.R`, and assert identical canonical output to `R0Vm::eval_script` on the original `.R0` source.
- **Corpus gap**: `PEDON_SCRIPT` (`tests/conformance.rs:212-239`, `src/main.rs:7-34`) exercises `: df`, `: dbl`, and `-> dbl`, but **does not use `fn`**. `tests/r0/g11_pedon_twin.R0` uses `function(...)` with no R0-only syntax.
- **Required corpus additions**:
  1. `PEDON_SCRIPT` (typed `function` with `: df`, `: dbl`, `-> dbl`).
  2. `fn` alias inside `list(...)`, nested calls, and with default arguments + type hints (e.g., `fn(x: dbl = 1) -> dbl { x + 1 }`).
  3. Expressions containing `:` (e.g., `1:10`) inside and outside `fn` bodies/defaults to verify no false stripping.

---

## 4. Placement Recommendation

- `Cargo.toml:7` builds `crate-type = ["cdylib", "rlib"]`; `src/lib.rs` exports modules for both WASM (`cdylib`) and native targets (`rlib`).
- **Recommendation**: Add `pub mod transpile;` in `src/lib.rs` as pure Rust (zero external dependencies, no `std::fs` or `std::process`), exposing `pub fn transpile_to_r(src: &str) -> Result<String, String>`.
  - Keeps the `wasm32-unknown-unknown` `cdylib` build clean.
  - Directly callable from `tests/gnu_r.rs` and `src/main.rs` (or a future CLI flag/binary if needed).

---

## 5. Scope, Effort, and First Plan-Wave Slice

- **24-hour feasibility**: **Yes**, because R0-only syntax is limited to `fn`, `: <type>`, and `-> <type>`.
- **First verifiable slice (`G12.1: Core Token Transpiler & GNU R Equivalence`)**:
  1. Add byte spans to `Tok` (or a span-tracking lexer wrapper) in `src/lexer.rs`.
  2. Implement `src/transpile.rs` (`transpile_to_r`) handling `fn` → `function`, `: <type>` param stripping, and `-> <type>` return stripping.
  3. Add R0-only corpus cases (including `PEDON_SCRIPT` and `fn` cases) and wire transpilation into `tests/gnu_r.rs`.
- **Verification commands**:
  - `cargo test --test gnu_r`
  - `cargo build --target wasm32-unknown-unknown --release`

---

## 6. Decisions (2026-11-15)

1. **Whitespace & comments**: Not required; normalized token-stream output is acceptable.
2. **CLI exposure**: Defer `--transpile` / CLI flag in `src/main.rs` until `tests/gnu_r.rs` equivalence passes.
3. **Rightward assignment**: Not supported; `->` is reserved exclusively for return type annotations.

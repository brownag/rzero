# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-14
Source State: STATE_4.md

## Circled Goal (Active)

- **[G9]** Loop control statements (`break` and `next`) and 2D DataFrame subsetting (`df[i, j]` / `df[mask, ]` / `df[, j]`) are implemented across `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` with conformance coverage.
  - Why now: G8 is complete (`STATE_4.md`); G9 is the last blocker for G10.
  - Exit criteria: `break`/`next` work in `for`/`while` (incl. nested) with deterministic fuel; 2D DataFrame subsetting works end-to-end; documented in `docs/LANGUAGE_SPEC.md`; `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release` all pass.
  - First action: `/plan-wave` drafts `planning/EXECUTION_PLAN_5.md`.

## Sequenced Goals (Dependency DAG)

- None.

## Independent Backlog

- **[G2]** Unary negation (`-x`) and `abs(x)` on `Int` vectors preserve `NA_INT` (`i32::MIN`) and trap with `ErrIntOverflow` on two's-complement overflow, covered by unit and conformance tests.
- **[G3]** `src/lexer.rs` validates matching open/close delimiters `(`, `[`, and `[[` so unbalanced or mismatched closing delimiters (`([)]`, `1 )`, `1 ]`) fail cleanly with a deterministic parse error (`[STATE-02]`).
- **[G4]** Read-only paths in `src/value.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` eliminate redundant allocations (`v.clone()` on already-typed `Dbl`/`Int` views, `self.peek().clone()` token cloning, `Vec<String>` allocations in `Display`, and dead `BinOp::InOp`) per `docs/ADVERSARIAL_ORACLE.md` §2.3.
- **[G5]** User-defined function declaration and invocation in `src/vm.rs` avoid full `HashMap<String, R0Val>` cloning per call by capturing only free variables or using a scoped lexical frame stack while preserving Axiom 4 value semantics.

## Deferred

- **[G10]** The `R0Vec` 16-byte descriptor (`src/value.rs`) and 63-opcode ISA (`src/isa.rs`) are wired to an SSA compiler and vector-register bytecode execution engine supporting loop-fused tile kernels (`docs/ARCHITECTURE.md` §§4–8).
  - Trigger: G9 is complete and verified by conformance tests.

## Completed

- **[G1]**, **[G6]**, **[G7]** (see prior `STATE_N.md`).
- **[G8]** Search, sorting, binning intrinsics + `approx(rule)` / `diff(lag)` (`STATE_4.md`, commits `179ebe5`..`ec71e81`). Residue: 6 `clippy::unnecessary_sort_by` errors in `src/vm.rs` (`order`/`rank` Int/Lgl paths) — folded into G9 Wave 1.

## Blocked / Pruned

- None.

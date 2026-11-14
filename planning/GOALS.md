# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-14
Source State: STATE_1.md, TODO_20261114-1.md, TODO_20261114-2.md, TODO.md

## Circled Goal (Active)

- **[G1]** `r0-wasm` completes all remaining v0.2 runtime discrepancy fixes in `TODO.md` (`UnaryOp::Pos`/`Neg` on `Range` and non-numeric types, locked prelude intrinsic shadowing rejection and strict user-function argument binding, `na.rm` validation helper across `sum`/`mean`/`min`/`max`/`pmin`/`pmax`, and `docs/LANGUAGE_SPEC.md` §7.1 alignment), passes all unit, conformance, clippy, and `wasm32` release gates, and archives `TODO.md` to `planning/TODO_20261114-3.md`.
  - Why now: Establishes the locked prelude invariant, strict argument-binding rules, and reusable `na.rm` validation infrastructure required before expanding the R0 intrinsic surface (`G6`–`G8`), while closing out the active `TODO.md` workstream.
  - Exit criteria: `+(1:3)`, `-(1:3)`, `+TRUE`, `-"a"`, prelude shadowing rejection, strict user-function argument errors (`UnknownArgument`, `DuplicateArgument`, `TooManyArguments`), and `na.rm` edge cases (`NA`, non-logical, empty logical, length > 1) are covered by passing unit and conformance tests; `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release` exit 0; `TODO.md` is archived to `planning/TODO_20261114-3.md`.
  - First action: Inspect `Expr::Unary` in `src/vm.rs` and implement Task 2 (`UnaryOp::Pos` and `UnaryOp::Neg` on `Range` and non-numeric types) with unit and conformance tests.

## Sequenced Goals (Dependency DAG)

- **[G6]** The 12 planned constructor, coercion, and shape intrinsics (`rep`, `rep_len`, `seq`, `numeric`, `integer`, `logical`, `character`, `as.numeric`, `as.integer`, `as.logical`, `nrow`, `ncol`) are implemented in `src/vm.rs` with pre-allocation fuel metering, locked prelude registration, and conformance tests.
  - Depends on: G1
  - Unblocks: G7, G8, G9

- **[G7]** The 21 planned predicate, logical reduction, trigonometric/rounding math, and statistical/cumulative reduction intrinsics (`is.finite`, `is.infinite`, `any`, `all`, `anyNA`, `sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2`, `prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax`) are implemented and verified against GNU R semantics.
  - Depends on: G1, G6
  - Unblocks: G8

- **[G8]** The 10 planned search, sorting, and binning intrinsics (`which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval`) plus optional `rule` on `approx` and `lag` on `diff` are implemented with deterministic fuel metering, completing all 69 specified R0 intrinsics.
  - Depends on: G1, G6, G7

- **[G9]** Loop control statements (`break` and `next`) and 2D DataFrame subsetting (`df[i, j]` / `df[mask, ]`) are implemented across `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` with conformance coverage.
  - Depends on: G1, G6

## Independent Backlog

- **[G2]** Unary negation (`-x`) and `abs(x)` on `Int` vectors preserve `NA_INT` (`i32::MIN`) and trap with `ErrIntOverflow` on two's-complement overflow, covered by unit and conformance tests.
- **[G3]** `src/lexer.rs` validates matching open/close delimiters `(`, `[`, and `[[` so unbalanced or mismatched closing delimiters (`([)]`, `1 )`, `1 ]`) fail cleanly with a deterministic parse error (`[STATE-02]`).
- **[G4]** Read-only paths in `src/value.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` eliminate redundant allocations (`v.clone()` on already-typed `Dbl`/`Int` views, `self.peek().clone()` token cloning, `Vec<String>` allocations in `Display`, and dead `BinOp::InOp`) per `docs/ADVERSARIAL_ORACLE.md` §2.3.
- **[G5]** User-defined function declaration and invocation in `src/vm.rs` avoid full `HashMap<String, R0Val>` cloning per call by capturing only free variables or using a scoped lexical frame stack while preserving Axiom 4 value semantics.

## Deferred

- **[G10]** The `R0Vec` 16-byte descriptor (`src/value.rs`) and 63-opcode ISA (`src/isa.rs`) are wired to an SSA compiler and vector-register bytecode execution engine supporting loop-fused tile kernels (`docs/ARCHITECTURE.md` §§4–8).
  - Trigger: All 69 prelude intrinsics (`G1`, `G6`, `G7`, `G8`) and core R0 language semantics (`G9`) are complete and verified by conformance tests.

## Blocked / Pruned

- None.

# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-14
Source State: STATE_2.md

## Circled Goal (Active)

- **[G7]** The 21 planned predicate, logical reduction, trigonometric/rounding math, and statistical/cumulative reduction intrinsics (`is.finite`, `is.infinite`, `any`, `all`, `anyNA`, `sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2`, `prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax`) are implemented and verified against GNU R semantics.
  - Why now: G1 and G6 are complete; G7 is the next root bottleneck that unblocks G8 (completing all 69 specified R0 intrinsics).
  - Exit criteria: All 21 intrinsics are registered in the locked prelude, implemented in `src/vm.rs` with deterministic fuel metering and `na.rm` validation where applicable, documented in `docs/LANGUAGE_SPEC.md` §7.1, and verified by passing unit + conformance tests, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`.
  - First action: Hand G7 to `/plan-wave` to inspect `src/vm.rs` and `docs/LANGUAGE_SPEC.md` and draft `planning/EXECUTION_PLAN_3.md`.

## Sequenced Goals (Dependency DAG)

- **[G8]** The 10 planned search, sorting, and binning intrinsics (`which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval`) plus optional `rule` on `approx` and `lag` on `diff` are implemented with deterministic fuel metering, completing all 69 specified R0 intrinsics.
  - Depends on: G7 (G1, G6 complete)

- **[G9]** Loop control statements (`break` and `next`) and 2D DataFrame subsetting (`df[i, j]` / `df[mask, ]`) are implemented across `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` with conformance coverage.
  - Depends on: none (G1, G6 complete)

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

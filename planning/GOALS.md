# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-15
Source State: STATE_9.md (orchestration 9 gate green)

## Circled Goal (Active)

- **[G12]** A `.R0 -> .R` transpiler strips R0-only syntax (`fn`, `->`, type hints, and similar) so R0 sources run under GNU R.
  - Unblocked: G11 is complete. The GNU-valid pedon twin is `tests/r0/g11_pedon_twin.R0`, the transpiler's test input.
- **[G10]** Deferred, see below. Listed here only for its dependency on G12.

## Sequenced Goals (Dependency DAG)

- **[G10]** Deferred, see below. Listed here only for its dependency on G12.

## Independent Backlog

- **[G20]** `var`, `sd`, and `atan2` with NaN return `NA` where GNU R does. Changes `eval_var_sd` and `atan2` in `src/vm.rs`, the matching conformance asserts in `conformance_g7_statistical_reductions` and the atan2 test, and a spec errata entry.
  - Depends on: none. Lifts the `atan2(NaN, NA_real_)` exclusion and any class-B program that touches `var`/`sd` NaN.
- **[G21]** Data frame single-index assignment and `diff` keep integer type. R0 currently coerces `d[c(1, 2)] <- 1:3` and `diff(1:5, lag = 2)` to double. The fix is in `src/`.
  - Depends on: none. Lifts the type-only exclusions in orchestration 8.
- **[G14]** `break`/`next` inside an `if`/`while` condition expression work inside a loop, via a `ConditionFlow` variant in `src/vm.rs`. The spec still lists this as a current limitation (§3).
  - Depends on: none.
- **[G13]** `docs/LANGUAGE_SPEC.md` and the `planning/EXECUTION_PLAN_5.md` Decision Log state one rule for logical masks: a short mask errors, matching the strict code behaviour in `src/vm.rs`.
  - Note: spec §3 still allows a length-1 mask, which is a recycle. Reconcile before closing G13. Not verified in this pass.
- **[G15]** The G9 fuel test (`conformance_g9_fuel_exhaustion_large_frame`) is verified to fire `ErrFuelExhausted` at the subset step, and tightened if it does not.
- **[G16]** `cargo fmt --check` is clean, with formatting changes in their own commit. Plan 6 baseline still shows it failing.
- **[G2]** Unary negation (`-x`) and `abs(x)` on `Int` vectors preserve `NA_INT` and trap with `ErrIntOverflow` on overflow, covered by unit and conformance tests.
- **[G3]** `src/lexer.rs` validates matching open/close delimiters (`(`, `[`, `[[`) so unbalanced or mismatched closers fail with a deterministic parse error (`[STATE-02]`).
- **[G4]** Read-only paths in `src/value.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` eliminate redundant allocations per `docs/ADVERSARIAL_ORACLE.md` §2.3.
- **[G5]** User-defined function invocation in `src/vm.rs` avoids full `HashMap<String, R0Val>` cloning per call while preserving Axiom 4 value semantics.

## Deferred

- **[G10]** The `R0Vec` 16-byte descriptor (`src/value.rs`) and 63-opcode ISA (`src/isa.rs`) are wired to an SSA compiler and vector-register bytecode engine supporting loop-fused tile kernels (`docs/ARCHITECTURE.md` §§4–8).
  - Trigger: G11 and G12 are complete and verified, and a performance need is measured.

## Completed

- **[G1]**, **[G6]**, **[G7]** (see prior `STATE_N.md`).
- **[G8]** Search, sorting, binning intrinsics + `approx(rule)` / `diff(lag)` (`STATE_4.md`).
- **[G9]** `break`/`next` and 2D / 1-index DataFrame and list subsetting, read and write (`STATE_5.md`). Recycling is deliberately not supported; see the direction recorded in the project memory.
- **[G11, harness gate]** Canonical comparison, runner, allowlist, `make test-gnu-r` (`STATE_6.md`).
- **[G11]** Closed in orchestration 9 (`STATE_9.md`). `tests/r0/` has 88 corpus files, `allowlist.txt` has no entries. The GNU-valid pedon twin, the B8–B12 errata files, and the restriction list in `docs/LANGUAGE_SPEC.md` "GNU R equivalence" are in. Exclusions are recorded as restrictions or linked to G20/G21. Earlier waves: harness gate (`STATE_6.md`), corpus waves 1–2 (orchestrations 7–8).
- **[G17]** Loop value is `NULL` invisibly, as in GNU R (orchestration 7, task 7.1.1).
- **[G18]** Three R0 results align to GNU R (orchestration 7): list `[<-` with an atomic value, `median` with missing values, and `atan2` with `NULL`.
- **[G19, na.rm part]** A non-scalar `na.rm` is an error in every intrinsic that takes it (orchestration 7, task 7.2.1). Spec errata B15.

## Blocked / Pruned

- **[G22]** Bare `c()` as input to `sin`, `order`, and `tabulate` (GNU R errors; R0 result differs).
  - Status: Pruned
  - Reason: an R0 restriction to record under G11, not a code change.
- **[G23]** `approx(..., 2)` (GNU R reads the 4th positional argument as `method` and errors).
  - Status: Pruned
  - Reason: an R0 restriction to record under G11, not a code change.

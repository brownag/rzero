# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-14
Source State: STATE_6.md

## Circled Goal (Active)

- **[G11]** The `tests/r0/` corpus covers every GNU-compatible R0 program in `tests/conformance.rs`, and each corpus program matches GNU R (`Rscript`) or its difference is listed as an explicit R0 restriction.
  - Why now: the harness gate is built and green (plan 6, 23 programs, zero allowlist entries). Its coverage is still small: 23 files against 80 conformance tests and roughly 528 Rust assertions. Plan 6 excluded migration on purpose. G12 cannot be verified until the corpus includes user-defined functions and other R0-only-adjacent syntax, and G17 needs the loop cases.
  - Exit criteria: `tests/conformance.rs` R0 programs are classified (GNU-compatible, or R0-only and excluded); every GNU-compatible one has a `tests/r0/*.R0` file; `cargo test --test gnu_r` and `make test-gnu-r` exit 0; the errata programs B1–B10 from `docs/LANGUAGE_SPEC.md` §9 are covered where GNU R has the same behaviour.
  - Classification: `planning/tasks/g11-conformance-classification.md` (80 test functions: 32 GNU-compatible, 28 mixed, 15 reject or resource, 4 R0-only pedon script, 1 known gap).
  - Wave 1 done (2026-11-14): 32 class-A corpus programs added (`tests/r0/`, one per test function). Gate green: `cargo test`, `make test-gnu-r`. Side change: `approx` now returns `list(x, y)` as in GNU R (D-1, `TODO.md`).
  - Wave 2 scope: the value asserts from the 28 class-B tests in `planning/tasks/g11-conformance-classification.md`. Error asserts stay in Rust.
  - First action: `/plan-first` for wave 2.

## Sequenced Goals (Dependency DAG)

- **[G12]** A `.R0 -> .R` transpiler strips R0-only syntax (`fn`, `->`, type hints, and similar) so R0 sources run under GNU R.
  - Depends on: G11 (the corpus must include R0-only input so the transpiler has a test. Plan 6 excluded this input on purpose, so G11 alone is not enough.)

## Independent Backlog

- **[G17]** Loop return value aligned to GNU R. `for` and `while` return `Null` invisibly, not the last body value. Decided 2026-11-14 (was spec §3, restriction §8). Changes `src/vm.rs` loop evaluation and the spec §3 loop text.
  - Exit criteria: loop programs match GNU R in the harness; spec §3 and §8 updated; the §8 loop-value restriction is removed; Rust tests that assert the old loop value are updated.
- **[G18]** Three R0 results align to GNU R (decided 2026-11-14, from `planning/tasks/g11-conformance-classification.md` D2, D3, D5):
  - `l["b"] <- c(1,2,3)` on a list sets `b` to the first value with a warning, not all three.
  - `median(c(1, NaN, 3))` returns `NA`, not `NaN`.
  - `atan2(c(), 1)` errors (non-numeric argument), not `numeric(0)`.
  - Exit criteria: each fixed in `src/vm.rs` with a conformance test and a corpus program that matches GNU R.
- **[G19]** `sum(x, na.rm = c(TRUE, FALSE))` stays lenient in R0 (first element used). Record it as a restriction in `docs/LANGUAGE_SPEC.md` §8 with a corpus program that pins it (decided 2026-11-14, D4).
- **[G13]** `docs/LANGUAGE_SPEC.md` and the `planning/EXECUTION_PLAN_5.md` Decision Log state one rule for logical masks: a short mask errors, matching the strict code behaviour in `src/vm.rs`.
  - Note: spec §3 line 146 still allows a length-1 mask, which is a recycle. Reconcile this with the strict rule before closing G13. Not verified in this pass.
- **[G14]** `break`/`next` inside an `if`/`while` condition expression work inside a loop, via a `ConditionFlow` variant in `src/vm.rs`. The spec still lists this as a current limitation (§3).
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
- **[G9]** `break`/`next` and 2D / 1-index DataFrame and list subsetting, read and write (`STATE_5.md`; commits `335f143`..`aa2f806`). Recycling is deliberately not supported; see the direction recorded in the project memory.
- **[G11, harness gate]** Canonical comparison, runner, allowlist, `make test-gnu-r`, and the first 23 corpus programs (`STATE_6.md`; commits `37360fc`..`b917973`). Coverage remains open under the circled G11.

## Recorded Decisions

- **Direction (2026-11-14):** R0 is a strict subset that GNU R runs. Canonical GNU R running unchanged in R0 is not a goal, and no recycling mode is planned. The transpiler (G12) is the bridge.
- **Plan 6 (2026-11-14):** The corpus lives in `tests/r0/`. Conformance Rust assertions are not migrated in plan 6. Programs R0 rejects are out of scope. The allowlist is empty.

## Blocked / Pruned

- **Pruned:** a recycling mode in R0 (2026-11-14). Recycling stays explicit; `rep_len()` / `rep()` are the answer.
- **Pruned:** canonical GNU R scripts running unchanged in R0 (2026-11-14). Replaced by the one-way guarantee plus the transpiler.

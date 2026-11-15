# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-14
Source State: STATE_7.md (orchestration 7 gate green at `5c6c829`)

## Circled Goal (Active)

- **[G11]** The `tests/r0/` corpus covers every GNU-compatible R0 program in `tests/conformance.rs`, and each corpus program matches GNU R (`Rscript`) or its difference is listed as an explicit R0 restriction.
  - Why now: this is the bottleneck. G12 (transpiler) cannot be verified until the corpus covers R0-only-adjacent syntax, and G11 wave 2 is the only open step on the path to that coverage. Its source-fix prerequisite (orchestration 7, G17/G18/`na.rm`) is done and green.
  - Exit criteria: `tests/conformance.rs` R0 programs are classified (GNU-compatible, or R0-only and excluded); every GNU-compatible one has a `tests/r0/*.R0` file; `cargo test --test gnu_r` and `make test-gnu-r` exit 0; the errata programs B1–B15 from `docs/LANGUAGE_SPEC.md` §9 are covered where GNU R has the same behaviour.
  - Classification: `planning/tasks/g11-conformance-classification.md` (80 test functions: 32 GNU-compatible, 28 mixed, 15 reject or resource, 4 R0-only pedon script, 1 known gap).
  - Wave 1 done (2026-11-14): 32 class-A corpus programs added (`tests/r0/`). Gate green.
  - Wave 2 scope (orchestration 8): the value asserts from the 28 class-B tests in `planning/tasks/g11-conformance-classification.md`. Error asserts stay in Rust. The source fixes this needs (loop value, list `[<-`, `median`, `atan2`, `na.rm`) are already in `src/vm.rs`.
  - Open prerequisite: `var`/`sd` with NaN. GNU R gives `NA`, R0 gives `NaN` (`planning/STATE_7.md` `## 7.3.1`). Any class-B corpus program that touches them must either wait for a source fix or be classified mixed.
  - First action: `/plan-wave` for wave 2 (orchestration 8).

## Sequenced Goals (Dependency DAG)

- **[G12]** A `.R0 -> .R` transpiler strips R0-only syntax (`fn`, `->`, type hints, and similar) so R0 sources run under GNU R.
  - Depends on: G11 (the corpus must include R0-only input so the transpiler has a test).
- **[G20]** `var` and `sd` with NaN return `NA`, as GNU R does (found during orchestration 7, `planning/STATE_7.md` `## 7.3.1`). Changes `eval_var_sd` in `src/vm.rs`, the matching conformance assert in `conformance_g7_statistical_reductions`, and a spec errata entry.
  - Depends on: none for the code change. Pulls in G11 wave 2 only if a class-B program covers `var` or `sd` NaN.

## Independent Backlog

- **[G13]** `docs/LANGUAGE_SPEC.md` and the `planning/EXECUTION_PLAN_5.md` Decision Log state one rule for logical masks: a short mask errors, matching the strict code behaviour in `src/vm.rs`.
  - Note: spec §3 still allows a length-1 mask, which is a recycle. Reconcile this with the strict rule before closing G13. Not verified in this pass.
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
- **[G17]** Loop value is `NULL` invisibly, as in GNU R (orchestration 7, task 7.1.1, commit `e1a900a`).
- **[G18]** Three R0 results align to GNU R (orchestration 7): list `[<-` with an atomic value stores the first element (7.5.1, `764cd50`); `median` returns `NA` for missing values (7.3.1, `95620f0`); `atan2` with `NULL` traps `TypeError` (7.4.1, `fb34fa6`).
- **[G19, na.rm part]** A non-scalar `na.rm` is an error, not lenient, in every intrinsic that takes `na.rm` (orchestration 7, task 7.2.1, commit `b335df2`). Decided 2026-11-14 with the user, who chose the strict rule because GNU R is inconsistent across intrinsics. Spec errata B15.

## Recorded Decisions

- **Direction (2026-11-14):** R0 is a strict subset that GNU R runs. Canonical GNU R running unchanged in R0 is not a goal, and no recycling mode is planned. The transpiler (G12) is the bridge.
- **Plan 6 (2026-11-14):** The corpus lives in `tests/r0/`. Conformance Rust assertions are not migrated in plan 6. Programs R0 rejects are out of scope. The allowlist is empty.
- **Plan 7 (2026-11-14):** Source fixes (G17, G18, `na.rm`) ran first as orchestration 7; corpus wave 2 is orchestration 8. Haiku workers, serial waves, no worktrees.

## Blocked / Pruned

- **Pruned:** a recycling mode in R0 (2026-11-14). Recycling stays explicit; `rep_len()` / `rep()` are the answer.
- **Pruned:** canonical GNU R scripts running unchanged in R0 (2026-11-14). Replaced by the one-way guarantee plus the transpiler.

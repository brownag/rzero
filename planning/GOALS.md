# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-14
Source State: STATE_5.md

## Circled Goal (Active)

- **[G11]** Every R0 conformance program that R0 accepts produces the same result under GNU R (`Rscript`), or the difference is documented as an explicit R0 restriction.
  - Why now: this is the acceptance test for the one-way guarantee (R0 is a subset that GNU R runs) and the gate the `.R0 -> .R` transpiler must pass. Nothing else in the sequence can be verified without it.
  - Exit criteria: a harness runs each `tests/conformance.rs` R0 program through both R0 and `Rscript` and diffs the output; every mismatch is either fixed or listed in `docs/LANGUAGE_SPEC.md` as a restriction; the harness runs locally with `cargo test` or a `make` target.
  - First action: `/plan-wave` drafts `planning/EXECUTION_PLAN_6.md` for the harness.

## Sequenced Goals (Dependency DAG)

- **[G12]** A `.R0 -> .R` transpiler strips R0-only syntax (`fn`, `->`, type hints, and similar) so R0 sources run under GNU R.
  - Depends on: G11

- **[G13]** `docs/LANGUAGE_SPEC.md` and the `planning/EXECUTION_PLAN_5.md` Decision Log state one rule for logical masks: a short mask errors, matching the strict code behaviour in `src/vm.rs`.
  - Depends on: none (sequenced after G11 so the harness can confirm the rule against GNU R)

## Independent Backlog

- **[G14]** `break`/`next` inside an `if`/`while` condition expression work inside a loop, via a `ConditionFlow` variant in `src/vm.rs`.
- **[G15]** The G9 fuel test (`conformance_g9_fuel_exhaustion_large_frame`) is verified to fire `ErrFuelExhausted` at the subset step, and tightened if it does not.
- **[G16]** `cargo fmt --check` is clean, with formatting changes in their own commit.
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

## Recorded Decisions

- **Direction (2026-11-14):** R0 is a strict subset that GNU R runs. Canonical GNU R running unchanged in R0 is not a goal, and no recycling mode is planned. The transpiler (G12) is the bridge.

## Blocked / Pruned

- **Pruned:** a recycling mode in R0 (2026-11-14). Recycling stays explicit; `rep_len()` / `rep()` are the answer.
- **Pruned:** canonical GNU R scripts running unchanged in R0 (2026-11-14). Replaced by the one-way guarantee plus the transpiler.

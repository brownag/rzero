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

- **[G14]** `break`/`next` inside an `if`/`while` condition expression work inside a loop, via a `ConditionFlow` variant in `src/vm.rs`. The spec still lists this as a current limitation (§3).
  - Depends on: none.
- **[G2]** Unary negation (`-x`) and `abs(x)` on `Int` vectors preserve `NA_INT` and trap with `ErrIntOverflow` on overflow, covered by unit and conformance tests.
  - Orchestration 10 (task 10.1.4) stopped: NA preservation is covered (`unary_pos_and_neg_handle_ranges_and_reject_non_numeric`, `missing_nan_semantics_cover_abs_cumsum_and_membership`), but the overflow trap is unreachable. `NA_INT` is `i32::MIN`, so negation and `abs` map it to NA and every other value fits. Reword to "preserve NA; cannot overflow", then close.
- **[G4]** Read-only paths in `src/value.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` eliminate redundant allocations per `docs/ADVERSARIAL_ORACLE.md` §2.3.
- **[G5]** User-defined function invocation in `src/vm.rs` avoids full `HashMap<String, R0Val>` cloning per call while preserving Axiom 4 value semantics.

## Deferred

- **[G10]** The `R0Vec` 16-byte descriptor (`src/value.rs`) and 63-opcode ISA (`src/isa.rs`) are wired to an SSA compiler and vector-register bytecode engine supporting loop-fused tile kernels (`docs/ARCHITECTURE.md` §§4–8).
  - Trigger: G11 and G12 are complete and verified, and a performance need is measured.

## Completed

- **[G20]** `var`, `sd`, and `atan2` return `NA` where GNU R does (orchestration 10, task 10.1.1, commit 3bdad60). `var`/`sd` with a NaN and `na.rm = FALSE` give `NA`. `atan2` gives `NA` when either operand is `NA`, else `NaN` when either is `NaN`. Spec restriction bullets removed in 10.4.1.
- **[G21]** `diff` returns integer for integer input, and data-frame single-index assignment gives the column the RHS type (orchestration 10, task 10.3.1, commit 14f38a8). Integer overflow in `diff` gives `NA` without a warning (R0 has no warning channel). `diff(c())` remains a restriction.
- **[G13]** One strict logical-mask rule: a mask must match the indexed length, and a length-1 mask is not recycled unless the dimension is 1. Code in `src/vm.rs` (commit 247354f), spec §3 (e93e5ff), restriction recorded (orchestration 10, task 10.2.1).
- **[G15]** `conformance_g9_fuel_exhaustion_large_frame` fires `ErrFuelExhausted` inside the df subset/assign code for all four tails (orchestration 10, task 10.1.3, no edit). The test checks the prefix only.
- **[G3]** `src/lexer.rs` rejects mismatched and unclosed delimiters with `ParseError:` (orchestration 10, task 10.1.2, commit 32ac822).
- **[G16]** `cargo fmt --check` is clean. Formatting is in its own commit ef32fc3 (orchestration 10, task 10.5.1).
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

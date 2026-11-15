# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-15
Source State: orchestration 12 complete (anonymous functions and closures, G24–G34 done). No Circled Goal until new candidates are ranked.

## Circled Goal (Active)

- None. The orchestration 12 round is complete. Next step: supply candidate goals, then run magic-wand to rank them.

## Sequenced Goals (Dependency DAG)

- None open.

## Independent Backlog

- **[G14]** `break`/`next` inside an `if`/`while` condition expression work inside a loop, via a `ConditionFlow` variant in `src/vm.rs`. The spec still lists this as a current limitation (§3).
  - Depends on: none. Unblocks nothing.

## Deferred

- **[G31]** Higher-order base functions (`lapply`, `sapply`, `Map`, `Filter`, `Reduce`, `do.call`) accept function values.
  - Depends on: G27 (complete). Now unblocked.
  - Trigger: a corpus file needs one of these functions.
- **[G10]** The `R0Vec` descriptor and 63-opcode ISA engine (`docs/ARCHITECTURE.md` §§4–8).
  - Depends on: G11 (complete), G12 (complete). G24 is folded into its design; the function value is now implemented on `R0Val`.
  - Trigger: a measured performance need.
- **[G4]** Read-only paths in `src/value.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` eliminate redundant allocations per `docs/ADVERSARIAL_ORACLE.md` §2.3.
  - Trigger: a benchmark shows the allocation cost matters (`performance-benchmarking`).
- **[G5]** User-defined function invocation in `src/vm.rs` avoids full `HashMap<String, R0Val>` cloning per call while preserving Axiom 4 value semantics. Function values are now cloned like other values; that cost is the known O(env) per top-level call.
  - Trigger: a benchmark shows per-call cloning matters.

## Completed

- **[G24]** `R0Val::Fn` function variant (orchestration 12, task 12.1.1, commit `532f9f4`).
- **[G25]** Lexer accepts `\(` as a function introducer (12.1.2, commit `c631c08`).
- **[G26]** Unnamed `fn`, `function`, and `\(...)` evaluate to function values (12.2.1, commit `65c09b5`).
- **[G27]** Calls through any callee expression: variables, list elements, immediate calls, call results (12.3.1 `a21d926`, 12.3.2 `c249a4b`). Bare names of stored functions resolve as values.
- **[G28]** Closure scoping: top-level functions look up free variables live; nested functions snapshot the enclosing frame (12.4.1, commit `644cec0`). Restriction recorded in the spec.
- **[G30]** Transpiler maps `\(...)` to `function(...)` (covered by the 12.1.2 transpiler test, commit `c631c08`).
- **[G32]** Corpus `tests/r0/g_anon_*.R0` (8 files) equal under GNU R and `R0Vm` (12.5.1, commit `6e394e4`). `gnu_r` 4/4 serially.
- **[G33]** `docs/LANGUAGE_SPEC.md` documents the anonymous-function forms, relaxed Axiom 1, and the scoping rule (12.5.2, commit `be70e9f`).
- **[G34]** Function values are not printable: `Display` gives `<function>`; canonical comparison is undefined (12.1.1 and 12.2.2, commits `532f9f4` and `399fefd`). Decided 2026-11-15 as "error", not "print source".
- **[G12]** A `.R0 -> .R` transpiler strips R0-only syntax (`fn`, `->`, type hints) and emits valid GNU R (plan 11, orchestration 11). Commits: `fcf03b2` (`transpile_to_r`), `dc5c585` (`r0_transpiled_matches_gnu_r`), planning records `8d8b23e` and `19bb9ff`. Verified: 7 transpile unit tests pass; `gnu_r` 4/4 serially; wasm build clean. Out of scope and not done: the `--transpile` CLI flag in `src/main.rs`.
- **[G2]** Unary negation (`-x`) and `abs(x)` on `Int` vectors preserve `NA_INT`; they cannot overflow, because `NA_INT` is `i32::MIN` and every other value's negation or absolute value fits. Closed by reword (orchestration 10, task 10.1.4, user decision 2026-11-15). Covered by `unary_pos_and_neg_handle_ranges_and_reject_non_numeric` and `missing_nan_semantics_cover_abs_cumsum_and_membership`. Binary `-` overflow is a separate, already covered behaviour.
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
- **[G11]** Closed in orchestration 9 (`STATE_9.md`). `tests/r0/` has corpus files, `allowlist.txt` has no entries. The GNU-valid pedon twin, the B8–B12 errata files, and the restriction list in `docs/LANGUAGE_SPEC.md` "GNU R equivalence" are in. Exclusions are recorded as restrictions or linked to G20/G21. Earlier waves: harness gate (`STATE_6.md`), corpus waves 1–2 (orchestrations 7–8).
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
- **[G29]** `λ` (UTF-8 lambda) as an alias for `function`.
  - Status: Pruned
  - Reason: user decision 2026-11-15. Low value; not worth the lexer work.

## Decisions

- 2026-11-15: G24–G34 round (anonymous functions and closures) complete in orchestration 12.
- 2026-11-15: G29 (`λ`) pruned. R0 need not be a strict GNU R subset; the transpiler carries equivalence (G33).
- 2026-11-15: G34 decided as "error" (`<function>` placeholder), not "print source".
- 2026-11-15: no Circled Goal until new candidate goals are supplied and ranked.

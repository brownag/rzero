# Project Goals (`planning/GOALS.md`)

Last Updated: 2026-11-15
Source State: STATE_11.md (plan 11 complete: G12 transpiler landed in `fcf03b2`, `dc5c585`); anonymous-function goals added from magic-wand, with user decisions recorded below

## Circled Goal (Active)

- **[G24]** R0 has a first-class function value: `R0Val` can hold a function, so it can be bound, stored in a list, returned, and passed as an argument.
  - Why now: unblocks every anonymous-function goal (G26, G27, G28, G31, G32, G33). The largest downstream fan-out in the graph.
  - Exit criteria: `R0Val` has a function variant; `list(g = fn(x) x + 1)` evaluates without the "must be bound" error; `cargo test` passes; `cargo build --target wasm32-unknown-unknown --release` is clean.
  - Decision (user, 2026-11-15): R0 is no longer required to be a strict GNU R subset. Its contract is that `transpile_to_r` turns every R0 program into GNU R that evaluates identically under `R0Vm`. R0-only syntax (`fn`, `: type`, `-> type`, `\(x)`) is allowed and removed by the transpiler. Relaxing Axiom 1 is part of G33.
  - First action: count and list the `R0Val` match sites with `grep -n "R0Val::" src/*.rs` to size the change, then plan with plan-wave.
  - Decision (user, 2026-11-15): folded into G10's design. G10 is an unimplemented proposal (`docs/ARCHITECTURE.md` §§4–8; `src/isa.rs` is scaffold only), so G10 stays out of this round. The function value is implemented on the current `R0Val`, and the design notes for G10 must account for it.

## Sequenced Goals (Dependency DAG)

- **[G25]** The lexer accepts `\` as a function-introducer token, so `\(x) x + 1` tokenises. `src/lexer.rs` currently rejects it at line 355 with `Unexpected character`.
  - Depends on: none. Can run in parallel with G24 (different files).
- **[G26]** Unnamed `fn(...)`, `function(...)`, and `\(...)` evaluate to function values instead of erroring at `src/vm.rs:396`.
  - Depends on: G24, G25.
- **[G27]** A function value can be called through a variable, a list element, or directly, e.g. `(\(x) x)(2)` and `l$f(2)`.
  - Depends on: G24.
- **[G28]** Closures capture their defining environment, so `function(x) function(y) x + y` works.
  - Depends on: G27.
  - Decision (user, 2026-11-15): in this round, as the logical next step after calls.
  - Risk: `docs/LANGUAGE_SPEC.md` §3.2 keeps function definitions in `self.functions`, separate from `env`. Capture needs a design for that split; this is the highest-risk task in the round.
- **[G30]** The transpiler maps `\(...)` to `function(...)`, so transpiled output is valid GNU R.
  - Depends on: G25. Can be planned with G25 in one wave.
- **[G31]** Higher-order base functions (`lapply`, `sapply`, `Map`, `Filter`, `Reduce`, `do.call`) accept function values.
  - Depends on: G27.
  - Trigger to start: a corpus file needs one of them. Deferred until then.
- **[G32]** Corpus files `tests/r0/g_anon_*.R0` run identically under GNU R (via the transpiler) and `R0Vm`, with rules for function-valued results recorded in §3.2.
  - Depends on: G26, G27, G34.
- **[G33]** `docs/LANGUAGE_SPEC.md` describes every anonymous-function form, its status, and the relaxed Axiom 1. Draft wording for Axiom 1: "Every valid R0 program transpiles (`transpile_to_r`) to valid GNU R that evaluates to the same result under `R0Vm`, within the supported subset, unless it triggers an R0 strictness trap." Section 1's "valid R0 programs use standard R syntax" line changes with it.
  - Depends on: G26, G27. Can start earlier for the Axiom 1 wording alone.
- **[G10]** The `R0Vec` descriptor and 63-opcode ISA engine (`docs/ARCHITECTURE.md` §§4–8).
  - Depends on: G11 (complete), G12 (complete).
  - Trigger: a measured performance need. Stays out of this round; G24 is folded into its design.

## Independent Backlog

- **[G14]** `break`/`next` inside an `if`/`while` condition expression work inside a loop, via a `ConditionFlow` variant in `src/vm.rs`. The spec still lists this as a current limitation (§3).
  - Depends on: none. Unblocks nothing.
- **[G34]** Decide how a function value prints and canonicalises. Options: error, or print the source as GNU R does. Needed before G32 writes expected output.
  - Depends on: G24.

## Deferred

- **[G4]** Read-only paths in `src/value.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs` eliminate redundant allocations per `docs/ADVERSARIAL_ORACLE.md` §2.3.
  - Trigger: a benchmark shows the allocation cost matters (`performance-benchmarking`).
- **[G5]** User-defined function invocation in `src/vm.rs` avoids full `HashMap<String, R0Val>` cloning per call while preserving Axiom 4 value semantics. Interacts with G24: function values get cloned like other values.
  - Trigger: a benchmark shows per-call cloning matters.
- **[G31]** See Sequenced Goals. Trigger: a corpus file needs `lapply`, `sapply`, `Map`, `Filter`, `Reduce`, or `do.call`.

## Completed

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
- **[G29]** `λ` (UTF-8 lambda) as an alias for `function`.
  - Status: Pruned
  - Reason: user decision 2026-11-15. Low value; not worth the lexer work.

## Decisions (2026-11-15)

- G24 folded into G10's design; G10 is out of this round.
- G28 closures are in this round, after G27.
- G29 (`λ`) pruned. R0 need not be a strict GNU R subset; the transpiler carries equivalence (G33).

## Round scope

Nine goals fit one `/plan-wave` run: G24, G25, G26, G27, G28, G30, G32, G33, G34. G31 (higher-order base functions) and G10 stay out. Chain: G24 → G26 → G27 → G28 is serial through `src/vm.rs`; G25 and G30 touch lexer/transpiler and can run alongside it.

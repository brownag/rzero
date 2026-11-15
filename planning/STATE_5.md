# Orchestration 5 State (`planning/STATE_5.md`)

**Goal:** G9 — `break`/`next` and DataFrame/list bracket subsetting (read + write) (`EXECUTION_PLAN_5.md`)
**Branch:** `main`

## Baseline (Pre-Wave 1)
- [x] `cargo test --quiet`: 112 unit, 70 conformance passed (0 failed)
- [x] `cargo build --target wasm32-unknown-unknown --release`: exit 0
- [ ] `cargo clippy --all-targets -- -D warnings`: FAILS (6 `unnecessary_sort_by` in `src/vm.rs`) — fixed by `5.1.1`

## Wave Board

| Wave | Task ID | Title | Status | Commit |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `5.1.1` | Clippy residue + `break`/`next` | done | `335f143`, `87a3061` |
| 2 | `5.2.1` | Bracket extraction `x[i, j]`, `x[j]` | done | `353cd9c` |
| 3 | `5.3.1` | Bracket assignment `x[i, j] <- v`, `x[j] <- v` | done | `31ad5a8` |
| 4 | `5.4.1` | Conformance tests for G9 | done | `aa2f806` |
| 4 | `5.4.2` | Spec update for G9 | done | `886c95a` |

## Task Reports

## 5.1.1
- Status: done.
- Files: src/vm.rs (sort_by_key at order/rank Int/Lgl arms; ControlFlow::Break/Next, outside_loop helper, loops, call_user, extract_scalar_cond, eval_script; 6 new tests at end of mod tests), src/lexer.rs (Tok::Break/Next + keywords), src/ast.rs (Expr::Break/Next), src/parser.rs (prefix arms).
- Commits: 335f143 fix(vm) clippy; 87a3061 feat(vm) break/next.
- Tests: `cargo test --lib` 118 passed, 0 failed (112 + 6 new); `cargo test --lib break_` 6/6; `cargo clippy --all-targets -- -D warnings` exit 0.
- Deviations: none.
- Gotchas: break/next inside an if/while condition expression errors ("used outside a loop") even inside a loop (known gap; extract_scalar_cond has no ConditionFlow variant) -- note in spec / later task. Function escape reuses the same message text. Loop var from `1:n` is Int (affects test expectations). Conformance suite not run (out of scope).

## 5.2.1
- Status: done.
- Files: src/ast.rs (Expr::Index2), src/parser.rs (LBrack arm), src/vm.rs (Assign Index2 error, eval Index2 arm, resolve_index/subset2 before `fn subset`, subset Df/List column arm, 3 new tests at end + null_df_and_list_access updated).
- Commit: see `git log` (feat(vm): add bracket extraction x[i, j] on df and list).
- Tests: `cargo test --lib` 121 passed, 0 failed; `cargo test --lib bracket` 4/4 (3 new + existing); `cargo clippy --all-targets -- -D warnings` exit 0.
- Deviations: none. Existing test `null_df_and_list_access` asserted `d[1L]` TypeError; updated to expect Df.
- Gotchas: resolve_index(idx, n, names) errors on OOB/NA/mixed/unknown (atomic Index1 keeps old NA-on-OOB path). Df/List len() is nrow, ncols = cols.len(). `cargo fmt --check` reports diffs (not run/applied). Index2 assign returns "Index2 assignment not yet supported" at the Assign match.

## 5.3.1
- Status: done.
- Files: src/vm.rs (Assign Index1 df/list branch + Index2 arm ~159-205; helpers recycle/atomic_rhs/assign_cols/assign2 before `fn subset_assign`; 2 new tests at end; `d[, 1] <- 0` assertion in bracket error test updated).
- Commit: see `git log` (feat(vm): add bracket assignment x[i, j] <- v and x[j] <- v on df and list).
- Tests: `cargo test --lib` 123 passed, 0 failed; `cargo clippy --all-targets -- -D warnings` exit 0.
- Deviations: none (Q1 per coordinator: subset_assign over all rows per column, so old column type rules apply).
- Gotchas: parser rejects `d[] <- v` (empty brackets); `list()` args must be named. List target `l[j] <- v` replaces elements (any length); 2D on List errors. Df RHS columns must have length nrow. Length-0/Null RHS errors.

## 5.4.2
- Status: done.
- Files: docs/LANGUAGE_SPEC.md (§2.3 bracket extraction/assignment, §3.3 break/next).
- Commit: 886c95a docs(spec): document break/next and df/list bracket subsetting.
- Tests: `cargo test --quiet` 123 unit + 70 conformance passed, 0 failed.
- Deviations: none.
- Gotchas: spec states break/next-in-condition limitation, `d[] <- v` parser rejection, 2D-on-List error. Pre-existing §4.6 text (B1 out-of-range growth) applies to atomic vectors only; not edited.

## 5.4.1
- Status: done.
- Files: tests/conformance.rs (10 new conformance_g9_* tests + g9 helpers at end).
- Commit: aa2f806 (test(conformance): add G9 break/next, bracket extraction and assignment tests).
- Gate: cargo test 123 lib + 80 conformance passed, 0 failed; clippy --all-targets -D warnings exit 0; wasm32 release build exit 0.
- Deviations: none vs plan; logical row mask shorter than nrow is asserted as ErrDimensionMismatch (src does not recycle, contradicting Decision Log "recycled").
- Gotchas: fuel test uses 1:200000 cols (1:2000000000 df took ~70s before fuel error; likely materialisation); break-in-if-condition test asserts the current error (known gap).

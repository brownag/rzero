# Orchestration 7 State (`planning/STATE_7.md`)

**Goal:** Align R0 with GNU R on loop value, list single-index assign, `median` missing values, `atan2` `NULL` arguments, and non-scalar `na.rm`, as the prerequisite for G11 wave 2 (`EXECUTION_PLAN_7.md`).
**Branch:** `main`
**Start commit:** `0230ac9`

## Baseline (pre-wave-1)
- [x] `cargo test`: 123 lib, 80 conformance, 3 gnu_r passed (HEAD `0230ac9`, recorded in G11 wave 1 gate)
- [x] `make test-gnu-r`: exit 0 (HEAD `0230ac9`)
- [ ] `cargo clippy --all-targets -- -D warnings`: not re-run for this orchestration. Run as the first gate step, record exit code here.
- [ ] `cargo build --target wasm32-unknown-unknown --release`: not re-run. Record exit code and byte size here.
- [ ] `Rscript --version`: R 4.6.2 at `/usr/bin/Rscript` (confirm before 7.3.1)

## Wave Board

| Wave | Task ID | Title | Status | Commit |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `7.1.1` | Loop value returns `NULL` | done | `e1a900a` |
| 2 | `7.2.1` | Non-scalar `na.rm` is an error | done | `b335df2` |
| 3 | `7.3.1` | `median` returns `NA` for missing values | done | `95620f0` |
| 4 | `7.4.1` | `atan2` errors on `NULL` arguments | pending | |
| 5 | `7.5.1` | List single-index assign stores the first element | pending | |
| 6 | `7.6.1` | Spec and errata text | pending | |

## Task Reports

(Workers append here under `## {{id}}`.)

## 7.1.1

- Status: done. For and while arms now return `ControlFlow::Val(R0Val::Null)` at loop end; body values discarded; `break`/`next` unchanged.
- Files changed: `src/vm.rs` L323-339 (For arm), L340-354 (While arm), new test `loop_value_is_null` inserted before `break_next_affect_innermost_loop_only` (~L5428).
- Code commit: `e1a900a` `fix(vm): loops return NULL invisibly as in GNU R`. TODO commit: `a56584d`.
- Tests:
  - `cargo test --lib loop_value_is_null`: 1 passed, 0 failed.
  - `cargo test --lib loop`: 4 passed, 0 failed.
  - `cargo test --test conformance`: 80 passed, 0 failed.
  - `cargo test --test gnu_r`: 3 passed, 0 failed.
- Deviations: none. Existing loop-final tests checked; none asserted a loop value, so none changed.
- Gotchas for dependents: loops now always yield NULL, so any R0 code relying on a loop's last body value will change. Corpus has no such case.

## 7.2.1

- Status: done. `parse_na_rm` rejects any logical `na.rm` with length != 1 as `ErrInvalidCondition: na.rm in {} requires a length-1 logical value`; length-1 TRUE/FALSE, NA, and non-logical branches unchanged.
- Files changed: `src/vm.rs` L1064-1081 (parse_na_rm Lgl arm), L3977-3978 (na_rm_validation_rules: two vector assertions flipped to error); `tests/conformance.rs` L463 (na_rm_validation vector assertion flipped to error).
- Code commit: `b335df2` `fix(vm): reject non-scalar na.rm as GNU R does`. TODO/STATE commit: separate `docs(planning)` commit.
- Tests:
  - `cargo test --lib na_rm`: 1 passed, 0 failed (before and after).
  - `cargo test --test conformance na_rm`: 1 passed, 0 failed (before and after).
- Deviations: none.
- Gotchas for dependents: `na.rm = c(...)` of any length other than 1 now errors for all of sum/mean/min/max/any/all/prod/var/sd/median/pmin/pmax. Empty `na.rm` now reports the length message, not the NA message (same prefix). G19 in planning/GOALS.md and g11 D4 still describe the old lenient behaviour; update handled outside this task.

## 7.3.1
- Status: done. `eval_median` returns `NA_real_` (not the NaN payload) when `na.rm = FALSE` and a missing value is present.
- Files changed: `src/vm.rs` L2076-2095 (eval_median, missing-value return), L4872-4882 (statistical_reductions_prod_var_sd_median: NaN assertion flipped to NA, two new assertions); `tests/conformance.rs` L1258-1260 (median NaN assertion flipped to `is_na_real`).
- Code commit: `95620f0` `fix(vm): median returns NA for missing values as GNU R does`. TODO/STATE commit: separate `docs(planning)` commit.
- Tests:
  - `cargo test --lib median`: 1 passed, 0 failed.
  - `cargo test --test conformance g7_statistical`: 1 passed, 0 failed.
- Rscript outputs: `median(c(1, NaN, 3))` -> `[1] NA`; `var(c(1, NaN, 3))` -> `[1] NA`; `sd(c(1, NaN, 3))` -> `[1] NA`.
- Deviations: existing unit assertion in `statistical_reductions_prod_var_sd_median` asserted the old NaN behaviour, so it was flipped to NA (necessary for the test to pass).
- OPEN ITEM for orchestrator: GNU R gives NA for `var` and `sd` with NaN; R0 does not match. Out of scope for 7.3.1 per coordinator decision. Needs its own task.
- Gotchas for dependents: `median` with NaN and na.rm=FALSE now yields NA_real_, so `is_na_real` (not `is_nan`) is the correct check.

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
| 2 | `7.2.1` | Non-scalar `na.rm` is an error | pending | |
| 3 | `7.3.1` | `median` returns `NA` for missing values | pending | |
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

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
| 1 | `7.1.1` | Loop value returns `NULL` | pending | |
| 2 | `7.2.1` | Non-scalar `na.rm` is an error | pending | |
| 3 | `7.3.1` | `median` returns `NA` for missing values | pending | |
| 4 | `7.4.1` | `atan2` errors on `NULL` arguments | pending | |
| 5 | `7.5.1` | List single-index assign stores the first element | pending | |
| 6 | `7.6.1` | Spec and errata text | pending | |

## Task Reports

(Workers append here under `## {{id}}`.)

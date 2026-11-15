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
| 1 | `5.1.1` | Clippy residue + `break`/`next` | pending | — |
| 2 | `5.2.1` | Bracket extraction `x[i, j]`, `x[j]` | pending | — |
| 3 | `5.3.1` | Bracket assignment `x[i, j] <- v`, `x[j] <- v` | pending | — |
| 4 | `5.4.1` | Conformance tests for G9 | pending | — |
| 4 | `5.4.2` | Spec update for G9 | pending | — |

## Task Reports

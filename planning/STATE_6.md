# Orchestration 6 State (`planning/STATE_6.md`)

**Goal:** G11 — R0 programs that R0 accepts match GNU R under the canonical comparison, or the difference is an allowlisted, documented restriction (`EXECUTION_PLAN_6.md`)
**Branch:** `main`

## Baseline (Pre-Wave 1)
- [x] `cargo test`: 123 lib, 80 conformance passed (0 failed) — HEAD `aa2f806`
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0
- [x] `cargo build --target wasm32-unknown-unknown --release`: exit 0
- [x] `Rscript --version`: R 4.6.2 at `/usr/bin/Rscript`
- [ ] `cargo fmt --check`: already reports differences (out of scope; tracked as G16)
- [ ] `make test-gnu-r`: does not exist yet (task `6.2.3`)

## Wave Board

| Wave | Task ID | Title | Status | Commit |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `6.1.1` | Canonical form on both sides, plus self-test | pending | — |
| 2 | `6.1.2` | Runner, allowlist, and skip rule | pending | — |
| 3 | `6.2.1` | Corpus A: scalars, NA, broadcasting, indexing, intrinsics | pending | — |
| 3 | `6.2.2` | Corpus B: data frames, lists, loops | pending | — |
| 3 | `6.2.3` | Makefile target `test-gnu-r` | pending | — |
| 4 | `6.3.1` | Triage: allowlist or STOP; spec section | pending | — |

## Task Reports

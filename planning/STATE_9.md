# STATE_9 -- Close G11 corpus coverage

Plan: `planning/EXECUTION_PLAN_9.md` (approved 2026-11-15). Goal: `planning/GOALS.md` circled goal G11.

## Baseline (from STATE_8 gate, 2026-11-14)

- [x] `cargo test`: 125 lib, 80 conformance, 3 gnu_r (exit 0)
- [x] `cargo clippy --all-targets -- -D warnings` (exit 0)
- [x] `cargo build --target wasm32-unknown-unknown --release` (exit 0)
- [x] `make test-gnu-r` (exit 0)
- [x] `tests/r0/`: 82 `.R0` files; `allowlist.txt` empty
- [x] Rscript 4.6.2 on PATH
- [ ] Baseline re-run at start of orchestration 9 (orchestrator, before wave 1)

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 9.1.1 Pedon GNU-valid twin | pending | |
| 1 | 9.1.2 Errata audit and coverage | pending | |
| 1 | 9.1.3 Plan-doc corrections | pending | |
| 2 | 9.2.1 Restrictions in the "GNU R equivalence" section | pending | |
| Gate | Full suites, GOALS.md update | pending | |

## Task reports

(workers append `## <id>` sections here)

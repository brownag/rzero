# STATE_10 -- Quick wins from GOALS.md (G20, G21, G13, G16, G15, G3, G2)

Plan: `planning/EXECUTION_PLAN_10.md` (DRAFT, awaiting approval 2026-11-15). Goals: `planning/GOALS.md`.

## Baseline (from STATE_9 gate, 2026-11-15)

- [x] `cargo test`: 125 lib, 80 conformance, 3 gnu_r (exit 0)
- [x] `cargo clippy --all-targets -- -D warnings` (clean)
- [x] `cargo build --target wasm32-unknown-unknown --release` (finished)
- [x] `make test-gnu-r` (3 passed)
- [x] `tests/r0/`: 88 `.R0` files; `allowlist.txt` has no entries
- [x] `git diff --stat 49a2baa -- src/`: empty at orchestration 9 gate
- [x] `cargo fmt --check`: fails (about 300 hunks across 10 files). Expected; fixed by 10.5.1.
- [ ] Baseline re-run at start of orchestration 10 (orchestrator, before wave 1)

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 10.1.1 G20 var/sd/atan2 NaN -> NA | pending | |
| 1 | 10.1.2 G3 lexer delimiter validation | pending | |
| 1 | 10.1.3 G15 fuel test verify (read-only) | pending | |
| 1 | 10.1.4 G2 negation/abs verify (read-only) | pending | |
| 2 | 10.2.1 G13 strict logical masks | pending | |
| 3 | 10.3.1 G21 integer type for diff and df assignment | pending | |
| 4 | 10.4.1 Spec restriction list | pending | |
| 5 | 10.5.1 G16 cargo fmt | pending | |
| Gate | Full suites, GOALS.md update | pending | |

## Task reports

(workers' reports are appended here as `## <id>` sections)

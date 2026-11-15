# STATE_9 -- Close G11 corpus coverage

Plan: `planning/EXECUTION_PLAN_9.md` (approved 2026-11-15). Goal: `planning/GOALS.md` circled goal G11.

## Baseline (from STATE_8 gate, 2026-11-14)

- [x] `cargo test`: 125 lib, 80 conformance, 3 gnu_r (exit 0)
- [x] `cargo clippy --all-targets -- -D warnings` (exit 0)
- [x] `cargo build --target wasm32-unknown-unknown --release` (exit 0)
- [x] `make test-gnu-r` (exit 0)
- [x] `tests/r0/`: 82 `.R0` files; `allowlist.txt` empty
- [x] Rscript 4.6.2 on PATH
- [x] Baseline re-run at start of orchestration 9 (orchestrator, before wave 1): `cargo test --test gnu_r` 3 passed; `make test-gnu-r` 3 passed; `allowlist.txt` has comment lines and no entries.

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 9.1.1 Pedon GNU-valid twin | done | `e844316` (code), `f98928b` (plan) |
| 1 | 9.1.2 Errata audit and coverage | done | `1e429e7` (code), `ac6932c` (plan and results) |
| 1 | 9.1.3 Plan-doc corrections | done | `08d4dbe` (docs), `b2163d0` (plan) |
| 2 | 9.2.1 Restrictions in the "GNU R equivalence" section | done | `8a7194e` (docs), `fb9382a` (plan) |
| Gate | Full suites, GOALS.md update | done | `GOALS.md` and this file (see Gate) |

## Task reports

## 9.1.1
- Status: done. Added `tests/r0/g11_pedon_twin.R0` (27 lines); header changed to plain `function(hz, z_min = 0, z_max = 100)`, body unchanged.
- Commits: `e844316`, `f98928b`.
- Tests: `cargo test --test gnu_r` 3 passed; `make test-gnu-r` 3 passed.
- Deviations: none.
- Gotcha: the harness silently skips a program R0 rejects (prints `reject:`, then passes). An orchestrator run with `--nocapture` showed no `reject:` lines, so the pedon file is accepted and compared.

## 9.1.2
- Status: done. Created `tests/r0/spec_errata_b8.R0` through `b12.R0`, each a `list(...)` of value asserts (b8 arithmetic NA/NaN order; b9 pmin/pmax; b10 min/max/sum/mean; b11 `for`/`while` value; b12 named-list and multi-slot assignment).
- Commits: `1e429e7` (code), `ac6932c` (plan and results).
- Audit: B1, B3, B4, B5, B6 covered by existing files; B8, B9, B10, B11, B12 needs-file, now created; B13 and B14 covered by `g7_statistical_reductions` and `g7_atan2`; B2 restriction (parse error in R0); B15 restriction (R0 traps); B7: no B7 in ledger.
- Restriction findings from R0 vs GNU R: `pmax(NA_real_, NaN, na.rm = TRUE)` (GNU `NaN`, R0 `NA`); `mean(c(NaN, NA_real_))` (GNU `NA`, R0 `NaN`). Excluded from the corpus.
- Deviation: the worker did not commit the corpus files; the orchestrator verified and committed them.
- Correction found at the gate: the audit's B15 row said GNU R accepts a length-2 `na.rm`. It does not; `sum(c(1,2), na.rm = c(TRUE,FALSE))` errors in GNU R too. B15 stays listed as an R0 trap, out of harness scope.

## 9.1.3
- Status: done. Added the 8.4.3 correction addendum in `EXECUTION_PLAN_8.md`, and an "Orchestration 8 exclusions" section listing nine value-exclusion items (plus three error-kind rejects), not seven as the spec said. Linked atan2 and the integer-type rows in `g11-conformance-classification.md` to G20 and G21.
- Commits: `08d4dbe`, `b2163d0` (plan).
- Flag, not edited: `planning/GOALS.md` said "the four exclusions", while the list had more items. That wording was replaced at the gate.

## 9.2.1
- Status: done. Replaced the placeholder with a verified restrictions list in `docs/LANGUAGE_SPEC.md` "GNU R equivalence" (9 bullets). §9 text unchanged.
- Commits: `8a7194e` (docs), `fb9382a` (plan).
- Tests: `cargo test --test gnu_r` 3 passed.
- Deviations: the `1e3L` claim follows the verified behaviour (GNU R: integer `1000L`, no warning), not the task text. The `approx` bullet says R0 "accepts a 4th positional argument" without naming the binding.

## Gate (orchestrator)
- `cargo test`: 125 lib, 0 doc, 80 conformance, 3 gnu_r: pass.
- `cargo test --test gnu_r`: 3 passed. `make test-gnu-r`: 3 passed.
- `cargo clippy --all-targets -- -D warnings`: clean.
- `cargo build --target wasm32-unknown-unknown --release`: finished.
- `git diff --stat 49a2baa -- src/`: empty. `allowlist.txt`: no entries.
- Corpus: 88 `.R0` files (82 + 6).
- `planning/GOALS.md`: G11 moved to Completed; G12 is the circled goal.

## Open findings for the user
- `tests/gnu_r.rs` (`r0_corpus_matches_gnu_r`) silently skips any corpus file R0 rejects. Because it passes in that case, it can hide a broken corpus file. Fixing it means editing `tests/gnu_r.rs`, which this orchestration did not allow. Suggest a new goal.
- The 9.1.2 B15 audit row was wrong (corrected above). The spec §9 B15 entry may need the same check.

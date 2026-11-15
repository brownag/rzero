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
- [x] Baseline re-run at start of orchestration 10 (2026-11-15): `cargo test` 125 lib, 80 conformance, 3 gnu_r (exit 0); Rscript 4.6.2 present

## Wave board

| Wave | Task | Status | Commit(s) |
| --- | --- | --- | --- |
| 1 | 10.1.1 G20 var/sd/atan2 NaN -> NA | done | 3bdad60 (code), c2702c2 (planning) |
| 1 | 10.1.2 G3 lexer delimiter validation | done | 32ac822 (code + planning, see below) |
| 1 | 10.1.3 G15 fuel test verify (read-only) | done (verified, no edit) | none |
| 1 | 10.1.4 G2 negation/abs verify (read-only) | STOPPED: claim partly wrong | none |
| 2 | 10.2.1 G13 strict logical masks | pending | |
| 3 | 10.3.1 G21 integer type for diff and df assignment | pending | |
| 4 | 10.4.1 Spec restriction list | pending | |
| 5 | 10.5.1 G16 cargo fmt | pending | |
| Gate | Full suites, GOALS.md update | pending | |

## Task reports

## 10.1.1
- Status: done. Rscript results (scratch `$HOME/scratch_r10/g20.R`): all `var`/`sd` NaN cases = NA; `atan2(NaN,NA_real_)` = NA; `atan2(NA_real_,NaN)` = NA; `atan2(NaN,1)` and `atan2(1,NaN)` = NaN.
- Files: `src/vm.rs` (`eval_var_sd` ~2064 returns `na_real()`; `"atan2"` arm ~2960 adds NA-first precedence), `tests/conformance.rs` (atan2 ~1150, var ~1201, sd ~1232), `src/vm.rs` unit tests (~4786, ~4846, ~4865).
- Commit: 3bdad60 `fix(vm): return NA from var, sd, and atan2 when GNU R does`; micro-plan c2702c2.
- Tests: `cargo test --lib` 125 pass; `cargo test --test conformance` 80 pass; `cargo test --test gnu_r` 3 pass; `make test-gnu-r` 3 pass.
- Deviations: none. Spec restriction bullets left for 10.4.1.

## 10.1.2
- Status: done. Stack pops now checked: `)` accepts Paren/Header, `]` accepts Brack; end of input with open delimiter errors.
- Files: `src/lexer.rs` (`')'` and `']'` arms, end of `tokenize`, two new tests).
- Commit: 32ac822 `fix(lexer): reject mismatched and unclosed delimiters`; micro-plan in the same planning commit.
- Tests: `cargo test --lib` 127 pass (125 + 2 new); conformance 80 pass; gnu_r 3 pass; `make test-gnu-r` 3 pass.
- Gotcha: `x[[1]` now errors (it was accepted). Intended.

## 10.1.3 (G15, read-only)
- Status: done, no edits, no commit. `conformance_g9_fuel_exhaustion_large_frame` passes.
- Evidence (scratch crate `$HOME/scratch_r10/fuelcheck`, calls `R0Vm::eval_script`): `data.frame(...)` alone needs 16 fuel; a 100,000 budget fails for all four tails with `ErrFuelExhausted`.
- Minimum fuel found by binary search: `d[1:1000, ]` 404,026; `d[, 1]` 600,021.
- Consuming sites: `d[, 1]` and `d[1:1000, ]` in the df read path (`subset2` ~728, `resolve_index` ~717); `d[1] <- 0` in `subset_assign` ~943; `d[, 1] <- 0` in `assign_cols` ~901 (`sel.len()*nrow`).
- Not isolated: the exact line that charges about 400k for the read tails. The claim holds (subset/assign code), but the test checks only the prefix.

## 10.1.4 (G2, read-only): STOPPED
- Status: stopped on a wrong claim. Goal text (`planning/GOALS.md` G2): "trap with `ErrIntOverflow` on overflow".
- Finding: NA preservation is true and covered: `vm.rs` `unary_pos_and_neg_handle_ranges_and_reject_non_numeric` (~3967) and `missing_nan_semantics_cover_abs_cumsum_and_membership` (~3768). The overflow trap is unreachable: `NA_INT` is `i32::MIN`, so `-x` and `abs(x)` map it to NA and every other value fits. Probe: `-NA_integer_` = NA; `abs(NA_integer_)` = NA; `-(-2147483647L)` = 2147483647; `-2147483647L - 1L` = ErrIntOverflow (binary subtraction, not negation).
- Action taken: none. Per the plan, no code or test added. G2 stays open. Decision for user: reword G2 to "preserve NA; cannot overflow because NA_INT is i32::MIN", then close it.

## Wave 2 gate
- Dependencies (10.1.1) done. 10.1.4 STOP does not block 10.2.1; continuing.

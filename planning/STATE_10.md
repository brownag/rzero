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
| 2 | 10.2.1 G13 strict logical masks | done | 247354f (code), e93e5ff (spec), 4d8c03d (planning) |
| 3 | 10.3.1 G21 integer type for diff and df assignment | done | 14f38a8 (code), 8ca8200 (planning) |
| 4 | 10.4.1 Spec restriction list | done | (see below) |
| 5 | 10.5.1 G16 cargo fmt | done | ef32fc3 |
| Gate | Full suites, GOALS.md update | done (G2 still open) | GOALS.md update in planning commit below |

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

## 10.2.1 (G13)
- Status: done. Length-1 masks no longer recycle; `mask.len() != n` is the only length check at the three sites. Index expression is `mask[i]`.
- Corpus: `make test-gnu-r` passes (no `.R0` file relies on length-1 recycling).
- Files: `src/vm.rs` (3 mask sites, unit test `logical_mask_length_and_na` extended), `docs/LANGUAGE_SPEC.md` lines 53, 146, and 160.
- Deviation: spec line 160 (slot selection) also said "`x.len()` or `1`". Updated so §3 states one rule. Not in the plan's list.
- Tests: lib 127, conformance 80, gnu_r 3, `make test-gnu-r` exit 0.

## 10.3.1 (G21)
- Status: done. `diff` returns `Int` for `Int`/`Range` input (NA propagates; overflow gives `NA_INT`, no warning). `assign_cols` (df branch) sets the column to the RHS (`cols[s] = rhs`) instead of coercing to the old column type.
- Rscript check of the rule: `g21.R`, `g21b.R`, `g21c.R`, `g21d.R` in scratch. `d[c(TRUE,FALSE)] <- 2L` gives integer; `d[-1] <- 8` gives double; `d[1] <- "x"` gives character.
- Files: `src/vm.rs` (`"diff"` arm, `assign_cols`, unit tests `diff_lag_semantics` and `bracket_assign_columns_and_lists`), `tests/conformance.rs` (`diff` block, `conformance_g9_assign_1index`).
- Deviations from the plan: `d[1] <- "x"` changed from `TypeError` to a character column (GNU R-verified); the unit assert was updated. Overflow: NA (plan default) is recorded in the task file's Questions section.
- Not fixed (out of scope): `d[2] <- NA` on a one-column df errors `ErrBoundsTrap`; GNU R extends the df.
- Tests: lib 127, conformance 80, gnu_r 3, `make test-gnu-r` exit 0.

## 10.4.1 (spec restrictions)
- Status: done. Removed the `atan2`, `var`/`sd` (G20) bullets and the integer-type bullet. Kept `diff(c())` as a restriction (GNU R `integer(0)`, R0 `double(0)`; G21 does not cover `c()`). Added a G13 bullet (length-1 masks: GNU R recycles, R0 errors).
- Verified by `Rscript` in 10.2.1 and 10.3.1. File: `docs/LANGUAGE_SPEC.md` "Restrictions" only. Commit: `docs(spec): update GNU R restrictions for G20, G21, and G13`.

## 10.5.1 (G16)
- Status: done. `cargo fmt` run once (a second run was needed to reach `--check` clean; the first run left `tests/gnu_r.rs` unformatted). `cargo fmt --check` exits 0. Only `.rs` files changed.
- Counts unchanged: lib 127, conformance 80, gnu_r 3. Commit: ef32fc3 `style: apply cargo fmt across crate`.

## Gate (orchestrator)
- [x] `cargo test`: 127 lib, 80 conformance, 3 gnu_r, 0 failures.
- [x] `make test-gnu-r` exit 0.
- [x] `cargo clippy --all-targets -- -D warnings` exit 0.
- [x] `cargo build --target wasm32-unknown-unknown --release` exit 0.
- [x] `cargo fmt --check` exit 0.
- [x] `git diff --stat 49a2baa -- src/`: 8 files (formatting touches all of them; logic changes in `vm.rs` and `lexer.rs`).
- [x] `tests/r0/allowlist.txt`: no entries.
- [x] GOALS.md: G20, G21, G13, G15, G16, G3 moved to Completed. G2 stays in the Backlog with the finding.
- Open: G2 (goal text is wrong; decision for user). `diff` overflow warning not modelled. `d[2] <- NA` out-of-range on a df.

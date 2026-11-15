# EXECUTION_PLAN_10 -- Quick wins from GOALS.md (G20, G21, G13, G16, G15, G3, G2)

Status: EXECUTED 2026-11-15 under `/wave-orchestration run plan 10` (approval given by that command). Gate result and open items: `planning/STATE_10.md`. G2 stopped (see STATE_10 10.1.4).

## Goal & Scope

**Goal.** Close the independent quick-win goals in `planning/GOALS.md` that do not depend on G12: NaN handling for `var`/`sd`/`atan2` (G20), integer type for `diff` and data-frame single-index assignment (G21), a strict logical-mask rule (G13), lexer delimiter validation (G3), a verified fuel test (G15), a verified integer-negation/abs check (G2), and a crate-wide `cargo fmt` pass (G16). Each goal ends in a closed, verified state, and the spec's GNU R restriction list shrinks where a goal lifts a restriction.

**In scope.**
- G20: `var`, `sd`, `atan2` with NaN return `NA` where GNU R does (`src/vm.rs`).
- G21: `diff` and data-frame single-index assignment keep integer type where GNU R does (`src/vm.rs`).
- G13: one logical-mask rule across the code and `docs/LANGUAGE_SPEC.md` §3 (`src/vm.rs`, spec).
- G3: lexer validates matching open and close delimiters (`src/lexer.rs`).
- G15: verify the G9 fuel test fires `ErrFuelExhausted` at the subset step (read-only unless it fails).
- G2: verify that integer negation and `abs` preserve `NA_INT` and trap on overflow (read-only unless it fails).
- G16: `cargo fmt --check` is clean, formatting in its own commit (crate-wide, last).
- Spec restriction list (`docs/LANGUAGE_SPEC.md` "GNU R equivalence", Restrictions) updated to match the goals that landed.

**Out of scope.**
- G12 transpiler (the user plans it separately), G14 (`break` in a condition), G4 and G5 (performance), G10 (deferred).
- Any `tests/r0/allowlist.txt` edit.
- Pushing or any remote action.
- Rewriting history in `EXECUTION_PLAN_5.md` or any earlier plan.
- Edits to §9 errata text.

**Verbatim constraints.**
- "Nothing is pushed." (repo rule, `planning/EXECUTION_PLAN_9.md`)
- G13 (`planning/GOALS.md`): "a short mask errors, matching the strict code behaviour in `src/vm.rs`." Recon found the premise is inverted (see decision 3).
- G16 (`planning/GOALS.md`): "`cargo fmt --check` is clean, with formatting changes in their own commit."

## Decision log

- 2026-11-15: Planning artifacts are **tracked**. Plan and state commits are `docs(planning):`, separate from code (same as orchestration 9).
- 2026-11-15: Quick-win set is G20, G21, G13, G16, G15, G3, G2. G12, G14, G4, G5, and G10 are excluded (see Out of scope).
- 2026-11-15 (default, needs approval): **G13 strict masks.** A logical mask must have length equal to the dimension. A length-1 mask errors unless the dimension is 1. Reason: the project direction is "no recycling" (`planning/GOALS.md` G9 note, project memory). This changes behaviour in `src/vm.rs` at the three mask sites (dataframe rows ~666, vector index ~767, assignment ~989). It is not a docs-only change. `EXECUTION_PLAN_5.md` line 27 ("recycled to dim") is historical and is not rewritten. The divergence from GNU R is recorded as a restriction in 10.4.1.
- 2026-11-15: **G20 NaN rule.** Each NaN and NA result follows the GNU R value shown by `Rscript`, verified before any code change. Known so far: `var(c(1, NaN, 3))` gives `NA` in GNU R. The worker verifies `sd`, `atan2(NaN, NA_real_)`, and the NA-vs-NaN precedence. The §9 errata text is not edited.
- 2026-11-15: **G21 integer rule.** Result types follow GNU R as verified by `Rscript`. `diff` returns integer for integer input. Data-frame assignment keeps the integer type the verified GNU R result shows. The worker confirms the exact column-type rule before editing.
- 2026-11-15: **G3 lexer.** A closer must match the open delimiter on top of the stack (`)` closes `(`, `]` closes `[`, `]]` closes `[[`). A mismatched or unmatched closer, or an opener still open at end of input, fails with a deterministic `ParseError:` message. Scope is `src/lexer.rs` only.
- 2026-11-15: **G15 and G2 are verify-only.** If the check passes, the task closes the goal with evidence and no code edit. If it fails, the worker stops and reports. No ad hoc fix and no scope widening (escalation rule 4).
- 2026-11-15: **G16 is last.** `cargo fmt` touches about 300 hunks across nearly every source and test file. It runs after all other edits to avoid conflicts. Workers must not run `cargo fmt` themselves.
- 2026-11-15: **Commit types.** Code `fix(vm): ...` / `fix(lexer): ...`, spec `docs(spec): ...`, format `style: ...`, planning `docs(planning): ...`. No phase or wave numbers in subjects.
- 2026-11-15: **Never** add `src/` edits outside the tasks named here. A corpus file that disagrees with GNU R stops the worker (orchestration 9 rule).

## Recon facts

- `src/vm.rs` is 5617 lines. `src/lexer.rs` is 431. `tests/conformance.rs` is 2020. `tests/gnu_r.rs` is 385.
- No ADRs in `docs/decisions/` (directory absent). Nothing supersedes.
- `cargo fmt --check` fails across: `ast.rs` 3 hunks, `isa.rs` 3, `lexer.rs` 8, `lib.rs` 2, `main.rs` 2, `parser.rs` 10, `value.rs` 9, `vm.rs` 204, `tests/conformance.rs` 61, `tests/gnu_r.rs` 8.
- `var`/`sd`: `eval_var_sd` at `src/vm.rs:2052`. A NaN with `na.rm = FALSE` returns the payload unchanged (`Dbl(vec![v])`), not `NA`. Unit tests around `src/vm.rs:4838` (var) and `:5012` (prod/var/sd loop).
- `atan2`: arm at `src/vm.rs:2947`. Missing precedence: `y` missing first, then `x`. Unit test `atan2_quadrants_recycling_and_na` at `src/vm.rs:4701`. Corpus: `tests/r0/g7_atan2.R0`.
- `diff`: arm at `src/vm.rs:3083`; output is always `R0Val::Dbl` (line ~3135). Unit test `diff_lag_semantics` at `src/vm.rs:5278`, asserting `Dbl` at 5291.
- Logical masks: `src/vm.rs:666-677` (df rows), `767-777` (vector index), `989-1000` (assignment). Each has `mask.len() != 1` recycling.
- Lexer: `src/lexer.rs:295` (`]]` checks top is `Brack2`), `:320-336` (`)` and `]` pop without checking kind). `Open` enum has `Paren`, `Header`, `Brack`, `Brack2`.
- Fuel test: `conformance_g9_fuel_exhaustion_large_frame` at `tests/conformance.rs:2014`. It asserts only the `ErrFuelExhausted:` prefix, not the step.
- Spec: §3 mask text at `docs/LANGUAGE_SPEC.md` lines ~53 and ~146; "GNU R equivalence" Restrictions at ~335-346.
- Baseline (orchestration 9 gate, 2026-11-15): `cargo test` 125 lib, 80 conformance, 3 gnu_r; `make test-gnu-r` 3 passed; clippy `-D warnings` clean; WASM release build finished; corpus 88 files.

## Waves

| Wave | Task | Files (must edit / can create) | Depends on |
| --- | --- | --- | --- |
| 1 | 10.1.1 G20 var/sd/atan2 NaN -> NA | must edit `src/vm.rs`, `tests/conformance.rs` | none |
| 1 | 10.1.2 G3 lexer delimiter validation | must edit `src/lexer.rs` | none |
| 1 | 10.1.3 G15 fuel test verify (read-only) | none | none |
| 1 | 10.1.4 G2 negation/abs verify (read-only) | none | none |
| 2 | 10.2.1 G13 strict logical masks | must edit `src/vm.rs`, `docs/LANGUAGE_SPEC.md` §3 only | 10.1.1 (same file, serialized) |
| 3 | 10.3.1 G21 integer type for diff and df assignment | must edit `src/vm.rs`, `tests/conformance.rs` | 10.2.1 (same file, serialized) |
| 4 | 10.4.1 Spec restriction list | must edit `docs/LANGUAGE_SPEC.md` "GNU R equivalence" Restrictions only | 10.1.1, 10.2.1, 10.3.1 |
| 5 | 10.5.1 G16 cargo fmt | must edit every `src/*.rs` and `tests/*.rs` file (formatting only) | 10.1.2, 10.2.1, 10.3.1 |
| Gate | orchestrator (no worker) | runs full suites; updates `GOALS.md` and `STATE_10.md` | all |

Wave 1 file scopes are disjoint. The two read-only tasks edit nothing. `src/vm.rs` is touched only by 10.1.1, 10.2.1, and 10.3.1, which run in separate waves.

---

## Task 10.1.1 -- G20: var, sd, atan2 with NaN return NA where GNU R does

- **Repo:** rzero
- **Files in scope:** must edit: `src/vm.rs`, `tests/conformance.rs`. Can create: none.
- **Depends on:** none.
- **Spec:**
  1. Analysis first (no edits): run `Rscript` on each candidate and record its result. Candidates: `var(c(1, NaN, 3))`, `var(c(NaN, NA_real_))`, `var(c(NA_real_, NaN))`, `sd(c(1, NaN, 3))`, `sd(c(NaN, 1))`, `atan2(NaN, NA_real_)`, `atan2(NA_real_, NaN)`, `atan2(NaN, 1)`, `atan2(1, NaN)`. Record each result in the micro-plan.
  2. In `eval_var_sd` (`src/vm.rs:2052`), when `na_rm` is false and a missing value is found, return `NA` (`na_real()`) wherever the GNU R result is `NA`. Keep the early-return structure.
  3. In the `atan2` arm (`src/vm.rs:2947`), change the missing-value precedence so each pair matches the verified GNU R result (`atan2(NaN, NA_real_)` should be `NA`). Do not change non-missing results.
  4. Update the unit assertions that encode the old NaN result (around `src/vm.rs:4838` and `:4701`) and the conformance asserts for var/sd/atan2 NaN (grep `conformance_g7_statistical_reductions` and the atan2 test in `tests/conformance.rs`).
  5. Do not edit `tests/r0/` corpus files, the spec, or §9.
- **Anchors:** `src/vm.rs` `fn eval_var_sd` (~2052), `"atan2" =>` (~2947), `fn atan2_quadrants_recycling_and_na` (~4701), `"var" | "sd"` arm (~2994); `tests/conformance.rs` `fn conformance_g7_statistical_reductions`, grep `atan2`.
- **Constraints:** no changes outside the two files. Do not run `cargo fmt`. Keep the existing error paths.
- **Reference:** the NA-vs-NaN precedence comment style already used in the `atan2` arm and `eval_median`.
- **Acceptance:** `cargo test --lib`, `cargo test --test conformance`, and `cargo test --test gnu_r` exit 0. `make test-gnu-r` exits 0. The `g7_atan2.R0` corpus file still matches GNU R.

## Task 10.1.2 -- G3: lexer validates matching open and close delimiters

- **Repo:** rzero
- **Files in scope:** must edit: `src/lexer.rs`. Can create: none.
- **Depends on:** none.
- **Spec:**
  1. In the `')'` arm (`src/lexer.rs:~320-325`) and the `']'` arm (`~331-336`), pop the stack and check the popped kind. `)` must pop `Open::Paren` or `Open::Header`. `]` must pop `Open::Brack`. `]]` (`~295`) already checks `Brack2`.
  2. A mismatch or an empty stack returns `Err("ParseError: unmatched ')'" )` or `Err("ParseError: unmatched ']'")` (match the existing `ParseError:` wording used in the file; grep for it).
  3. At end of input, if the stack is non-empty, return `Err("ParseError: unclosed '<delim>'")`. Before adding this, confirm with the existing tests that no valid program leaves the stack non-empty. Newline suppression may rely on the stack; check it.
  4. Keep the `header` logic for `if`/`while`/`for`/`fn` unchanged.
- **Anchors:** `src/lexer.rs` `pub fn tokenize` (~112), `enum Open` (~102-110), the `'(' =>` arm (~316), the `')' =>` arm (~320), `']' =>` (~331), `']]'` branch (~295), end of `tokenize`. Test module at the bottom of the file (`fn toks`, ~351).
- **Constraints:** `src/lexer.rs` only. Do not change token types or the parser. Do not run `cargo fmt`.
- **Reference:** the existing `nested_index_brackets` test (~380) for bracket style; `unexpected_char_message` (~416) for error assertions.
- **Acceptance:** add lexer unit tests for `f(1]`, `x[1)`, `(1`, `x[[1]`, and `)` alone, each asserting an `Err` starting `ParseError:`. Run `cargo test --lib` (exit 0) and `cargo test --test gnu_r` (exit 0).

## Task 10.1.3 -- G15: verify the G9 fuel test fires at the subset step (read-only)

- **Repo:** rzero
- **Files in scope:** must edit: none. Can create: none (scratch output in the session scratchpad only).
- **Depends on:** none.
- **Spec:**
  1. Run `cargo test --test conformance conformance_g9_fuel_exhaustion_large_frame` and record the result.
  2. The test asserts only the `ErrFuelExhausted:` prefix. Check the goal's claim "fires at the subset step" by reading the consuming code for each tail (`d[, 1]`, `d[1:1000, ]`, `d[1] <- 0`, `d[, 1] <- 0`), and by running one failing tail's error message through the `R0Vm::new(100_000)` path to see the step name if the message carries one. Read-only: no test file edit.
  3. If all four tails fire at the subset or assignment step, close G15 with the evidence in the report. If any tail fires somewhere else, or the test does not fire, STOP and report. Do not tighten the test (that would edit `tests/conformance.rs`, which 10.1.1 owns in this wave).
- **Anchors:** `tests/conformance.rs` `fn conformance_g9_fuel_exhaustion_large_frame` (~2014); `src/vm.rs` `fn consume_fuel` and the df subset code path (grep `consume_fuel` near the `Df` arms).
- **Constraints:** read-only. Do not run `cargo fmt`. Do not commit.
- **Acceptance:** the report includes the test result line and, for each tail, the consuming function and line.

## Task 10.1.4 -- G2: verify integer negation and abs preserve NA_INT and trap on overflow (read-only)

- **Repo:** rzero
- **Files in scope:** must edit: none. Can create: none.
- **Depends on:** none.
- **Spec:**
  1. Read the unary negation path (`src/vm.rs` around line 223, `map(|x| if x == NA_INT { NA_INT } else { -x })`) and the `abs` implementation (grep `"abs"`).
  2. Find existing tests that cover `-` on Int with `NA_INT`, `abs` on Int with `NA_INT`, and overflow (`-i32::MIN` and `abs(-2147483647L - 1L)`-style inputs; grep `ErrIntOverflow`). Record each test name.
  3. If each claim is covered by a test that runs, close G2 with the test names. If any claim has no test or is wrong, STOP and report. Do not add tests or code (that would be a new task).
- **Anchors:** `src/vm.rs:223` (negation), `grep -n '"abs"' src/vm.rs`, `grep -n ErrIntOverflow src/vm.rs tests/conformance.rs`.
- **Constraints:** read-only. Run `cargo test --lib` only to confirm the named tests pass.
- **Acceptance:** the report lists each covering test name and its pass/fail, or the gap found.

---

## Task 10.2.1 -- G13: one strict logical-mask rule

- **Repo:** rzero
- **Files in scope:** must edit: `src/vm.rs`, `docs/LANGUAGE_SPEC.md` (§3 mask text only, lines ~53 and ~146). Can create: none.
- **Depends on:** 10.1.1 (serialized, same file).
- **Spec:**
  1. Recon has already confirmed the code recycles length-1 masks at the three sites (`src/vm.rs` ~666-677, ~767-777, ~989-1000). Change each to require `mask.len() == n`, with one exception: `n == 1` accepts a length-1 mask. Otherwise return `ErrDimensionMismatch: Logical mask length <l> does not match <dim> <n>` (keep each site's existing wording).
  2. Update the spec §3 text at the two lines to state the strict rule. Remove "or `1`" wording. Keep the NA-in-mask rule.
  3. Update unit tests that rely on length-1 recycling: `src/vm.rs` `fn logical_mask_length_and_na` (~3807), `fn subset_assign_mask_and_index` (~3823), and any `which(mask)` test using a 100-length mask (~3591) only if it breaks.
  4. Check the corpus: `grep -l "\[" tests/r0/*.R0` for length-1 logical masks. If a corpus file relies on recycling, STOP and report it (GNU R recycles, so the file must be dropped or the restriction recorded). Do not edit the corpus.
- **Anchors:** the three mask sites above; `docs/LANGUAGE_SPEC.md` ~53 ("logical masks of length 1 or the dimension") and ~146 ("Mask length must equal `x.len()` or `1`").
- **Constraints:** only the listed files. Do not touch §9 or the Restrictions section (10.4.1 owns it). Do not edit `EXECUTION_PLAN_5.md`. Do not run `cargo fmt`.
- **Reference:** the existing error wording at the three sites.
- **Acceptance:** `cargo test --lib`, `cargo test --test conformance`, and `cargo test --test gnu_r` exit 0. `grep -n "or \`1\`" docs/LANGUAGE_SPEC.md` returns no mask-related hits.

## Task 10.3.1 -- G21: integer type for diff and data-frame single-index assignment

- **Repo:** rzero
- **Files in scope:** must edit: `src/vm.rs`, `tests/conformance.rs`. Can create: none.
- **Depends on:** 10.2.1 (serialized, same file).
- **Spec:**
  1. Analysis first (no edits): with `Rscript`, record `typeof(diff(1:5, lag = 2))`, `typeof(diff(c()))`, `typeof(diff(c(1, 4, 9)))`, and for the data frame case `d <- data.frame(a = c(1, 2, 3), b = c(4, 5, 6)); d[c(1, 2)] <- 1:3; typeof(d[, 2])` (and the integer-column variant). Record the exact GNU R rule for when the column stays integer. Write the results in the micro-plan.
  2. In the `diff` arm (`src/vm.rs:3083`), return `R0Val::Int` when the input is `Int` (including the `Range` case), otherwise keep `Dbl`. The NA rule (`is_missing`) carries over to the `Int` path using `NA_INT`.
  3. In data-frame single-index assignment (`src/vm.rs`, the `Df` arm of `subset_assign` around ~989-1100), keep the column's integer type where the verified GNU R rule keeps it. Do not coerce on write when GNU R does not.
  4. Update unit assertions in `diff_lag_semantics` (`src/vm.rs:5278`, the `Dbl(...)` expectations at ~5281-5291) to the new types, and the conformance asserts for diff and data-frame assignment type.
  5. Do not edit the spec or corpus files.
- **Anchors:** `src/vm.rs` `"diff" =>` (~3083) and the `Dbl(out)` return (~3135); `fn diff_lag_semantics` (~5278); the `Df` assignment branch (grep `subset_assign` and `Df` near 989-1100); `tests/conformance.rs` (grep `diff(` and `g9_assign_1index`).
- **Constraints:** `src/vm.rs` and `tests/conformance.rs` only. Do not run `cargo fmt`. Keep the `lag` validation unchanged.
- **Reference:** the `Range` handling in the `"sort"` arm (`Range` returns `Int` without materializing).
- **Acceptance:** `cargo test --lib`, `cargo test --test conformance`, and `cargo test --test gnu_r` exit 0. `make test-gnu-r` exits 0.

## Task 10.4.1 -- Spec restriction list

- **Repo:** rzero
- **Files in scope:** must edit: `docs/LANGUAGE_SPEC.md` "GNU R equivalence" Restrictions only. Can create: none.
- **Depends on:** 10.1.1, 10.2.1, 10.3.1.
- **Spec:**
  1. Remove the bullets that the landed goals lift: the `atan2(NaN, NA_real_)` bullet, the `var`/`sd` NaN bullet (both G20), and the integer-type bullet (G21). Keep the others (bare `c()`, `approx`, B2, B9, B10, B15).
  2. Add a bullet for G13: a length-1 logical mask that GNU R recycles now errors in R0 (`ErrDimensionMismatch`). Name GNU R's result and R0's result, in the existing bullet style.
  3. Use only results verified by `Rscript` in the earlier tasks. Do not write unverified claims.
- **Anchors:** `docs/LANGUAGE_SPEC.md` `- **Restrictions:**` (~335) and its bullets.
- **Constraints:** Restrictions section only. Do not touch §9 or §3 (10.2.1 owns §3). Do not run `cargo fmt`.
- **Acceptance:** `grep -n "Restrictions" -A12 docs/LANGUAGE_SPEC.md` shows the updated list. `cargo test --test gnu_r` exits 0.

## Task 10.5.1 -- G16: cargo fmt across the crate

- **Repo:** rzero
- **Files in scope:** must edit: every `src/*.rs` and `tests/*.rs` file (formatting only). Can create: none.
- **Depends on:** 10.1.2, 10.2.1, 10.3.1 (all code edits land first).
- **Spec:**
  1. Run `cargo fmt` once. Confirm `git diff --stat` touches only `.rs` files. Confirm no logic changed by re-running the full suite counts (step Acceptance). `git diff -w` is not a valid check: `cargo fmt` reflows lines, which whitespace-insensitive diff does not ignore.
  2. Commit once as `style: apply cargo fmt across crate`. This is the only commit in this task.
- **Anchors:** `Makefile` target `format` (`cargo fmt`).
- **Constraints:** formatting only. No logic change. No other file.
- **Acceptance:** `cargo fmt --check` exits 0. `cargo test` counts unchanged from the wave-4 gate result (record them in the report). `cargo test --test gnu_r` and `make test-gnu-r` exit 0. `git diff --stat` lists only `.rs` files.

---

## Gate (orchestrator, after wave 5)

1. `cargo test` (all suites) and `cargo test --test gnu_r`.
2. `make test-gnu-r`.
3. `cargo clippy --all-targets -- -D warnings`.
4. `cargo build --target wasm32-unknown-unknown --release`.
5. `cargo fmt --check`.
6. `git diff --stat 49a2baa -- src/` shows only expected files. `tests/r0/allowlist.txt` has no entries.
7. Update `planning/GOALS.md`: close G20, G21, G13, G16, G15, G3, G2 as verified; move them to Completed.
8. Record reports and commit ids in `STATE_10.md`.

## Escalation

- Worker needs out-of-scope edits: stop and report (decomposition bug: re-scope here).
- A corpus file disagrees with GNU R, or a corpus file relies on recycling: stop and report with R0 and Rscript output.
- G15 or G2 verification fails: stop and report; no ad hoc fix.
- Two identical test failures: stop and report (no thrash).

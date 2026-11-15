# Execution Plan 7 (`planning/EXECUTION_PLAN_7.md`)

Orchestration 7 is the source-fix prerequisite for G11 wave 2 (orchestration 8, the corpus migration). It aligns four R0 behaviours with GNU R and makes one design choice that GNU R itself leaves inconsistent.

## Goal & Scope

**Goal:** R0 behaves like GNU R for these five items, so that the class-B corpus programs in wave 2 can match GNU R without allowlist entries:

1. Loop value: `for` and `while` return `NULL` invisibly (G17).
2. List single-index assignment with an atomic value stores the first element, not the whole vector (G18, D2).
3. `median` with `na.rm = FALSE` and any missing value returns `NA`, not `NaN` (G18, D3).
4. `atan2` with a `NULL` argument errors, instead of returning `numeric(0)` (G18, D5).
5. A non-scalar `na.rm` is an error in every intrinsic that takes `na.rm` (decision below; D4).

**In scope**
- `src/vm.rs`: the loop arms, `parse_na_rm`, `eval_median`, the `atan2` arm, and `assign_cols`.
- Rust tests in `src/vm.rs` and `tests/conformance.rs` that assert the old behaviour. Update them to the new behaviour.
- `docs/LANGUAGE_SPEC.md`: §3 loop text, §7 rows for `median`, `atan2`, and the `na.rm` rule, §8 restriction list, and a new errata entry for each changed behaviour.

**Out of scope**
- Corpus files under `tests/r0/` (orchestration 8).
- `var` and `sd` with NaN. Not confirmed against GNU R. Verify only (see hazard H3).
- Any change to `tests/gnu_r.rs`, `tests/gnu_r/canon.R`, or `tests/r0/allowlist.txt`. The allowlist stays empty.
- Any other intrinsic's behaviour, and any new intrinsic.
- Restructuring of `subset_assign` or the loop evaluator beyond the changed return value.

**Verbatim constraints (from the user)**
- "separate source plan first (Recommended)" (answer to the source-fix sequencing question).
- "make this an error. we tested this with pmin. GNU R is inconsistnet so we need to just choose one" (answer to the `sum(..., na.rm = c(TRUE, FALSE))` question).
- "Haiku, serial waves, no worktrees (Recommended)" (answer to the worker question).
- "Keep wave 1 rule (Recommended)" (answer to the grouping question, for orchestration 8).
- "yes, main" (branch). Work on `main`.

## Decision Log

- **2026-11-14 (sequencing):** Source fixes run first in orchestration 7. Corpus wave 2 is orchestration 8. Each orchestration stays green on its own.
- **2026-11-14 (na.rm):** A non-scalar `na.rm` is an error, not a first-element rule. The user chose this because GNU R is inconsistent across intrinsics (checked with `pmin`). R0 aligns with the stricter behaviour (`sum` errors in GNU R). Spec §7 row 'na.rm rule' changes accordingly. Error prefix: `ErrInvalidCondition:`, the family the existing empty and NA `na.rm` errors already use.
- **2026-11-14 (loop value):** Aligns with GNU R (`NULL` invisible). Spec §3 and §8 change. Restriction list in §8 loses the loop bullet.
- **2026-11-14 (median):** GNU R's `median` returns `NA` for any `NA` or `NaN` with `na.rm = FALSE`. R0 aligns: return `NA_real_`, not the first payload. This also changes the spec's "first encountered missing value payload" rule for `median` (spec §7 row 57).
- **2026-11-14 (atan2):** Only `NULL` arguments error. `numeric(0)` still returns `numeric(0)`, matching GNU R. Error prefix: `TypeError:`.
- **2026-11-14 (list assign):** Atomic value into a list with `[<-` stores element `k % len` as a length-1 vector of the same type, matching GNU R (warning suppressed; R0 has no warnings).
- **2026-11-14 (workers):** Haiku workers, one task per wave, serial. Every task edits `src/vm.rs`, so no two tasks run in the same wave. No worktrees.
- **2026-11-14 (tracking):** Planning artifacts are tracked. Planning commits are `docs(planning):`, separate from code. Code commits are one per verified task.
- **2026-11-14 (allowlist):** Stays empty. Nothing in orchestration 7 is allowlisted.

## Recon / Hazards

- **H1 (`src/vm.rs` is 5,583 lines).** Every task greps anchors. Never read the whole file.
- **H2 (shared file).** All six tasks edit `src/vm.rs`. Sequence them. Never run two in parallel.
- **H3 (`var` and `sd` with NaN).** `conformance_g7_statistical_reductions` asserts `var(c(1, NaN, 3))` and `sd(c(1, NaN, 3))` give `NaN`. Not checked against GNU R. Task 7.3.1 runs `Rscript` on those two expressions and reports the result. If GNU R gives `NA`, STOP and report. Do not change `var` or `sd` in this orchestration.
- **H4 (`c()` is `Null`).** `atan2(c(), 1)` now errors. Any test that relies on `c()` as an empty numeric argument to `atan2` must be updated (7.4.1).
- **H5 (`na.rm` helper is shared).** `parse_na_rm` serves `sum`, `mean`, `min`, `max`, `any`, `all`, `prod`, `var`, `sd`, `median`, `pmin`, `pmax`. The single change in 7.2.1 applies to all of them. Check every test that passes a vector `na.rm`.
- **H6 (baseline).** `cargo test` passed at `0230ac9` (123 lib, 80 conformance, 3 gnu_r). `make test-gnu-r` exit 0 at the same commit. See `STATE_7.md` baseline.

## Wave & Dependency Table

| Wave | Task | Title | Depends on | Files (must edit) |
| :--- | :--- | :--- | :--- | :--- |
| 1 | 7.1.1 | Loop value returns `NULL` | none | `src/vm.rs` |
| 2 | 7.2.1 | Non-scalar `na.rm` is an error | 7.1.1 | `src/vm.rs`, `tests/conformance.rs` |
| 3 | 7.3.1 | `median` returns `NA` for missing values | 7.2.1 | `src/vm.rs`, `tests/conformance.rs` |
| 4 | 7.4.1 | `atan2` errors on `NULL` arguments | 7.3.1 | `src/vm.rs`, `tests/conformance.rs` |
| 5 | 7.5.1 | List single-index assign stores the first element | 7.4.1 | `src/vm.rs`, `tests/conformance.rs` |
| 6 | 7.6.1 | Spec and errata text | 7.1.1 to 7.5.1 | `docs/LANGUAGE_SPEC.md` |

Every wave holds one task. Waves are serial because all tasks share `src/vm.rs`.

## Task Specs

### 7.1.1: Loop value returns `NULL`
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** must edit `src/vm.rs`. Can create nothing.
- **Depends on:** none
- **Do not touch:** everything except the two loop arms and the Rust tests named below.
- **Grep anchors:** `Expr::For { var, iter, body }` (~L323), `Expr::While { cond, body }` (~L340), `Ok(ControlFlow::Val(last))` (two occurrences, ~L337 and ~L352). Test anchors: `fn break_next` and `break_next` in the `vm.rs` test module (grep `break_next`).
- **Spec:**
  1. In both loop arms, return `ControlFlow::Val(R0Val::Null)` at loop end. Keep `last` only if still needed; otherwise remove it. Body evaluation, `break`, and `next` are unchanged.
  2. Add one unit test in the `vm.rs` test module: `for (i in 1:3) i` and `while (FALSE) 1` each evaluate to `R0Val::Null`. Also `for (i in 1:3) { if (i == 2) break }` evaluates to `R0Val::Null`.
  3. Update any existing unit or conformance test that asserts a loop's value (not a final non-loop expression) to `R0Val::Null`. Grep the test modules for loop finals first.
- **Acceptance:** `cargo test --lib loop` (or the new test name) exits 0. `cargo test --test conformance` exits 0. Then `cargo test --test gnu_r` exits 0 (no loop is a corpus final, so no change expected).
- **Commit:** `fix(vm): loops return NULL invisibly as in GNU R`

### 7.2.1: Non-scalar `na.rm` is an error
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** must edit `src/vm.rs`, `tests/conformance.rs`. Can create nothing.
- **Depends on:** 7.1.1
- **Do not touch:** `parse_na_rm`'s other branches (`NA`, non-logical, and empty behaviour stay as they are).
- **Grep anchors:** `fn parse_na_rm` (~L1066). Tests: `fn na_rm_validation_rules` in `src/vm.rs` (~L3973), lines containing `na.rm = c(TRUE, FALSE)`. In `tests/conformance.rs`: `fn na_rm_validation` (~L457), line containing `na.rm = c(TRUE, FALSE)`.
- **Spec:**
  1. In `parse_na_rm`, for `Some(R0Val::Lgl(v))` with `v.len() != 1`, return `Err(format!("ErrInvalidCondition: na.rm in {} requires a length-1 logical value", func))`. Length-1 `TRUE` and `FALSE` keep working. `NA` (length 1) keeps its current error. Empty (`len 0`) now hits the new branch; the existing empty-vector test message changes, so update that expectation if it asserts the old text.
  2. In `src/vm.rs` `na_rm_validation_rules`, change the assertions for `sum(c(1L, NA_integer_), na.rm = c(TRUE, FALSE))` and `pmin(1L, NA_integer_, na.rm = c(FALSE, TRUE))` to assert `starts_with("ErrInvalidCondition:")`.
  3. In `tests/conformance.rs` `na_rm_validation`, change the `sum(c(1L, NA_integer_), na.rm = c(TRUE, FALSE))` assertion to expect an `ErrInvalidCondition:` error. Keep the other assertions.
- **Acceptance:** `cargo test --lib na_rm` exits 0; `cargo test --test conformance na_rm` exits 0.
- **Commit:** `fix(vm): reject non-scalar na.rm as GNU R does`

### 7.3.1: `median` returns `NA` for missing values
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** must edit `src/vm.rs`, `tests/conformance.rs`. Can create nothing.
- **Depends on:** 7.2.1
- **Do not touch:** `eval_var_sd`, `var`, `sd`.
- **Grep anchors:** `fn eval_median` (~L2073). In `tests/conformance.rs`: `median(c(1, NaN, 3))` in `fn conformance_g7_statistical_reductions` (~L1253).
- **Spec:**
  1. Before editing, run `Rscript -e 'median(c(1, NaN, 3))'` and record the output in the task report. Expected: `[1] NA`.
  2. In `eval_median`, when `!na_rm` and a missing value is found, return `R0Val::Dbl(vec![na_real()])`, not `vec![v]`. This drops the NaN payload.
  3. Update `conformance_g7_statistical_reductions` so `median(c(1, NaN, 3))` asserts `is_na_real` (not NaN).
  4. Add one unit test in `src/vm.rs`: `median(c(NaN, 1, 2))` returns NA. Also check `median(c(1, NA_real_, NaN))` returns NA.
- **Acceptance:** `cargo test --lib median` exits 0; `cargo test --test conformance g7_statistical` exits 0.
- **Commit:** `fix(vm): median returns NA for missing values as GNU R does`

### 7.4.1: `atan2` errors on `NULL` arguments
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** must edit `src/vm.rs`, `tests/conformance.rs`. Can create nothing.
- **Depends on:** 7.3.1
- **Do not touch:** the dimension-mismatch and numeric paths.
- **Grep anchors:** `"atan2" => {` (~L2939). In `tests/conformance.rs`: `fn conformance_g7_atan2`, section `// Empty inputs` (~L1119).
- **Spec:**
  1. Run `Rscript -e 'atan2(numeric(0), 1); atan2(1, numeric(0))'` and record output (expected: `numeric(0)` both). Then run `Rscript -e 'atan2(NULL, 1)'` and record the error (expected: non-numeric argument).
  2. In the `atan2` arm, when `y_val` or `x_val` is `R0Val::Null`, return `Err("TypeError: non-numeric argument to mathematical function in atan2".into())`. Keep the `ny == 0 || nx == 0` path returning `Dbl(Vec::new())`, so `numeric(0)` inputs still give empty output.
  3. In `conformance_g7_atan2` `// Empty inputs`: change `atan2(c(), 1)`, `atan2(1, c())`, and `atan2(c(), c())` to `eval_err` expecting `TypeError:`. Keep `atan2(numeric(0), numeric(0))` as an empty result.
  4. Add one unit test in `src/vm.rs`: `atan2(numeric(0), 1)` gives `Dbl([])`, and `atan2(NULL, 1)` errors with `TypeError:`.
- **Acceptance:** `cargo test --lib atan2` exits 0; `cargo test --test conformance g7_atan2` exits 0.
- **Commit:** `fix(vm): atan2 rejects NULL arguments as GNU R does`

### 7.5.1: List single-index assign stores the first element
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** must edit `src/vm.rs`, `tests/conformance.rs`. Can create nothing.
- **Depends on:** 7.4.1
- **Do not touch:** the data frame branch of `assign_cols` (`is_df == true`), and `subset_assign`.
- **Grep anchors:** `fn assign_cols` (~L865). The line `v if !is_df => vec![v.clone(); sel.len()],`. The helper `fn elem_at` (grep it for the element extraction helper). In `tests/conformance.rs`: `fn conformance_g9_assign_1index`, the block with `l["b"] <- c(1, 2, 3)` (~L1983).
- **Spec:**
  1. In `assign_cols`, for a list target (`!is_df`) and an atomic value `v`, replace `vec![v.clone(); sel.len()]` with one length-1 element per selected slot. Element `k` is `v`'s element `k % v.len()`, as a length-1 vector of the same type. Use the existing element helper if it fits; otherwise build the one-element vector directly per type (`Dbl`, `Int`, `Lgl`, `Chr`).
  2. Keep the `R0Val::List` and `R0Val::Df` value branches unchanged.
  3. Update `conformance_g9_assign_1index` so `l["b"] <- c(1, 2, 3)` then `l` gives `cols[1]` with length 1 and value `1.0`.
  4. Add one unit test in `src/vm.rs`: `l <- list(a = 1, b = 2); l["b"] <- c(1, 2, 3); l$b` gives `Dbl([1.0])`. Also `l["a"] <- "x"` gives a `Chr` element.
- **Acceptance:** `cargo test --lib assign` exits 0; `cargo test --test conformance g9_assign` exits 0.
- **Commit:** `fix(vm): list [<- with atomic value stores the first element as GNU R does`

### 7.6.1: Spec and errata text
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** must edit `docs/LANGUAGE_SPEC.md`. Can create nothing.
- **Depends on:** 7.1.1, 7.2.1, 7.3.1, 7.4.1, 7.5.1
- **Do not touch:** `src/**`, `tests/**`, `planning/**`.
- **Grep anchors:** `Loop value` and `while` returns text (~L88 and ~L89, the `for` rule line; ~L91 `break`/`next` loop-value sentence). §7 `na.rm` rule paragraph (~L241). §7 row `57 | median`. The `atan2` row (grep `| 6`–`atan2` in §7 table). §8 `Restrictions` list (~L335–336). §9 Errata Ledger, last entry (~L349).
- **Spec:**
  1. §3: change the `while` and `for` return text to `NULL` invisibly (`R0 and GNU R agree`). Remove the sentence claiming the last body value. Update the `break`/`next` sentence (~L91) to say the loop evaluates to `NULL`.
  2. §7 `na.rm` paragraph (~L241): replace "if `length(na.rm) > 1`, the first element is used" with "`na.rm` must have length 1; any other length traps with `ErrInvalidCondition`".
  3. §7 `median` row (57): replace "With `na.rm = FALSE`, returns first encountered missing value payload" with "With `na.rm = FALSE`, returns `NA_real_` if any missing value is present".
  4. §7 `atan2` row: state that `NULL` arguments trap with `TypeError:`, and `numeric(0)` returns `numeric(0)`.
  5. §8 restrictions: remove the loop-value bullet. Replace the bullet with: "The restriction list is empty; see the errata."
  6. §9 Errata: add B11 (loop value), B12 (list `[<-` atomic value), B13 (`median` missing value), B14 (`atan2` `NULL`), B15 (`na.rm` length). One line each, with the old and new behaviour.
- **Acceptance:** `grep -n "last body value" docs/LANGUAGE_SPEC.md` returns no matches. `grep -n "first element is used" docs/LANGUAGE_SPEC.md` returns no matches. `grep -n "B1[1-5]" docs/LANGUAGE_SPEC.md` shows 5 entries.
- **Commit:** `docs(spec): record loop, median, atan2, na.rm, and list-assign errata`

## Gate for the whole orchestration (run by the orchestrator, serially, after 7.6.1)
- `cargo test` exit 0
- `cargo clippy --all-targets -- -D warnings` exit 0
- `cargo build --target wasm32-unknown-unknown --release` exit 0
- `make test-gnu-r` exit 0

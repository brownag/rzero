# Orchestration 8 State (`planning/STATE_8.md`)

**Goal:** G11 wave 2. Migrate the value asserts of the 28 class-B tests into `tests/r0/` as one `list(...)` corpus program per test function (`EXECUTION_PLAN_8.md`).
**Branch:** `main`
**Start commit:** `5c6c829` (orchestration 7 gate green)

## Baseline (pre-wave-1)
- [x] `cargo test`: 125 lib, 80 conformance, 3 gnu_r passed (HEAD `5c6c829`, recorded in `STATE_7.md` `## Orchestration gate`)
- [x] `cargo clippy --all-targets -- -D warnings`: exit 0 (HEAD `5c6c829`)
- [x] `cargo build --target wasm32-unknown-unknown --release`: exit 0; `r0-wasm.wasm` 440,992 bytes (HEAD `5c6c829`)
- [x] `make test-gnu-r`: exit 0, 3 passed (HEAD `5c6c829`)
- [x] `Rscript --version`: R 4.6.2 at `/usr/bin/Rscript` (orchestration 7 Rscript checks)
- [x] `tests/r0/`: 56 files (55 `.R0` + `allowlist.txt`, empty allowlist)
- [ ] Pilot (8.1.1): `list(...)` final value matches GNU R under `r0_corpus_matches_gnu_r` (hazard H2). Result recorded in the 8.1.1 report.

## Wave Board

| Wave | Task ID | Corpus file | Source function | Status | Commit |
| :--- | :--- | :--- | :--- | :--- | :--- |
| 1 | `8.1.1` | `g6_constructors.R0` | `conformance_g6_constructors` | done | 2e5e4a9 |
| 1 | `8.1.2` | `g6_shape_data_frame.R0` | `conformance_g6_shape_data_frame` | done | efe30e8 |
| 1 | `8.1.3` | `g6_coercions.R0` | `conformance_g6_coercions` | done | 0e5c7a5 |
| 1 | `8.1.4` | `g6_rep_len.R0` | `conformance_g6_rep_len` | done | 4f66543 |
| 1 | `8.1.5` | `g6_rep.R0` | `conformance_g6_rep` | done | 4ade576 |
| 1 | `8.1.6` | `g6_seq.R0` | `conformance_g6_seq` | done | c914e23 |
| 1 | `8.1.7` | `state_string_escapes.R0` | `state_01_string_escapes` | done | a54c0dc |
| 1 | `8.1.8` | `na_rm_value_asserts.R0` | `na_rm_validation` | done (no-op) | 29ddf30 |
| 2 | `8.2.1` | `g7_predicates.R0` | `conformance_g7_predicates` | pending | |
| 2 | `8.2.2` | `g7_logical_reductions.R0` | `conformance_g7_logical_reductions` | pending | |
| 2 | `8.2.3` | `g7_trig.R0` | `conformance_g7_trig` | pending | |
| 2 | `8.2.4` | `g7_rounding.R0` | `conformance_g7_rounding` | pending | |
| 2 | `8.2.5` | `g7_atan2.R0` | `conformance_g7_atan2` | pending | |
| 2 | `8.2.6` | `g7_statistical_reductions.R0` | `conformance_g7_statistical_reductions` | pending | |
| 2 | `8.2.7` | `g7_cumulative_reductions.R0` | `conformance_g7_cumulative_reductions` | pending | |
| 3 | `8.3.1` | `g8_search.R0` | `conformance_g8_search` | pending | |
| 3 | `8.3.2` | `g8_match.R0` | `conformance_g8_match` | pending | |
| 3 | `8.3.3` | `g8_deduplication.R0` | `conformance_g8_deduplication` | pending | |
| 3 | `8.3.4` | `g8_sorting_and_ranking.R0` | `conformance_g8_sorting_and_ranking` | pending | |
| 3 | `8.3.5` | `g8_binning.R0` | `conformance_g8_binning` | pending | |
| 3 | `8.3.6` | `g8_extensions.R0` | `conformance_g8_extensions` | pending | |
| 4 | `8.4.1` | `g9_extract_index_kinds.R0` | `conformance_g9_extract_index_kinds` | pending | |
| 4 | `8.4.2` | `g9_df_list_single_index.R0` | `conformance_g9_df_list_single_index` | pending | |
| 4 | `8.4.3` | `g9_assign_1index.R0` | `conformance_g9_assign_1index` | pending | |
| 4 | `8.4.4` | `spec_integer_binary_arith.R0` | `spec_02_integer_binary_arithmetic_preserves_type` | pending | |
| 4 | `8.4.5` | `unary_pos_neg_range.R0` | `unary_pos_and_neg_range_and_type_checks` | pending | |
| 4 | `8.4.6` | `seq_len_bounds.R0` | `seq_len_and_seq_along_zero_and_invalid_bounds` | pending | |
| 4 | `8.4.7` | `locked_prelude_values.R0` | `locked_prelude_and_strict_user_function_args` | pending | |

## Wave Gates
- Wave 1 gate: done. cargo test 125/80/3 pass; clippy exit 0; wasm release exit 0; make test-gnu-r 3 pass (allowlist empty)
- Wave 2 gate: pending
- Wave 3 gate: pending
- Wave 4 gate: pending (also updates `planning/tasks/g11-conformance-classification.md`)

## Task Reports

(Workers append here under `## {{id}}`.)

## 8.1.1
- Status: done (pilot, H2 confirmed: list-final shape matches GNU R, no STOP needed).
- File created: `tests/r0/g6_constructors.R0` (one `list(...)` of 14 named value asserts).
- Code commit: `2e5e4a9` (`test(gnu-r): add g6 constructors corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (exit 0).
- `cargo test --test conformance conformance_g6_constructors`: PASS (exit 0).
- Value asserts migrated: 14. Error asserts kept in Rust: 5.
- Deviations from approved plan: none. Element names use the approved group-word prefixes (`default_length_*`, `explicit_0_*`, `positive_*`, `named_length_*`).
- Gotchas for dependents: `numeric(length = 2)` named-arg form and `character(3)` empty-string canonicalization both match GNU R. Planning file `planning/tasks/8.1.1.todo.md` is left uncommitted (not part of the code commit).

## 8.1.2
- Status: done (Null elements in list-final form match GNU R; no STOP needed).
- File created: `tests/r0/g6_shape_data_frame.R0` (two `data.frame` assignments, then one `list(...)` of 13 named value asserts).
- Code commit: `efe30e8` (`test(gnu-r): add g6 shape data frame corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (exit 0).
- `cargo test --test conformance conformance_g6_shape_data_frame`: PASS (exit 0).
- Value asserts migrated: 13. Error asserts kept in Rust: 4.
- Deviations from approved plan: none. Element names: `regular_*`, `empty_*`, `range_*`, `double_*`, `string_*`, `null_*`, `list_*`.
- Gotchas for dependents: `list(name = nrow(...))` keeps Null-valued elements and matches GNU R (9 Null elements in this program). Multi-statement preamble (plain newline-separated assignments) before the final `list(...)` parses and canonicalizes correctly. Planning file `planning/tasks/8.1.2.todo.md` is left uncommitted.

## 8.1.3
- Status: done (list-final form with `c()` Null elements matches GNU R; no STOP needed).
- File created: `tests/r0/g6_coercions.R0` (one `list(...)` of 16 named value expressions, prefixes `as_numeric_*`, `as_integer_*`, `as_logical_*`).
- Code commit: `0e5c7a5` (`test(gnu-r): add g6 coercions corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (exit 0).
- `cargo test --test conformance conformance_g6_coercions`: PASS (exit 0).
- Value asserts migrated: 16 expressions (covering 25 Rust value-assert statements). Error asserts kept in Rust: 6 (`TypeError:` on data.frame and list inputs).
- Deviations from approved plan: none.
- Gotchas for dependent tasks: `as.numeric`/`as.integer` on `"invalid"` strings emit GNU R coercion warnings on stderr; values still match. `c()` as argument to `as.*` returns zero-length vectors on both sides.

## 8.1.4
- Status: done (rep_len list-final form with `c()` Null element matches GNU R; no STOP needed).
- File created: `tests/r0/g6_rep_len.R0` (one `list(...)` of 7 named value expressions, prefixes `cyclic_*` and `na_*`).
- Code commit: `4f66543` (`test(gnu-r): add g6 rep_len corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (exit 0).
- `cargo test --test conformance conformance_g6_rep_len`: PASS (exit 0).
- Value asserts migrated: 7. Error asserts kept in Rust: 5.
- Deviations from approved plan: none.
- Gotchas for dependent tasks: `rep_len(c(), 0)` as a list element returns Null and matches GNU R (same `c()` pattern as 8.1.3). Planning file `planning/tasks/8.1.4.todo.md` is left uncommitted.

## 8.1.5
- Status: done (list-final form with `c()` Null elements and an NA_character_ element matches GNU R; no STOP needed).
- File created: `tests/r0/g6_rep.R0` (one `list(...)` of 11 named value expressions, prefixes `times_*`, `each_*`, `both_times_each`, `length_out_*`, `empty_*`, `na_chr_each`).
- Code commit: `4ade576` (`test(gnu-r): add g6 rep corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (exit 0).
- `cargo test --test conformance conformance_g6_rep`: PASS (exit 0; the substring filter also ran `conformance_g6_rep_len`, 2 passed).
- Value asserts migrated: 11. Error asserts kept in Rust: 6.
- Deviations from approved plan: none. Question 1 answered: `rep(1:3, 2)` kept as its own element.
- Gotchas for dependent tasks: `NA_character_` inside `c(...)` canonicalizes the same as GNU R. Planning file `planning/tasks/8.1.5.todo.md` is left uncommitted.

## 8.1.6
- Status: done (list-final form with `c()` empty-argument elements and Range-valued `seq` results matches GNU R; no STOP needed).
- File created: `tests/r0/g6_seq.R0` (one `list(...)` of 14 named value expressions, prefixes `seq_n_*`, `seq_from_to_*`, `seq_by_*`, `seq_length_out_*`, `seq_along_*`).
- Code commit: `c914e23` (`test(gnu-r): add g6 seq corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (ok, 1 passed).
- `cargo test --test conformance conformance_g6_seq`: PASS (ok, 1 passed).
- Value asserts migrated: 14. Error asserts kept in Rust: 5.
- Deviations from approved plan: none.
- Gotchas for dependent tasks: `seq(c())` and `seq(along.with = c())` as list elements canonicalize the same as GNU R (empty integer). Planning file `planning/tasks/8.1.6.todo.md` is left uncommitted.

## 8.1.7
- Status: done (escaped-quote and escaped-backslash string values match GNU R; no STOP needed).
- File created: `tests/r0/state_string_escapes.R0` (one `list(...)` of 2 named string values, `dq_escaped_quote` and `sq_escaped_backslash`).
- Code commit: `a54c0dc` (`test(gnu-r): add state string escapes corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (ok, 1 passed).
- `cargo test --test conformance state_01_string_escapes`: PASS (ok, 1 passed).
- Value asserts migrated: 2. Error asserts kept in Rust: 0. Tokenize-level assert kept in Rust: 1 (`tokenize(...).is_ok()`).
- Deviations from approved plan: none.
- Gotchas for dependent tasks: string values with `"` and `\` canonicalize the same as GNU R, so no `canon` quoting issue. Planning file `planning/tasks/8.1.7.todo.md` is left uncommitted.

## 8.1.8
- Status: done as no-op (0 value asserts).
- File created: none. The wave table lists `tests/r0/na_rm_value_asserts.R0`, so this deviates from that "new corpus file" column.
- Code commit: none.
- Acceptance: not run. No corpus file was created, so `r0_corpus_matches_gnu_r` has nothing new to check. `cargo test --test conformance na_rm_validation` was not run either.
- Value asserts migrated: 0. Error asserts kept in Rust: 4 (L463 ErrInvalidCondition, L464 ErrInvalidCondition, L465 TypeError, L466 ErrInvalidCondition).
- Deviations from approved plan: the approved plan made this a no-op per coordinator decision on Question 1.
- Gotchas for dependent tasks: the task-row example `sum(c(1, NA), na.rm = TRUE)` is not in this function. Do not add it. Planning file `planning/tasks/8.1.8.todo.md` is left uncommitted.

## 8.2.1
- Status: done (is.finite/is.infinite/anyNA value asserts match GNU R; no STOP needed).
- File created: `tests/r0/g7_predicates.R0` (one `list(...)` of 26 named value expressions: `is_finite_*` 7, `is_infinite_*` 7, `any_na_*` 12).
- Code commit: `10281bf` (`test(gnu-r): add g7 predicates corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (ok, 1 passed).
- `cargo test --test conformance conformance_g7_predicates`: PASS (ok, 1 passed).
- Value asserts migrated: 26. Error asserts kept in Rust: 12 (`eval_err` contains/starts_with).
- Deviations from approved plan: none.
- Gotchas for dependent tasks: `c()` and `numeric(0)`/`logical(0)` empty inputs canonicalize the same as GNU R. `NaN` counts as NA for `anyNA`, matching GNU R. Planning file `planning/tasks/8.2.1.todo.md` is left uncommitted.

## 8.2.2
- Status: done (any/all truth tables, na.rm, Int/Dbl/Range coercion values match GNU R; no STOP needed).
- File created: `tests/r0/g7_logical_reductions.R0` (one `list(...)` of 36 named value expressions: `any_truth_*` 8, `any_na_rm_*` 4, `all_truth_*` 8, `all_na_rm_*` 4, `any_coerce_*` 4, `all_coerce_*` 4, `any_range_*` 2, `all_range_*` 2).
- Code commit: `22b0da5` (`test(gnu-r): add g7 logical reductions corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (ok, 1 passed).
- `cargo test --test conformance conformance_g7_logical_reductions`: PASS (ok, 1 passed).
- Value asserts migrated: 36. Error asserts kept in Rust: 8 (TypeError x4, ErrInvalidCondition x1, missing argument x2 for `any()`/`all()`).
- Deviations from approved plan: none.
- Gotchas for dependent tasks: `any(c())`/`all(c())`/`logical(0)` empty inputs and `na.rm = FALSE` NA propagation canonicalize the same as GNU R. Planning file `planning/tasks/8.2.2.todo.md` is left uncommitted.

## 8.2.3
- Status: done with deviation (20 corpus elements, not 21; `sin(c())` excluded, see below).
- File created: `tests/r0/g7_trig.R0` (one `list(...)` of 20 named value expressions: `zero_*` 6, `vec_*` 3, `nan_*` 2, `int_cos_0L`, `range_sin_0L_0L`, `na_nan_*` 6, `empty_cos_numeric0`).
- Code commit: `27aa311` (`test(gnu-r): add g7 trig corpus program`).
- `cargo test --test gnu_r r0_corpus_matches_gnu_r`: PASS (ok, 1 passed).
- `cargo test --test conformance conformance_g7_trig`: PASS (ok, 1 passed).
- Value asserts migrated: 20. Error asserts kept in Rust: 5 (`eval_err` starts_with/contains: TypeError x3, missing argument x1, TooManyArguments x1).
- Deviations from approved plan: 20 elements, not 21. `sin(c())` is excluded from the corpus and kept in Rust (coordinator decision, H4-style). GNU R errors on `sin(NULL)` ("non-numeric argument to mathematical function"), while R0 returns an empty Dbl. `cos(numeric(0))` is kept, since it is valid double input in both.
- Gotchas for dependent tasks: `c()` as a direct math-function argument is a GNU R error, so it cannot go in a corpus list. Use `numeric(0)` for empty trig inputs. `NaN` and `NA_real_` elements inside `c(...)` match GNU R under the canonical comparison. Planning file `planning/tasks/8.2.3.todo.md` is left uncommitted.

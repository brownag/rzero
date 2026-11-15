# Execution Plan 8 (`planning/EXECUTION_PLAN_8.md`)

Orchestration 8 is G11 wave 2: migrate the value asserts of the 28 class-B tests into the `tests/r0/` corpus, so GNU R checks each value. Source fixes it needed (orchestration 7) are done.

## Goal & Scope

**Goal:** Every class-B test function in `tests/conformance.rs` has one corpus program in `tests/r0/` whose final value is a `list(...)` of that function's value asserts. `cargo test --test gnu_r` and `make test-gnu-r` exit 0 with the allowlist still empty.

**In scope**
- 28 new corpus files, one per class-B test function, in four waves (below).
- Corpus file naming: `tests/r0/<prefix>_<topic>.R0`, where the prefix is `g6_`, `g7_`, `g8_`, or `g9_` for the matching test group, and `spec_` / `state_` / `na_rm_` / `unary_` / `seq_len_` / `locked_` / `index_` for the leftover groups listed in the wave table.
- Classification file `planning/tasks/g11-conformance-classification.md`: updated only to record the migrated list at wave end (orchestrator task, not a worker task).

**Out of scope**
- Deleting or changing any Rust assert in `tests/conformance.rs`. Value asserts stay in Rust (decision D-8.2 below). Error asserts stay in Rust regardless.
- Any change to `src/`, `tests/gnu_r.rs`, `tests/gnu_r/canon.R`, or `tests/r0/allowlist.txt`. The allowlist stays empty.
- Class C (reject or resource) and class D (R0-only syntax) tests. They stay in Rust, per the classification.
- `var` and `sd` with NaN. Their asserts stay in Rust until G20 (`planning/GOALS.md`).
- The G12 transpiler and any R0 syntax beyond what the 28 functions already use.

**Verbatim constraints (from the user)**
- "One file per test function (Recommended)" (answer to granularity: each corpus file ends in a `list(...)` of that function's value asserts).
- "4 themed waves (Recommended)" (answer to wave grouping).
- "Keep them in Rust, exclude from corpus (Recommended)" (answer to `var` and `sd` NaN).
- "Track (Recommended)" (answer to planning-artifact tracking).
- "Keep them (Recommended)" (answer to whether Rust value asserts are deleted).

## Decision Log

- **2026-11-14 (granularity, D-8.1):** One corpus file per class-B test function. Each file's final expression is `list(...)` of that function's value asserts, in source order, with names matching the assert intent. Reason: the harness (`tests/gnu_r.rs` `r0_corpus_matches_gnu_r`) compares only the last value of each file, so one `list` is the only way to cover every value in one program.
- **2026-11-14 (waves, D-8.2):** Four waves, serial, sized 8, 7, 6, 7. The user asked for four themed waves. The 28 functions split as 6 G6 + 2 misc (wave 1, 8 tasks), 7 G7 (wave 2), 6 G8 (wave 3), and 3 G9 + 4 language-rule functions (wave 4, 7 tasks). This is a correction to the counts discussed earlier: the 6 misc functions do not fit one wave under the 8-task rule, so two go to wave 1 and four to wave 4.
- **2026-11-14 (Rust asserts, D-8.3):** Value asserts in `tests/conformance.rs` are kept, not deleted. Reason (user answer): no conformance.rs edits per task, so tasks stay disjoint and reversible. Cleanup is a later plan.
- **2026-11-14 (var/sd NaN, D-8.4):** The `var` and `sd` NaN asserts in `conformance_g7_statistical_reductions` are excluded from the corpus file and stay in Rust. Reason (user answer): G20 is not yet fixed, and the corpus must match GNU R. G20 adds them later.
- **2026-11-14 (serial tasks, D-8.5, judgment call):** Tasks run one at a time, even inside a wave. Reason: the corpus tasks are disjoint and could run in parallel, but the workers would commit to the same index and race on `.git/index.lock`. Serial matches the orchestration 7 preference (serial waves). Judgment call; the user may choose parallel execution with orchestrator-only commits.
- **2026-11-14 (tracking, D-8.6):** Planning artifacts are tracked (user answer). Plan, template, and state commit separately as `docs(planning):`. Corpus code commits are separate: `test(gnu-r): ...`.
- **2026-11-14 (commit subjects, D-8.7):** Each corpus task commits one file as `test(gnu-r): add <group> <topic> corpus program`, in the style of wave 1 (`be2adfa`).
- **2026-11-14 (allowlist, D-8.8):** Stays empty. Nothing in orchestration 8 is allowlisted.

## Recon / Hazards

- **H1 (harness compares final value only).** `tests/gnu_r.rs` `run_rscript_file` evaluates each `.R0` file in order and canonicalizes the last value (`canon(last)`). Each file must end in the `list(...)`. A bare value at the end would drop every earlier assert.
- **H2 (list canonicalization).** `canon_lines` handles `R0Val::List` as `list[n]` with names (`tests/gnu_r.rs` ~L80). Wave 1 pilot (task 8.1.1) must confirm a `list(...)` final value matches GNU R on one program before the other tasks rely on it. If it fails, STOP the wave and report.
- **H3 (error asserts stay).** Every `eval_err` and `TypeError`, `ErrInvalidCondition`, `ErrDimensionMismatch`, `ErrIntOverflow`, `ErrFuelExhausted`-style assert stays in Rust. Corpus files contain only programs that succeed in both R0 and GNU R.
- **H4 (known divergences in class-B tests).** These asserts must be left out of the corpus file, not adapted:
  - `spec_02_integer_binary_arithmetic_preserves_type`: `2147483647L + 1L` (R0 errors, GNU R gives NA).
  - `locked_prelude_and_strict_user_function_args`: shadowing a built-in name (R0 rule, GNU R allows).
  - `conformance_g9_extract_index_kinds`: `d[c(TRUE, FALSE), 1]` (GNU R recycles, R0 rejects).
  - `conformance_g7_statistical_reductions`: `var(c(1, NaN, 3))` and `sd(c(1, NaN, 3))` (D-8.4).
  - `conformance_g7_atan2`: `atan2(c(), ...)` and `atan2(NULL-like, ...)` error asserts (TypeError).
  - `na_rm_validation`: vector `na.rm` asserts (now `ErrInvalidCondition`).
- **H5 (`NULL` literal).** R0 has no `NULL` literal. Use `c()` for the Null value in corpus sources, as the 7.4.1 gotcha records.
- **H6 (baseline).** At `5c6c829`: `cargo test` exit 0 (125 lib, 80 conformance, 3 gnu_r); `cargo clippy --all-targets -- -D warnings` exit 0; wasm release build exit 0; `make test-gnu-r` exit 0. Recorded in `planning/STATE_7.md` `## Orchestration gate`.
- **H7 (R side).** `Rscript` 4.6.2 at `/usr/bin/Rscript` is required by `make test-gnu-r`. Workers run `Rscript` only through the harness, not by hand.
- **H8 (tests/r0 size).** 56 files now (`allowlist.txt` plus 55 `.R0`). Each wave adds 6 to 8.

## Wave & Dependency Table

| Wave | Task | Corpus file (new) | Source test function (`tests/conformance.rs`) | Depends on |
| :--- | :--- | :--- | :--- | :--- |
| 1 | 8.1.1 | `tests/r0/g6_constructors.R0` | `conformance_g6_constructors` (L470) | none |
| 1 | 8.1.2 | `tests/r0/g6_shape_data_frame.R0` | `conformance_g6_shape_data_frame` (L505) | 8.1.1 |
| 1 | 8.1.3 | `tests/r0/g6_coercions.R0` | `conformance_g6_coercions` (L539) | 8.1.2 |
| 1 | 8.1.4 | `tests/r0/g6_rep_len.R0` | `conformance_g6_rep_len` (L613) | 8.1.3 |
| 1 | 8.1.5 | `tests/r0/g6_rep.R0` | `conformance_g6_rep` (L640) | 8.1.4 |
| 1 | 8.1.6 | `tests/r0/g6_seq.R0` | `conformance_g6_seq` (L680) | 8.1.5 |
| 1 | 8.1.7 | `tests/r0/state_string_escapes.R0` | `state_01_string_escapes` (L421) | 8.1.6 |
| 1 | 8.1.8 | `tests/r0/na_rm_value_asserts.R0` | `na_rm_validation` (L462) | 8.1.7 |
| 2 | 8.2.1 | `tests/r0/g7_predicates.R0` | `conformance_g7_predicates` (L757) | 8.1.8 |
| 2 | 8.2.2 | `tests/r0/g7_logical_reductions.R0` | `conformance_g7_logical_reductions` (L834) | 8.2.1 |
| 2 | 8.2.3 | `tests/r0/g7_trig.R0` | `conformance_g7_trig` (L893) | 8.2.2 |
| 2 | 8.2.4 | `tests/r0/g7_rounding.R0` | `conformance_g7_rounding` (L978) | 8.2.3 |
| 2 | 8.2.5 | `tests/r0/g7_atan2.R0` | `conformance_g7_atan2` (L1071) | 8.2.4 |
| 2 | 8.2.6 | `tests/r0/g7_statistical_reductions.R0` | `conformance_g7_statistical_reductions` (L1161) | 8.2.5 |
| 2 | 8.2.7 | `tests/r0/g7_cumulative_reductions.R0` | `conformance_g7_cumulative_reductions` (L1275) | 8.2.6 |
| 3 | 8.3.1 | `tests/r0/g8_search.R0` | `conformance_g8_search` (L1435) | 8.2.7 |
| 3 | 8.3.2 | `tests/r0/g8_match.R0` | `conformance_g8_match` (L1478) | 8.3.1 |
| 3 | 8.3.3 | `tests/r0/g8_deduplication.R0` | `conformance_g8_deduplication` (L1515) | 8.3.2 |
| 3 | 8.3.4 | `tests/r0/g8_sorting_and_ranking.R0` | `conformance_g8_sorting_and_ranking` (L1583) | 8.3.3 |
| 3 | 8.3.5 | `tests/r0/g8_binning.R0` | `conformance_g8_binning` (L1654) | 8.3.4 |
| 3 | 8.3.6 | `tests/r0/g8_extensions.R0` | `conformance_g8_extensions` (L1717) | 8.3.5 |
| 4 | 8.4.1 | `tests/r0/g9_extract_index_kinds.R0` | `conformance_g9_extract_index_kinds` (L1887) | 8.3.6 |
| 4 | 8.4.2 | `tests/r0/g9_df_list_single_index.R0` | `conformance_g9_df_list_single_index` (L1933) | 8.4.1 |
| 4 | 8.4.3 | `tests/r0/g9_assign_1index.R0` | `conformance_g9_assign_1index` (L1983) | 8.4.2 |
| 4 | 8.4.4 | `tests/r0/spec_integer_binary_arith.R0` | `spec_02_integer_binary_arithmetic_preserves_type` (L398) | 8.4.3 |
| 4 | 8.4.5 | `tests/r0/unary_pos_neg_range.R0` | `unary_pos_and_neg_range_and_type_checks` (L445) | 8.4.4 |
| 4 | 8.4.6 | `tests/r0/seq_len_bounds.R0` | `seq_len_and_seq_along_zero_and_invalid_bounds` (L436) | 8.4.5 |
| 4 | 8.4.7 | `tests/r0/locked_prelude_values.R0` | `locked_prelude_and_strict_user_function_args` (L453) | 8.4.6 |

Waves are serial (D-8.5). Tasks inside a wave run one at a time, in the order of the table. Each task creates one new file. No task edits an existing file, so the tasks are disjoint in the file sense.

## Task Specs

Every task uses the shared rules below. Only the per-task row in the table, plus its exclusions, varies.

### Shared rules (all tasks)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** must create the one corpus file named in the table. Must not edit any other file.
- **Work:** read the source function by its anchor (`grep -n`, then read only that function's lines). Collect every assert whose expression returns a value (`assert_eq!`, `matches!` on a value, `assert!` on a value). Skip every error assert (`eval_err`, `assert!(... starts_with("..."))`, a `Err` match), and every exclusion listed for the task.
- **Corpus program:** one R0 file. Each collected expression becomes an element of a `list(...)` final expression, with a name that says what it checks. Earlier assignments in the function (`x <- ...`) are copied in as statements before the `list`. Only R0-valid syntax. No `NULL` literal (use `c()`).
- **Acceptance:** `cargo test --test gnu_r r0_corpus_matches_gnu_r` exits 0 (this runs every corpus file, including the new one, against `Rscript`). `cargo test --test conformance <function_name>` exits 0 (proves the Rust function is unchanged and still passes).
- **Commit:** one commit per task, touching only the new file: `test(gnu-r): add <group> <topic> corpus program`.
- **Exclusions:** the per-task list in the table notes below.

### Per-task notes (exclusions and hazards)
- **8.1.1 (`conformance_g6_constructors`):** pilot for H2. Before the other 27 tasks, confirm the `list(...)` final value canonicalizes the same as GNU R. If the harness fails only on the list shape, STOP the wave.
- **8.1.7 (`state_01_string_escapes`):** the Rust assert uses `tokenize`. Include only the value that R0 evaluates as an R0 string literal. Do not add tokenize-level checks.
- **8.1.8 (`na_rm_validation`):** value asserts only (for example `sum(c(1, NA), na.rm = TRUE)`). Vector `na.rm` asserts are `ErrInvalidCondition` errors and stay in Rust.
- **8.2.5 (`conformance_g7_atan2`):** `numeric(0)` value asserts go to the corpus. `c()` as an argument to `atan2` is a TypeError assert and stays in Rust.
- **8.2.6 (`conformance_g7_statistical_reductions`):** exclude `var(c(1, NaN, 3))` and `sd(c(1, NaN, 3))` (D-8.4). Keep the `median` NA asserts (fixed in 7.3.1).
- **8.4.1 (`conformance_g9_extract_index_kinds`):** exclude `d[c(TRUE, FALSE), 1]` (H4). The rest are value asserts.
- **8.4.3 (`conformance_g9_assign_1index`):** the list `[<-` value is now the first element (7.5.1). The corpus value is `l["b"]` after `l["b"] <- c(1, 2, 3)`, which is `1`.
- **8.4.4 (`spec_02_integer_binary_arithmetic_preserves_type`):** exclude `2147483647L + 1L` (H4). Keep all other value asserts.
- **8.4.5 (`unary_pos_and_neg_range_and_type_checks`):** type-check asserts are errors and stay in Rust. Only the range and value asserts go to the corpus.
- **8.4.6 (`seq_len_and_seq_along_zero_and_invalid_bounds`):** invalid-bound asserts are errors and stay in Rust. Only the zero-length and valid-bound value asserts go to the corpus.
- **8.4.7 (`locked_prelude_and_strict_user_function_args`):** exclude shadowing a built-in name (H4). The strict-argument value asserts go to the corpus only if R0 and GNU R both accept them.

## Gate for the whole orchestration (run by the orchestrator, serially, after each wave and at the end)
- `cargo test` exit 0
- `cargo clippy --all-targets -- -D warnings` exit 0
- `cargo build --target wasm32-unknown-unknown --release` exit 0
- `make test-gnu-r` exit 0

After wave 4 the orchestrator also updates `planning/tasks/g11-conformance-classification.md` to record which class-B functions are migrated, and records the count of corpus files.

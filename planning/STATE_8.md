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
| 1 | `8.1.1` | `g6_constructors.R0` | `conformance_g6_constructors` | pending | |
| 1 | `8.1.2` | `g6_shape_data_frame.R0` | `conformance_g6_shape_data_frame` | pending | |
| 1 | `8.1.3` | `g6_coercions.R0` | `conformance_g6_coercions` | pending | |
| 1 | `8.1.4` | `g6_rep_len.R0` | `conformance_g6_rep_len` | pending | |
| 1 | `8.1.5` | `g6_rep.R0` | `conformance_g6_rep` | pending | |
| 1 | `8.1.6` | `g6_seq.R0` | `conformance_g6_seq` | pending | |
| 1 | `8.1.7` | `state_string_escapes.R0` | `state_01_string_escapes` | pending | |
| 1 | `8.1.8` | `na_rm_value_asserts.R0` | `na_rm_validation` | pending | |
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
- Wave 1 gate: pending
- Wave 2 gate: pending
- Wave 3 gate: pending
- Wave 4 gate: pending (also updates `planning/tasks/g11-conformance-classification.md`)

## Task Reports

(Workers append here under `## {{id}}`.)

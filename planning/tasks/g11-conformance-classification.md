# G11 classification: `tests/conformance.rs` against GNU R

Source: `tests/conformance.rs` (80 `#[test]` functions, 2,014 lines). Checked on 2026-11-14 against Rscript 4.6.2.

Rule: a corpus program must be valid GNU R, accepted by R0, and have a final-expression value. Error-only assertions are R0 rejects, which are out of scope. Fuel and resource tests are R0 limits, not language behaviour.

## Summary

| Class | Test functions | Action |
| :--- | ---: | :--- |
| A. GNU-compatible value tests | 32 | Migrate to `tests/r0/` as one program per assertion group |
| B. Mixed: value asserts plus error asserts | 28 | Migrate the value asserts. Keep the error asserts in Rust |
| C. Reject-only or fuel/resource | 15 | Stay in Rust. Out of corpus scope |
| D. R0-only syntax (`fn`, `->`, type hints) | 4 (+1 gap) | Rewrite as GNU-valid input for G12 |
| Total | 80 | |

## A. GNU-compatible value tests (32)

- `newline_binary_op_continues`, `newline_statement_terminates`, `newline_inside_parens_suppressed`, `newline_inside_brackets_suppressed`, `newline_multiline_pipeline`
- `pipe_implicit_first_arg`, `pipe_placeholder_substitution` (needs R >= 4.2 for `_`)
- `kleene_and_full_table`, `kleene_or_full_table`, `kleene_not`
- `broadcast_scalar_vector`, `broadcast_equal_length`
- `index_positive`, `index_oob_na_padding`, `index_negative_drop`, `index_logical_mask`, `index_scalar`, `assign_subset_mask`, `index_preserves_int_type`
- `fuel_finite_loop_ok`
- `spec_errata_b1_*`, `spec_errata_b3_*`, `spec_errata_b4_*`, `spec_errata_b5_*`, `spec_errata_b6_*`
- `spec_01_na_character_literal`, `spec_03_empty_return_is_null`, `spec_04_pmin_pmax_preserve_integer_type`, `state_04_return_unwinds_value_subexpressions`
- `conformance_g9_break_next`
- `conformance_g9_extract_drop_rule`
- `conformance_g9_assign_2d`

## B. Mixed (28)

Value asserts go to the corpus. Error asserts stay in Rust.

- `spec_02_integer_binary_arithmetic_preserves_type` (`2147483647L + 1L` errors; R gives NA)
- `state_01_string_escapes` (uses Rust `tokenize`; the value part is portable)
- `seq_len_and_seq_along_zero_and_invalid_bounds`
- `unary_pos_and_neg_range_and_type_checks`
- `locked_prelude_and_strict_user_function_args` (shadowing is an R0 rule; GNU R allows it)
- `na_rm_validation` (see D4)
- `conformance_g6_constructors`, `conformance_g6_shape_data_frame`, `conformance_g6_coercions`, `conformance_g6_rep_len`, `conformance_g6_rep`, `conformance_g6_seq`
- `conformance_g7_predicates`, `conformance_g7_logical_reductions`, `conformance_g7_trig`, `conformance_g7_rounding`
- `conformance_g7_atan2` (see D5)
- `conformance_g7_statistical_reductions` (see D3)
- `conformance_g7_cumulative_reductions`
- `conformance_g8_search`, `conformance_g8_match`, `conformance_g8_deduplication`, `conformance_g8_sorting_and_ranking`, `conformance_g8_binning`, `conformance_g8_extensions`
- `conformance_g9_extract_index_kinds` (`d[c(TRUE,FALSE),1]` recycles in GNU R; R0 rejects, so keep that assert in Rust)
- `conformance_g9_df_list_single_index`
- `conformance_g9_assign_1index` (see D2)

## C. Reject-only or resource (15)

- Reject-only: `pipe_rhs_not_call`, `kleene_if_na_traps`, `kleene_if_vector_traps`, `broadcast_mismatch_traps`, `index_mixed_subscript_traps`, `index_scalar_oob_traps`, `spec_errata_b2_float_with_l_suffix_is_parse_error`, `conformance_g9_extract_errors`, `conformance_g9_assign_errors`
- Fuel and resource: `fuel_infinite_loop_halts`, `fuel_small_budget_vector_op_halts`, `conformance_g6_fuel_exhaustion`, `conformance_g7_fuel_exhaustion_and_locking`, `conformance_g8_fuel_exhaustion_and_locking`, `conformance_g9_fuel_exhaustion_large_frame`

## D. R0-only syntax (4 tests, one script) and one known gap

- `PEDON_SCRIPT` (`src/main.rs` E5 form) uses `: df`, `: dbl` type annotations and `-> dbl`. Tests: `pedon_depth_weighted_clay_0_100`, `pedon_approx_25_50_85`, `pedon_remaining_oob`, `pedon_result_is_list`.
  - This is the natural G12 transpiler input. Write a GNU-valid twin of the script for the corpus, and keep the R0 original as the transpiler source.
- `conformance_g9_break_in_condition_gap`: GNU R accepts `if (break)` inside a loop. R0 rejects it. Known gap G14.

## Divergences found (R0 returns a non-error result that differs from GNU R)

Confirmed with Rscript 4.6.2.

| ID | Case | GNU R | R0 | Options |
| :--- | :--- | :--- | :--- | :--- |
| D1 | Loop value: `for` / `while` return value | `NULL`, invisible | last body value | G17 |
| D2 | `l <- list(a=1,b=2); l["b"] <- c(1,2,3)` | `b` becomes `1` (warning) | `b` keeps 3 elements | align or restrict |
| D3 | `median(c(1, NaN, 3))` | `NA` | `NaN` | align or restrict |
| D4 | `sum(c(1L, NA), na.rm = c(TRUE, FALSE))` | error: `'length = 2' in coercion to 'logical(1)'` | `1` (first element used, spec §7.1) | restrict (likely) or align to error |
| D5 | `atan2(c(), 1)` | error: non-numeric argument | `numeric(0)` | align or restrict |

Checked and matching (no action): `rank` with NA and NaN, `cumprod`/`cummin` with NaN, `which.min`, `sort` and `order` with NA/NaN, `tabulate`, `findInterval` with NA and logical `x`, `seq(0)`, `rep_len(c(), 0)` (NULL), `var(c(5))`, `as.logical("foo")`, `round(-0.5)`, `round(1.2345, 3)`, `round(1.25, 1)`, and `d[c(1,2)] <- 1:3`.

Not verified yet: `median` with NA vs NaN beyond the one case above, and `var`/`sd` with NaN.

## R0 rejects that GNU R accepts (out of corpus scope, by the plan 6 rule)

- Shadowing built-in names (`sum <- function(x) 0`): locked prelude is an R0 rule.
- `2147483647L + 1L`: R0 `ErrIntOverflow`; GNU R gives `NA` with a warning.
- `1.5L`: GNU R errors too, but the parse-error messages differ.
- `d[c(TRUE, FALSE), 1]`: GNU R recycles the mask; R0 rejects it.

These belong in the spec's restriction list, not the allowlist, since they are rejects.

## Sizing

Class A and the B value asserts are about 60 test functions' worth of assertions. Each corpus file is one program with one final expression, so the migration is roughly 100 to 150 files. That is a multi-wave plan, not a single wave.

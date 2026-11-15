# TODO

## Goal
Migrate the 32 GNU-compatible (class A) conformance tests from `tests/conformance.rs` into `tests/r0/` corpus programs, so that `cargo test --test gnu_r` checks each one against GNU R (`Rscript`) with no mismatches and no allowlist entries.

Source classification: `planning/tasks/g11-conformance-classification.md`. Goal reference: G11 in `planning/GOALS.md`.

## Decisions (defaults, confirm on approval)
- **Branch:** `main` (user confirmed).
- **Planning artifacts:** tracked. Planning commits are `docs(planning):`, separate from corpus commits.
- **File grouping:** one `.R0` file per source test function (32 files). Each file is one valid GNU R program. Tests with several asserts or a table return one `list(...)` of the asserted values as the final expression, so the canonical form compares them. Single-assert tests return the value directly.
- **Prefixes:** `nl_` (newline), `pipe_`, `logic_` (Kleene), `bc_` (broadcast), `idx_` (indexing and assignment), `spec_` (errata and spec items), `state_`, `fuel_`, `ctl_` (loops, existing prefix), `df_` (data frame, existing prefix).
- **Scope:** only the 32 class-A tests. Class B value asserts, class C rejects and fuel tests, and the pedon script are out of wave 1.
- **No `src/**` changes.** A mismatch STOPs the wave and is reported. It is not allowlisted and not fixed here.
- **Validity:** each file uses only `<-`, `function`, `c()`, `list()`, base R functions, and no `fn`, `->`, or type hints.

## Notes
- Loop results must not be used as the final expression (decided: G17 is out of wave 1).
- `matches!` checks in `conformance_g9_extract_drop_rule` must be rewritten as R-visible values (for example `class()` or `ncol()`), since the canonical form does not compare Rust enum variants.
- Verification gate per wave: `cargo test --test gnu_r` exits 0 with no `reject:` lines and no mismatches. Final gate: `cargo test` and `make test-gnu-r` exit 0.
- Each source test function's Rust test stays in `tests/conformance.rs` unchanged. Migration adds corpus files only.
- Rscript is R 4.6.2 at `/usr/bin/Rscript`. Scratch files go in the scratchpad, not `/tmp`.

## Tasks

### 0. Planning baseline
- [x] Commit `planning/GOALS.md` and `planning/tasks/g11-conformance-classification.md` in one `docs(planning):` commit. (`a652f0c`)
- [x] Commit this `TODO.md` in its own `docs(planning):` commit. (`6e39775`)

### 1. Newline and pipe programs (7 files)
- [x] `tests/r0/nl_binary_op.R0` from `newline_binary_op_continues`
- [x] `tests/r0/nl_statement.R0` from `newline_statement_terminates`
- [x] `tests/r0/nl_parens.R0` from `newline_inside_parens_suppressed`
- [x] `tests/r0/nl_brackets.R0` from `newline_inside_brackets_suppressed`
- [x] `tests/r0/nl_pipeline.R0` from `newline_multiline_pipeline`
- [x] `tests/r0/pipe_implicit.R0` from `pipe_implicit_first_arg`
- [x] `tests/r0/pipe_placeholder.R0` from `pipe_placeholder_substitution`
- [x] Run `cargo test --test gnu_r`: exit 0, no mismatches, no `reject:` lines. Commit `test(gnu-r): add newline and pipe corpus programs`.

### 2. Logic and broadcast programs (5 files)
- [x] `tests/r0/logic_and_table.R0` from `kleene_and_full_table` (list of the 9 results)
- [x] `tests/r0/logic_or_table.R0` from `kleene_or_full_table` (list of the 9 results)
- [x] `tests/r0/logic_not.R0` from `kleene_not`
- [x] `tests/r0/bc_scalar_vector.R0` from `broadcast_scalar_vector`
- [x] `tests/r0/bc_equal_length.R0` from `broadcast_equal_length`
- [x] Run `cargo test --test gnu_r`: exit 0, no mismatches, no `reject:` lines. Commit `test(gnu-r): add logic and broadcast corpus programs`.

### 3. Indexing and assignment programs (7 files)
- [x] `tests/r0/idx_positive.R0` from `index_positive`
- [x] `tests/r0/idx_oob_na.R0` from `index_oob_na_padding` (list of the Dbl NA and Int NA results)
- [x] `tests/r0/idx_negative_drop.R0` from `index_negative_drop`
- [x] `tests/r0/idx_logical_mask.R0` from `index_logical_mask`
- [x] `tests/r0/idx_scalar.R0` from `index_scalar`
- [x] `tests/r0/idx_assign_mask.R0` from `assign_subset_mask`
- [x] `tests/r0/idx_preserves_int.R0` from `index_preserves_int_type`
- [x] Run `cargo test --test gnu_r`: exit 0, no mismatches, no `reject:` lines. Commit `test(gnu-r): add indexing and assignment corpus programs`.

### 4. Spec, errata, and state programs (10 files)
- [x] `tests/r0/spec_errata_b1.R0` from `spec_errata_b1_out_of_range_assignment_extends_with_na`
- [x] `tests/r0/spec_errata_b3.R0` from `spec_errata_b3_na_and_nan_are_distinct`
- [x] `tests/r0/spec_errata_b4.R0` from `spec_errata_b4_is_nan_intrinsic`
- [x] `tests/r0/spec_errata_b5.R0` from `spec_errata_b5_na_in_character_c_is_na_character`
- [x] `tests/r0/spec_errata_b6.R0` from `spec_errata_b6_is_na_on_character`
- [x] `tests/r0/spec_na_character.R0` from `spec_01_na_character_literal`
- [x] `tests/r0/spec_empty_return.R0` from `spec_03_empty_return_is_null`
- [x] `tests/r0/spec_pmin_pmax_int.R0` from `spec_04_pmin_pmax_preserve_integer_type`
- [x] `tests/r0/state_return_unwinds.R0` from `state_04_return_unwinds_value_subexpressions`
- [x] `tests/r0/fuel_finite_loop.R0` from `fuel_finite_loop_ok`
- [x] Run `cargo test --test gnu_r`: exit 0, no mismatches, no `reject:` lines. Commit `test(gnu-r): add spec and state corpus programs`.

### 5. Loop, extraction, and assignment programs (3 files)
- [x] `tests/r0/ctl_break_next_nested.R0` from `conformance_g9_break_next` (list of the 5 results)
- [x] `tests/r0/df_extract_drop_rule.R0` from `conformance_g9_extract_drop_rule`, with `matches!` checks rewritten as R-visible values
- [x] `tests/r0/df_assign_2d_values.R0` from `conformance_g9_assign_2d` (list of the 5 results)
- [x] Run `cargo test --test gnu_r`: exit 0, no mismatches, no `reject:` lines. Commit `test(gnu-r): add loop, extraction, and 2D assignment corpus programs`.

### 6. Coverage and final gate
- [x] Confirm `tests/r0/` has 32 new `.R0` files, one per class-A test function, and that every class-A test is accounted for in the file list.
- [x] Run `cargo test`: exit 0.
- [x] Run `make test-gnu-r`: exit 0.
- [x] Update `planning/GOALS.md` G11 with wave 1 done and the wave 2 scope. Commit as `docs(planning):`.
- [x] Mark all tasks `[x]` in `TODO.md`, commit as `docs(planning):`, then archive to `planning/TODO_20261114-4.md` in a separate `docs(planning):` commit.

## Discovered Tasks
- **D-1 (task 1 STOP):** `pipe_placeholder` mismatch. `xs |> approx(xs, ys, xout = _)` gives `double[3]` `10 20 30` in R0 (`docs/LANGUAGE_SPEC.md` §7 row 26: `approx` yields the interpolated `y` vector). GNU R `approx` returns `list(x = <xout>, y = <interpolated>)`. The mismatch is an R0 return-shape divergence, not a test-mapping problem. It is not in the class-A classification. Needs a decision before task 1 can finish: (a) restriction in the spec, with the file pinned as an allowlist entry with a reason, (b) align `approx` to return `list(x, y)` (a src change), or (c) drop this file from the corpus and keep the test in Rust. Not allowlisted or fixed without approval.
- Note: the other six task-1 files (`nl_*`, `pipe_implicit`) matched GNU R in the same run.
- **D-1 resolved (user chose option 2, align):** approved scope expansion, which overrides "No `src/**` changes" for this item only.
  - `src/vm.rs` `approx`: return `list(x = xout, y = interpolated)` (`R0Val::List` with names `x`, `y`).
  - Update Rust tests that compare an `approx` result to `Dbl`: `vm.rs` `approx_and_named_args`, `approx_rule_semantics`; `tests/conformance.rs` `conformance_g8_extensions`, `pipe_placeholder_substitution`, `pedon_approx_25_50_85`. Compare the `y` element instead (`$y` in R0, or the list's `y` column in Rust).
  - `docs/LANGUAGE_SPEC.md` §7 row 26: state the list return.
  - Verification: `cargo test` exit 0 after the change, then `cargo test --test gnu_r` exit 0 with `pipe_placeholder.R0` included.

All tasks in TODO.md are complete.

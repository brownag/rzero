# Execution Plan 3: G7 — Predicate, Logical, Trig/Rounding, and Statistical/Cumulative Intrinsics

**Goal:** Implement the 21 planned predicate, logical reduction, trigonometric/rounding math, and statistical/cumulative reduction intrinsics (`is.finite`, `is.infinite`, `any`, `all`, `anyNA`, `sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2`, `prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax`) in `src/vm.rs`, verify them against GNU R semantics with unit and conformance tests, and document them in `docs/LANGUAGE_SPEC.md` §7.1.

## Scope

- **In scope:**
  - Locked prelude registration and VM dispatch for all 21 G7 intrinsics in `src/vm.rs`
  - Deterministic fuel metering and `na.rm` validation (`parse_na_rm`) where applicable
  - Unit tests in `src/vm.rs` and conformance tests in `tests/conformance.rs`
  - Documentation in `docs/LANGUAGE_SPEC.md` §7.1
- **Out of scope:**
  - G8 search/sorting/binning intrinsics (`which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval`, `approx` `rule`, `diff` `lag`)
  - G9 loop control (`break`, `next`) and 2D DataFrame subsetting
  - G10 SSA compiler / vector-register bytecode engine

## Decision Log

- **2026-11-14 (Wave structure):** Serialized `src/vm.rs` edits across Waves 1–3 (`3.1.1` predicates/logical, `3.2.1` trig/rounding, `3.3.1` stats/cumulative) to avoid merge conflicts on the 2,913-line `src/vm.rs`, followed by parallel Wave 4 (`3.4.1` conformance tests + `3.4.2` spec update).
- **2026-11-14 (Tracking & gates):** Track `planning/EXECUTION_PLAN_3.md`, `planning/PROMPT_TEMPLATE_3.md`, and `planning/STATE_3.md` in Git with a separate `docs(planning):` commit. Final Wave 4 gates: `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`.

## Hazard Notes (Recon)

- `src/vm.rs` is 2,913 lines: always use `rg -n` grep anchors (`register_prelude`, `call_intrinsic`, `parse_na_rm`, `#[cfg(test)]`) instead of whole-file reads.
- All `src/vm.rs` tasks are strictly serial (Waves 1 → 2 → 3).
- `tests/conformance.rs` (749 lines) and `docs/LANGUAGE_SPEC.md` (310 lines) are disjoint and run in parallel in Wave 4.

## Wave Table

| Wave | Task ID | Title | Files in Scope | Depends On |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `3.1.1` | Predicate & logical reductions (`is.finite`, `is.infinite`, `any`, `all`, `anyNA`) | `src/vm.rs` | — |
| 2 | `3.2.1` | Trig & rounding math (`sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2`) | `src/vm.rs` | `3.1.1` |
| 3 | `3.3.1` | Statistical & cumulative reductions (`prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax`) | `src/vm.rs` | `3.2.1` |
| 4 | `3.4.1` | Conformance tests for all 21 G7 intrinsics | `tests/conformance.rs` | `3.3.1` |
| 4 | `3.4.2` | Update `docs/LANGUAGE_SPEC.md` §7.1 for G7 | `docs/LANGUAGE_SPEC.md` | `3.3.1` |

---

## Task Specs

### Task `3.1.1`: Predicate & logical reductions (`is.finite`, `is.infinite`, `any`, `all`, `anyNA`)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/vm.rs`
- **Depends on:** —
- **Do not touch:** `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`, `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`
- **Grep anchors:** `register_prelude`, `is.na`, `sum`, `parse_na_rm` in `src/vm.rs`
- **Spec:**
  1. Register `is.finite`, `is.infinite`, `any`, `all`, `anyNA` in `register_prelude`.
  2. `is.finite(x)` / `is.infinite(x)`: element-wise `Logical` vector; `NA`/`NaN` -> `false` (not `NA` in R for `is.finite`/`is.infinite`! Verify: in R `is.finite(NA)` is `FALSE`, `is.infinite(NA)` is `FALSE`, `is.infinite(Inf)` is `TRUE`, `is.infinite(-Inf)` is `TRUE`).
  3. `any(x, na.rm = FALSE)` / `all(x, na.rm = FALSE)`: logical reduction using `parse_na_rm`; follow GNU R 3-valued logic (`any(c(TRUE, NA))` is `TRUE`, `any(c(FALSE, NA))` is `NA`, `all(c(FALSE, NA))` is `FALSE`, `all(c(TRUE, NA))` is `NA`, `any(logical(0))` is `FALSE`, `all(logical(0))` is `TRUE`).
  4. `anyNA(x)`: returns scalar `Logical` (`true` if any element is `NA`/`NaN`, `false` otherwise; `false` for empty).
  5. Charge fuel proportional to vector length before allocating/scanning.
  6. Add unit tests in `src/vm.rs`.
- **Acceptance:** `cargo test --lib` passes with new unit tests for all 5 intrinsics.

### Task `3.2.1`: Trig & rounding math (`sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2`)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/vm.rs`
- **Depends on:** `3.1.1`
- **Do not touch:** `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `register_prelude`, `sqrt`, `exp`, `log` in `src/vm.rs`
- **Spec:**
  1. Register `sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2` in `register_prelude`.
  2. Unary math (`sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`): element-wise `Dbl` output, preserving `NA_REAL`/`NaN` and charging fuel proportional to length.
  3. `round(x, digits = 0)`: default `digits = 0`; supports scalar integer/double `digits` (positive, zero, negative); uses IEEE-754 round-to-even (ties to even) or standard R rounding, preserving `NA`.
  4. `atan2(y, x)`: binary element-wise `Dbl` with R recycling rules (error on non-multiple lengths, empty if either is empty), preserving `NA`.
  5. Add unit tests in `src/vm.rs`.
- **Acceptance:** `cargo test --lib` passes with new unit tests for all 9 intrinsics.

### Task `3.3.1`: Statistical & cumulative reductions (`prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax`)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/vm.rs`
- **Depends on:** `3.2.1`
- **Do not touch:** `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `register_prelude`, `cumsum`, `mean`, `parse_na_rm` in `src/vm.rs`
- **Spec:**
  1. Register `prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax` in `register_prelude`.
  2. `prod(x, na.rm = FALSE)`: scalar `Dbl` product (`prod(numeric(0))` is `1.0`).
  3. `var(x, na.rm = FALSE)` / `sd(x, na.rm = FALSE)`: sample variance (`n - 1` denominator) and standard deviation; return `NA_REAL` when `n < 2` after NA handling.
  4. `median(x, na.rm = FALSE)`: sort non-NA values (return `NA_REAL` if any NA when `na.rm = FALSE`, or if empty); average middle two for even length.
  5. `cumprod(x)`, `cummin(x)`, `cummax(x)`: cumulative vector operations matching `cumsum` NA propagation (once `NA` is encountered, remaining elements are `NA`).
  6. Add unit tests in `src/vm.rs`.
- **Acceptance:** `cargo test --lib` passes with new unit tests for all 7 intrinsics.

### Task `3.4.1`: Conformance tests for all 21 G7 intrinsics
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/conformance.rs`
- **Depends on:** `3.3.1`
- **Do not touch:** `src/vm.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `G6` or end of `tests/conformance.rs`
- **Spec:**
  1. Add conformance tests covering all 21 G7 intrinsics, including NA propagation, `na.rm` validation, empty vectors, recycling (`atan2`), and error cases.
  2. Run full gate: `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`.
- **Acceptance:** All 3 commands exit 0.

### Task `3.4.2`: Update `docs/LANGUAGE_SPEC.md` §7.1 for G7
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `docs/LANGUAGE_SPEC.md`
- **Depends on:** `3.3.1`
- **Do not touch:** `src/vm.rs`, `tests/conformance.rs`
- **Grep anchors:** `§7.1` or `7.1` in `docs/LANGUAGE_SPEC.md`
- **Spec:**
  1. Document all 21 G7 intrinsics in `docs/LANGUAGE_SPEC.md` §7.1 with signatures and key semantics (`na.rm`, recycling, NA behavior).
- **Acceptance:** `cargo test --quiet` passes and `docs/LANGUAGE_SPEC.md` §7.1 lists all 21 intrinsics.

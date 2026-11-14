# Execution Plan 4: G8 — Search, Sorting, Binning Intrinsics & `approx`/`diff` Extensions

**Goal:** Implement the 10 remaining search, sorting, and binning intrinsics (`which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval`) plus optional `rule` on `approx` and `lag` on `diff` in `src/vm.rs`, verify them against GNU R semantics with unit and conformance tests, and document them in `docs/LANGUAGE_SPEC.md` §7.1 (completing all 69 specified R0 intrinsics).

## Scope

- **In scope:**
  - Locked prelude registration and VM dispatch for all 10 G8 intrinsics (`which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval`) in `src/vm.rs`
  - Optional `rule` argument on `approx` and optional `lag` argument on `diff` in `src/vm.rs`
  - Deterministic fuel metering proportional to vector length / comparison count
  - Unit tests in `src/vm.rs` and conformance tests in `tests/conformance.rs`
  - Documentation in `docs/LANGUAGE_SPEC.md` §7.1 and §7.2
- **Out of scope:**
  - G9 loop control (`break`, `next`) and 2D DataFrame subsetting (`df[i, j]`, `df[mask, ]`)
  - G10 SSA compiler / vector-register bytecode engine
  - Independent backlog items (`G2`–`G5`)

## Decision Log

- **2026-11-14 (Wave structure):** Serialized `src/vm.rs` edits across Waves 1–3 (`4.1.1` search/dedup, `4.2.1` sorting/ranking, `4.3.1` binning & `approx`/`diff` extensions) to avoid merge conflicts on the 3,984-line `src/vm.rs`, followed by parallel Wave 4 (`4.4.1` conformance tests + `4.4.2` spec update).
- **2026-11-14 (Tracking & gates):** Track `planning/EXECUTION_PLAN_4.md`, `planning/PROMPT_TEMPLATE_4.md`, and `planning/STATE_4.md` in Git with a separate `docs(planning):` commit. Final Wave 4 gates: `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`.

## Hazard Notes (Recon)

- `src/vm.rs` is 3,984 lines: always use `rg -n` grep anchors (`register_prelude`, `call_intrinsic`, `approx`, `diff`, `#[cfg(test)]`) instead of whole-file reads.
- All `src/vm.rs` tasks are strictly serial (Waves 1 → 2 → 3).
- `tests/conformance.rs` (1,427 lines) and `docs/LANGUAGE_SPEC.md` (321 lines) are disjoint and run in parallel in Wave 4.

## Wave Table

| Wave | Task ID | Title | Files in Scope | Depends On |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `4.1.1` | Search & deduplication (`which.min`, `which.max`, `match`, `unique`, `duplicated`) | `src/vm.rs` | — |
| 2 | `4.2.1` | Sorting & ranking (`order`, `sort`, `rank`) | `src/vm.rs` | `4.1.1` |
| 3 | `4.3.1` | Binning & extensions (`tabulate`, `findInterval`, `approx` `rule`, `diff` `lag`) | `src/vm.rs` | `4.2.1` |
| 4 | `4.4.1` | Conformance tests for all G8 intrinsics and extensions | `tests/conformance.rs` | `4.3.1` |
| 4 | `4.4.2` | Update `docs/LANGUAGE_SPEC.md` §7.1–§7.2 for G8 | `docs/LANGUAGE_SPEC.md` | `4.3.1` |

---

## Task Specs

### Task `4.1.1`: Search & deduplication (`which.min`, `which.max`, `match`, `unique`, `duplicated`)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/vm.rs`
- **Depends on:** —
- **Do not touch:** `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`, `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`
- **Grep anchors:** `register_prelude`, `which`, `%in%`, `#[cfg(test)]` in `src/vm.rs`
- **Spec:**
  1. Register `which.min`, `which.max`, `match`, `unique`, `duplicated` in `register_prelude` and `intrinsic_signature`.
  2. `which.min(x)` / `which.max(x)`: return 1-based scalar `Int` index of the first minimum/maximum non-NA element; return empty `Int` (`integer(0)`) if `x` is empty or all `NA`/`NaN`.
  3. `match(x, table, nomatch = NA_integer_)`: return `Int` vector of 1-based first-match indices in `table` (or `nomatch` if not found), matching `%in%` equality/NA semantics; charge fuel proportional to `x.len() * table.len()`.
  4. `unique(x)` / `duplicated(x)`: `unique(x)` preserves first-occurrence order; `duplicated(x)` returns `Logical` vector (`false` on first occurrence, `true` on subsequent duplicates); charge fuel proportional to `x.len()`.
  5. Add unit tests in `src/vm.rs`.
- **Acceptance:** `cargo test --lib` passes with new unit tests for all 5 intrinsics.

### Task `4.2.1`: Sorting & ranking (`order`, `sort`, `rank`)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/vm.rs`
- **Depends on:** `4.1.1`
- **Do not touch:** `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `register_prelude`, `median`, `#[cfg(test)]` in `src/vm.rs`
- **Spec:**
  1. Register `order`, `sort`, `rank` in `register_prelude` and `intrinsic_signature`.
  2. `sort(x, decreasing = FALSE)`: sort `Int`, `Dbl`, `Lgl`, or `Chr` vectors (dropping `NA`s by default, matching R's `na.last = NA` default); charge fuel proportional to `x.len() * log2(x.len())`.
  3. `order(x, decreasing = FALSE)`: return 1-based `Int` permutation indices that sort `x` (with `NA`s placed last by default, matching R's `na.last = TRUE` default, stable tie-breaking).
  4. `rank(x)`: return `Dbl` vector of 1-based sample ranks with average tie-breaking (`ties.method = "average"` default), placing `NA`s last.
  5. Add unit tests in `src/vm.rs`.
- **Acceptance:** `cargo test --lib` passes with new unit tests for all 3 intrinsics.

### Task `4.3.1`: Binning & extensions (`tabulate`, `findInterval`, `approx` `rule`, `diff` `lag`)
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/vm.rs`
- **Depends on:** `4.2.1`
- **Do not touch:** `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `register_prelude`, `"approx"`, `"diff"`, `#[cfg(test)]` in `src/vm.rs`
- **Spec:**
  1. Register `tabulate` and `findInterval` in `register_prelude` and `intrinsic_signature`.
  2. `tabulate(bin, nbins = max(1, bin))`: count positive integer occurrences `1..=nbins` (ignoring `<= 0` and `NA`), returning `Int` vector of length `nbins`; charge fuel proportional to `bin.len() + nbins`.
  3. `findInterval(x, vec)`: for sorted non-decreasing `vec`, return `Int` vector of interval indices (`0` if `< vec[0]`, `k` if `vec[k-1] <= x < vec[k]`, `vec.len()` if `>= vec.last()`); preserve `NA`s.
  4. `approx(x, y, xout, rule = 1)`: add optional `rule = 1` (`1` = `NA` outside `[min(x), max(x)]`, `2` = clamp to `y` at boundary).
  5. `diff(x, lag = 1)`: add optional `lag = 1` (must be positive integer `>= 1`; return empty vector if `x.len() <= lag`).
  6. Add unit tests in `src/vm.rs`.
- **Acceptance:** `cargo test --lib` passes with new unit tests for `tabulate`, `findInterval`, `approx(..., rule=...)`, and `diff(..., lag=...)`.

### Task `4.4.1`: Conformance tests for all G8 intrinsics and extensions
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/conformance.rs`
- **Depends on:** `4.3.1`
- **Do not touch:** `src/vm.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `G7` or end of `tests/conformance.rs`
- **Spec:**
  1. Add conformance tests covering all 10 G8 intrinsics plus `approx(..., rule=...)` and `diff(..., lag=...)`, including `NA`/`NaN` handling, empty vectors, tie-breaking, and error cases.
  2. Run full gate: `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`.
- **Acceptance:** All 3 commands exit 0.

### Task `4.4.2`: Update `docs/LANGUAGE_SPEC.md` §7.1–§7.2 for G8
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `docs/LANGUAGE_SPEC.md`
- **Depends on:** `4.3.1`
- **Do not touch:** `src/vm.rs`, `tests/conformance.rs`
- **Grep anchors:** `## 7.` or `7.2 Unimplemented` in `docs/LANGUAGE_SPEC.md`
- **Spec:**
  1. Document all 10 G8 intrinsics and the `approx(rule)` / `diff(lag)` extensions in `docs/LANGUAGE_SPEC.md` §7.1.
  2. Update §7.2 to reflect that all 69 specified R0 intrinsics are now implemented.
- **Acceptance:** `cargo test --quiet` passes and `docs/LANGUAGE_SPEC.md` accurately lists all 69 intrinsics.

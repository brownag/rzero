# Orchestration 4 State (`planning/STATE_4.md`)

**Goal:** G8 — Search, Sorting, Binning Intrinsics & `approx`/`diff` Extensions (`EXECUTION_PLAN_4.md`)
**Branch:** `master`

## Baseline (Pre-Wave 1)
- [x] `cargo test --quiet`: 99 unit tests passed, 63 conformance tests passed (0 failed)
- [ ] `cargo clippy --all-targets -- -D warnings` (run at Wave 4 end)
- [ ] `cargo build --target wasm32-unknown-unknown --release` (run at Wave 4 end)

## Wave Board

| Wave | Task ID | Title | Status | Commit |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `4.1.1` | Search & deduplication (`which.min`, `which.max`, `match`, `unique`, `duplicated`) | complete | `179ebe5` |
| 2 | `4.2.1` | Sorting & ranking (`order`, `sort`, `rank`) | complete | `092cadd` |
| 3 | `4.3.1` | Binning & extensions (`tabulate`, `findInterval`, `approx` `rule`, `diff` `lag`) | complete | `7bfec76` |
| 4 | `4.4.1` | Conformance tests for all G8 intrinsics and extensions | complete | `dc15fb3` |
| 4 | `4.4.2` | Update `docs/LANGUAGE_SPEC.md` §7.1–§7.2 for G8 | complete | `ec71e81` |

## Task Reports

### 4.1.1
- **Status:** complete
- **Files changed:** `src/vm.rs` (lines 337-345, 1829-1836, 1948-2104, 2623-2704, 4235-4370)
- **Commit:** `179ebe5` (`feat(vm): add which.min, which.max, match, unique, and duplicated intrinsics`)
- **Tests:** `cargo test --lib` (103 passed, 0 failed)
- **Deviations:** none
- **Gotchas:** `NULL` in user code is represented by `c()` or `R0Val::Null`; `match` handles `c()`/`NULL` table without erroring on type mismatches.

### 4.2.1
- **Status:** complete
- **Files changed:** `src/vm.rs` (lines 342-344, 829-871, 1879-1881, 2154-2403, 4662-4772)
- **Commit:** `092cadd` (`feat(vm): add sort, order, and rank intrinsics`)
- **Tests:** `cargo test --lib` (107 passed, 0 failed)
- **Deviations:** none
- **Gotchas:** `sort` drops NAs/NaNs by default, but `order` and `rank` place them last in original order even when `decreasing = TRUE`; in `rank`, distinct NAs receive consecutive distinct ranks (not averaged).

### 4.3.1
- **Status:** complete
- **Files changed:** `src/vm.rs` (lines 348-350, 1875-1888, 2409-2545, 2827-2943, 5005-5163)
- **Commit:** `7bfec76` (`feat(vm): add tabulate and findInterval intrinsics, approx rule and diff lag`)
- **Tests:** `cargo test --lib` (112 passed, 0 failed)
- **Deviations:** none
- **Gotchas:** `findInterval` expects `vec` to be non-decreasing with no NAs/NaNs (errors otherwise); `tabulate` defaults `nbins = max(1, bin)` ignoring `<= 0` and `NA` (returns `c(0)` for empty/all-non-positive input).

### 4.4.1
- **Status:** complete
- **Files changed:** `tests/conformance.rs` (lines 1428-1845)
- **Commit:** `dc15fb3` (`test(conformance): add conformance tests for G8 intrinsics and extensions`)
- **Tests:** `cargo test --test conformance` (70 passed, 0 failed), `cargo test` (112 unit + 70 conformance passed, 0 failed), `cargo build --target wasm32-unknown-unknown --release` (exited 0)
- **Deviations:** none
- **Gotchas:** `cargo clippy --all-targets -- -D warnings` reported 6 `clippy::unnecessary_sort_by` errors in `src/vm.rs` from task 4.2.1; orchestrator will handle separately.

### 4.4.2
- **Status:** complete
- **Files changed:** `docs/LANGUAGE_SPEC.md` (lines 229-299)
- **Commit:** `ec71e81` (`docs(spec): document G8 intrinsics and extensions in language spec`)
- **Tests:** `cargo test --quiet` (112 unit + 70 conformance passed, 0 failed)
- **Deviations:** none
- **Gotchas:** none

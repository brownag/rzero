# Execution Plan 2: G6 — Constructor, Coercion, and Shape Intrinsics

## Goal & Scope

- **Goal:** Implement the 12 planned constructor, coercion, and shape intrinsics (`rep`, `rep_len`, `seq`, `numeric`, `integer`, `logical`, `character`, `as.numeric`, `as.integer`, `as.logical`, `nrow`, `ncol`) in `src/vm.rs` with pre-allocation fuel metering, locked prelude registration, and conformance tests (completing Goal **G6** in `planning/GOALS.md`).
- **In scope:**
  - `numeric`, `integer`, `logical`, `character`, `nrow`, `ncol` in `src/vm.rs` + unit tests
  - `as.numeric`, `as.integer`, `as.logical` in `src/vm.rs` + unit tests
  - `rep`, `rep_len`, `seq` in `src/vm.rs` + unit tests
  - Conformance tests in `tests/conformance.rs` and `docs/LANGUAGE_SPEC.md` §7.1 alignment (moving the 12 intrinsics out of the unimplemented list)
- **Out of scope:**
  - `G7` (21 predicate/math/stat intrinsics), `G8` (10 search/sort/binning intrinsics), `G9` (`break`/`next`, 2D DataFrame subsetting), `G10` (SSA/ISA compiler)
  - Refactoring `src/vm.rs` into `src/intrinsics/` submodules
- **Verbatim constraints:**
  - "pre-allocation fuel metering, locked prelude registration, and conformance tests" (`planning/GOALS.md`)
  - Locked prelude invariant: user functions cannot shadow prelude intrinsics.

## Decision Log

- **2026-11-14 — Structure & Parallelism:** Keep all 12 intrinsics in `src/vm.rs` and sequence by family across Waves 1–3 (no `src/intrinsics/` submodule refactor).
- **2026-11-14 — Planning Artifacts Tracking:** Track `planning/EXECUTION_PLAN_2.md`, `planning/PROMPT_TEMPLATE_2.md`, `planning/STATE_2.md`, and `planning/tasks/2.*.todo.md` in Git using separate `docs(planning):` Conventional Commits.
- **2026-11-14 — Wave-End Acceptance Bar:** Full gate at the end of every wave: `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`.

## Hazard Notes (from Recon)

- `src/vm.rs` is 1,698 lines; always use grep anchors (`fn call_intrinsic`, `fn consume_fuel`, `#[cfg(test)]`) rather than reading the whole file.
- All 12 intrinsics must call `self.consume_fuel(...)` **before** allocating output vectors (`Vec::with_capacity` / `vec![]`) to preserve deterministic pre-allocation fuel metering.
- `tests/conformance.rs` is 462 lines; append new conformance tests at the end.

## Wave Table

| Wave | Task ID | Title | Files in Scope | Depends On |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `2.1.1` | Constructors & shape (`numeric`, `integer`, `logical`, `character`, `nrow`, `ncol`) | `src/vm.rs` | — |
| 2 | `2.2.1` | Coercions (`as.numeric`, `as.integer`, `as.logical`) | `src/vm.rs` | `2.1.1` |
| 3 | `2.3.1` | Sequence & repetition (`rep`, `rep_len`, `seq`) | `src/vm.rs` | `2.2.1` |
| 4 | `2.4.1` | Conformance tests for all 12 G6 intrinsics | `tests/conformance.rs` | `2.3.1` |
| 4 | `2.4.2` | Update `docs/LANGUAGE_SPEC.md` §7.1 for G6 | `docs/LANGUAGE_SPEC.md` | `2.3.1` |

*(Note: Wave 4 tasks `2.4.1` and `2.4.2` touch disjoint files and run in parallel.)*

---

## Task Specs

### Task 2.1.1: Constructors & shape (`numeric`, `integer`, `logical`, `character`, `nrow`, `ncol`)
- **id:** `2.1.1`
- **repo:** `/home/andrew/AI/rzero`
- **files-in-scope:**
  - must edit: `src/vm.rs`
  - can create: `planning/tasks/2.1.1.todo.md`
  - temp/scratch: `.development/`
- **depends-on:** none
- **spec prose:**
  Implement `numeric(length = 0)`, `integer(length = 0)`, `logical(length = 0)`, `character(length = 0)`, `nrow(x)`, and `ncol(x)` in `VM::call_intrinsic` and ensure they are registered in the locked prelude.
  - Constructors (`numeric`, `integer`, `logical`, `character`): accept 0 or 1 argument (`length`, default `0`). Validate `length` is non-negative finite scalar (error on negative, `NA`, `NaN`, `Inf`, or length > 1). Call `self.consume_fuel(n)` **before** allocating the `n`-element zero/FALSE/"" vector.
  - `nrow(x)` / `ncol(x)`: accept 1 argument `x`. For `DataFrame`, return row count / column count as `Int` scalar (`NA_INT` or `i32`). For atomic vectors / `Null`, return `Null` (matching GNU R `nrow(1:3)` -> `NULL`).
  - Add unit tests in `src/vm.rs` covering normal cases, `length = 0`, negative/`NA` length errors, `nrow`/`ncol` on `DataFrame` and vectors, and prelude shadowing rejection.
- **constraints:** Do not touch `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`, or other `src/*.rs` files.
- **grep anchors:** `fn call_intrinsic` (`src/vm.rs:943`), `fn consume_fuel` (`src/vm.rs:59`), `fn locked_prelude_and_user_function_arg_binding` (`src/vm.rs:1669`).
- **acceptance:** `cargo test --lib` passes with new unit tests for all 6 intrinsics.

### Task 2.2.1: Coercions (`as.numeric`, `as.integer`, `as.logical`)
- **id:** `2.2.1`
- **repo:** `/home/andrew/AI/rzero`
- **files-in-scope:**
  - must edit: `src/vm.rs`
  - can create: `planning/tasks/2.2.1.todo.md`
  - temp/scratch: `.development/`
- **depends-on:** `2.1.1`
- **spec prose:**
  Implement `as.numeric(x)`, `as.integer(x)`, and `as.logical(x)` in `VM::call_intrinsic` and register them in the locked prelude.
  - Call `self.consume_fuel(x.len())` before allocating output.
  - Follow GNU R atomic coercion semantics:
    - `as.numeric`: `Dbl` -> `Dbl`, `Int` -> `Dbl` (`NA_INT` -> `NA_REAL`), `Lgl` -> `Dbl` (`TRUE` -> `1.0`, `FALSE` -> `0.0`, `NA` -> `NA_REAL`), `Chr` -> parse float or `NA_REAL` on invalid, `Null` -> empty `Dbl(vec![])`.
    - `as.integer`: `Int` -> `Int`, `Dbl` -> truncate toward zero (`NA_REAL`/`NaN`/`Inf` -> `NA_INT`), `Lgl` -> `0`/`1`/`NA_INT`, `Chr` -> parse int or `NA_INT`, `Null` -> empty `Int(vec![])`.
    - `as.logical`: `Lgl` -> `Lgl`, `Int` -> `!= 0` (`NA_INT` -> `NA`), `Dbl` -> `!= 0.0` (`NA_REAL`/`NaN` -> `NA`), `Chr` -> `"TRUE"`/`"T"`/`"true"` -> `TRUE`, `"FALSE"`/`"F"`/`"false"` -> `FALSE`, else `NA`, `Null` -> empty `Lgl(vec![])`.
  - Add unit tests in `src/vm.rs` covering all atomic types, `NA` propagation, and `Null`.
- **constraints:** Do not touch `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`, or other `src/*.rs` files.
- **grep anchors:** `fn call_intrinsic` (`src/vm.rs:943`), `fn consume_fuel` (`src/vm.rs:59`).
- **acceptance:** `cargo test --lib` passes with new unit tests for `as.numeric`, `as.integer`, `as.logical`.

### Task 2.3.1: Sequence & repetition (`rep`, `rep_len`, `seq`)
- **id:** `2.3.1`
- **repo:** `/home/andrew/AI/rzero`
- **files-in-scope:**
  - must edit: `src/vm.rs`
  - can create: `planning/tasks/2.3.1.todo.md`
  - temp/scratch: `.development/`
- **depends-on:** `2.2.1`
- **spec prose:**
  Implement `rep(x, ...)`, `rep_len(x, length.out)`, and `seq(...)` in `VM::call_intrinsic` and register them in the locked prelude.
  - `rep_len(x, length.out)`: repeat/truncate `x` cyclically to `length.out` (pre-allocate fuel `self.consume_fuel(out_len)` before allocating). Error on negative/`NA` `length.out` or empty `x` with `length.out > 0`.
  - `rep(x, times = 1, length.out = NULL, each = 1)`: support positional & named `times`, `each`, `length.out`. Pre-allocate fuel for output length before allocating.
  - `seq(from = 1, to = 1, by = ..., length.out = NULL)`: support `seq(n)` (1..n when `n >= 1`, `1:0` or empty), `seq(from, to)`, `seq(from, to, by=...)`, `seq(from, to, length.out=...)`. Pre-allocate fuel for output length before allocating.
  - Add unit tests in `src/vm.rs` covering `rep`, `rep_len`, `seq` edge cases (`length.out = 0`, `each`, `by`, negative step, fuel exhaustion).
- **constraints:** Do not touch `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`, or other `src/*.rs` files.
- **grep anchors:** `fn call_intrinsic` (`src/vm.rs:943`), `fn consume_fuel` (`src/vm.rs:59`).
- **acceptance:** `cargo test --lib` passes with new unit tests for `rep`, `rep_len`, `seq`.

### Task 2.4.1: Conformance tests for all 12 G6 intrinsics
- **id:** `2.4.1`
- **repo:** `/home/andrew/AI/rzero`
- **files-in-scope:**
  - must edit: `tests/conformance.rs`
  - can create: `planning/tasks/2.4.1.todo.md`
  - temp/scratch: `.development/`
- **depends-on:** `2.3.1`
- **spec prose:**
  Add end-to-end conformance tests in `tests/conformance.rs` covering all 12 G6 intrinsics (`rep`, `rep_len`, `seq`, `numeric`, `integer`, `logical`, `character`, `as.numeric`, `as.integer`, `as.logical`, `nrow`, `ncol`), including `NA` handling, `DataFrame` shape, error cases, and fuel exhaustion.
- **constraints:** Do not touch `src/*.rs` or `docs/LANGUAGE_SPEC.md`.
- **grep anchors:** `#[test]` at end of `tests/conformance.rs` (`tests/conformance.rs:422-462`).
- **acceptance:** `cargo test --test conformance` passes with new G6 conformance tests.

### Task 2.4.2: Update `docs/LANGUAGE_SPEC.md` §7.1 for G6
- **id:** `2.4.2`
- **repo:** `/home/andrew/AI/rzero`
- **files-in-scope:**
  - must edit: `docs/LANGUAGE_SPEC.md`
  - can create: `planning/tasks/2.4.2.todo.md`
  - temp/scratch: `.development/`
- **depends-on:** `2.3.1`
- **spec prose:**
  Update `docs/LANGUAGE_SPEC.md` §7.1 to remove the 12 G6 intrinsics (`rep`, `rep_len`, `seq`, `numeric`, `integer`, `logical`, `character`, `as.numeric`, `as.integer`, `as.logical`, `nrow`, `ncol`) from the "not implemented in v0.2" list and document them as implemented.
- **constraints:** Do not touch `src/*.rs` or `tests/conformance.rs`.
- **grep anchors:** `rep_len` (`docs/LANGUAGE_SPEC.md:266`).
- **acceptance:** `rg -n "rep_len" docs/LANGUAGE_SPEC.md` shows it is no longer listed as unimplemented.

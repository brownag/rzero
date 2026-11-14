# TODO

## Goal
Fix the four runtime discrepancies identified during the v0.2 spec audit (`seq_len` / `seq_along` edge cases, unary `+`/`-` on `Range` and non-numeric types, locked prelude / strict user-function argument binding, and `na.rm` validation), align `docs/LANGUAGE_SPEC.md`, and cover each fix with unit and conformance tests.

## Tasks

### 1. Fix `seq_len` and `seq_along` edge cases
- [x] In `src/vm.rs`, update `seq_len(x)` to require a length-1 non-NA non-negative integer (`n >= 0`), returning `R0Val::Int(Vec::new())` when `n == 0` and `R0Val::Range { start: 1, end: n, step: 1 }` when `n >= 1` (trapping on empty, length > 1, `NA_INT`, or `n < 0`).
- [x] In `src/vm.rs`, update `seq_along(x)` to return `R0Val::Int(Vec::new())` when `x.is_empty()` and `R0Val::Range { start: 1, end: x.len() as i32, step: 1 }` otherwise.
- [x] Add unit and conformance tests for `seq_len(0L)`, `seq_len(-1L)`, `seq_len(NA_integer_)`, `seq_len(c(1L, 2L))`, and `seq_along(c())`.

### 2. Fix unary `+` and `-` on `Range` and non-numeric types
- [x] In `src/vm.rs`, update `UnaryOp::Pos` and `UnaryOp::Neg` so both accept `Dbl`, `Int`, and `Range` (supporting `-(1:3)` and `+(1:3)`) and reject non-numeric types (`Lgl`, `Chr`, `Df`, `List`, `Null`) with an error.
- [x] Add unit and conformance tests for `-(1:3)`, `+(1:3)`, `+TRUE`, and `-"a"`.

### 3. Enforce locked prelude and strict user-function argument binding
- [ ] In `src/vm.rs`, reject user function declarations that attempt to shadow any of the 27 built-in intrinsic names, and ensure built-in intrinsics cannot be shadowed.
- [ ] In `src/vm.rs` (`call_user`), reuse strict argument binding so unknown named arguments, duplicate named arguments, and excess positional arguments return errors (`UnknownArgument`, `DuplicateArgument`, `TooManyArguments`), while still supporting positional, named, and default parameter binding.
- [ ] Add unit and conformance tests for intrinsic shadowing rejection and user-function argument errors.

### 4. Validate `na.rm` in `sum`, `mean`, `min`, `max`, `pmin`, and `pmax`
- [ ] In `src/vm.rs`, extract a helper to parse `na.rm`: when omitted (`None`), default to `false`; when provided, require `R0Val::Lgl(v)` with `!v.is_empty()` and `v[0] != LGL_NA` (taking `v[0] == LGL_TRUE` when `v.len() >= 1`, and returning an error on non-`Lgl`, empty `Lgl`, or `v[0] == LGL_NA`).
- [ ] Update `docs/LANGUAGE_SPEC.md` §7.1 to reflect the exact `na.rm` rule (takes first element if length > 1; rejects non-logical, length 0, and `NA`).
- [ ] Add unit and conformance tests for `na.rm = NA`, `na.rm = 1`, `na.rm = c(TRUE, FALSE)`, and empty logical `na.rm`.

### 5. Final verification
- [ ] Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`.
- [ ] Archive `TODO.md` to `planning/TODO_20261114-3.md` using a `docs(planning):` commit.

## Notes
- Branch: `master` (direct local commits, no remote pushes).
- Planning artifact: `TODO.md` is tracked and committed separately with `docs(planning):` commits, then archived to `planning/TODO_20261114-3.md`.
- Preserve all four project axioms and zero external dependencies.

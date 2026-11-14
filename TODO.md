# TODO

## Goal
Fix the correctness bugs inherited from the R0 spec (out-of-range assignment, invalid `1.5L`, NA vs NaN, `is.nan`, NA in character `c()`, `is.na` on character, and the `FOR_RANGE_STEP` encoding), record each clarification as an erratum in the spec docs, and cover each fix with conformance tests.

## Decisions (answered by user, "go with recs")
- Scope: the seven items above. Prelude constructors (`integer`, `numeric`, and similar from spec §6.1) are deferred to a separate plan.
- NA vs NaN are distinct values. NA keeps the R payload (low word 1954). `is.nan(NaN)` is TRUE and `is.nan(NA)` is FALSE. Both are `is.na` TRUE.
- Git: branch `fix/spec-inherited-bugs`, created off `master` at the start of implementation. No pushes.
- This plan's `TODO.md` is tracked. Plan commits are `docs(planning):`, separate from code.
- `1.5L` (any float literal followed by `L`) is a parse error, per spec §5 `INT_LIT`.
- Spec clarifications go in an `## Errata (clarifications)` section at the end of `docs/LANGUAGE_SPEC.md` and `docs/ARCHITECTURE.md`. Each erratum cites the bug ID below.

## Tasks

### 1. Spec clarifications (docs only)
- [x] Add `## Errata (clarifications)` to `docs/LANGUAGE_SPEC.md` with entries B1-B6 below, each giving the rule, the old prototype behavior, and the corrected behavior. Commit: `docs(spec): clarify inherited R0 rules in errata`.
- [x] Add the same heading to `docs/ARCHITECTURE.md` with entry B7 (FOR_RANGE_STEP encoding). Commit: `docs(arch): clarify FOR_RANGE_STEP encoding in errata`.

Entries (for the errata text):
- B1 Out-of-range assignment: `x[i] <- v` with i > length(x) extends x to length i, filling new positions with NA of typeof(x). Prototype silently ignored it.
- B2 Invalid `1.5L`: a float literal with an `L` suffix is a parse error. `INT_LIT` is digits only. Prototype lexed it as a double followed by an identifier.
- B3 NA vs NaN: NA_real_ is the R payload 1954 in the low word. NaN is any other NaN. `is.nan` is TRUE only for NaN. `is.na` is TRUE for both. NA prints as NA, NaN prints as NaN.
- B4 `is.nan(x)`: added to the intrinsic set, per spec §6.2.
- B5 NA in character `c()`: an all-NA logical argument promotes to NA_character_ when the result is character.
- B6 `is.na` on character: returns TRUE for NA_character_, FALSE otherwise.
- B7 FOR_RANGE_STEP: operands are A = r_iter, B = r_limit (8 bits). The signed 16-bit jump offset is the next 32-bit word, not Bx. Prototype's Bx overlapped B.

### 2. Lexer: `1.5L` is a parse error (B2)
- [ ] Add failing unit test in `src/lexer.rs`: `1.5L` and `1e3L` return Err containing `ParseError:`. Commit `test(lexer): cover float literal with L suffix`.
- [ ] Make the lexer return `ParseError: float literal cannot have L suffix` for any float literal followed by `L`. Keep `1L` and `2147483647L` working. Commit `fix(lexer): reject float literal with L suffix`.

### 3. NA vs NaN representation (B3, B4)
- [ ] Add failing unit tests in `src/value.rs`: `is_na_real(na_real())` is true; `is_na_real(f64::NAN)` is false; a NaN from `0.0/0.0` is not NA. Commit `test(value): cover NA and NaN distinction`.
- [ ] Change `is_na_real` to check the low 32 bits of the bits equal 1954 (0x7a2), with the high word NaN. Commit `fix(value): distinguish NA from NaN by payload`.
- [ ] Update arithmetic in `src/vm.rs` `eval_binary` so NA inputs give NA and NaN inputs give NaN (no longer collapsing NaN to NA). Add unit test `na_nan_propagation`. Commit `fix(vm): propagate NA and NaN separately in arithmetic`.
- [ ] Update `Display` for `R0Val::Dbl` so NA prints `NA` and NaN prints `NaN`. Commit `fix(value): print NaN distinct from NA`.
- [ ] Add the `is.nan` intrinsic in `src/vm.rs` `call_intrinsic` (TRUE only for NaN, not NA). Add unit test. Commit `feat(vm): add is.nan intrinsic`.

### 4. Out-of-range assignment (B1)
- [ ] Add failing unit test in `src/vm.rs`: `x <- c(1,2,3)\n x[5] <- 9\n x` gives `Dbl([1, 2, 3, NA, 9])`; the same for Int and Lgl targets. Commit `test(vm): cover out-of-range subset assignment`.
- [ ] In `subset_assign`, extend the target to the max positive index with NA of its type, then assign. Keep mask-length errors unchanged. Commit `fix(vm): extend vector on out-of-range subset assignment`.

### 5. Character NA and is.na (B5, B6)
- [ ] Add failing unit tests: `c("a", NA)` gives `Chr([Some("a"), None])`; `is.na(c("a", NA))` gives `Lgl([0, 1])`. Commit `test(vm): cover character NA and is.na`.
- [ ] In `common_concat`, an all-NA logical argument promotes to NA_character_ when the result is Chr. Non-NA logical mixed with Chr stays TypeError. Commit `fix(vm): promote all-NA logical to NA_character_ in c()`.
- [ ] Extend `is.na` to handle `R0Val::Chr` (None is NA). Commit `fix(vm): support is.na on character vectors`.

### 6. FOR_RANGE_STEP encoding (B7)
- [ ] Add failing unit test in `src/isa.rs`: `FOR_RANGE_STEP` encodes A = iter, B = limit, and a signed 16-bit offset in the following word; decode round-trips offsets from -32768 to 32767. Commit `test(isa): cover FOR_RANGE_STEP offset word`.
- [ ] Add `Instr::ext_offset(i16) -> u32` and `Instr::decode_ext_offset(u32) -> i16`. Document the encoding in the `isa.rs` module doc. Commit `feat(isa): encode FOR_RANGE_STEP offset in extension word`.

### 7. Conformance tests for every fix
- [ ] Add `spec_errata_` tests to `tests/conformance.rs` covering B1-B6 through `R0Vm::eval_script`. Commit `test(conformance): add spec errata conformance tests`.

### 8. Final gate
- [ ] Run `cargo test`, `cargo clippy --all-targets -- -D warnings`, and `cargo build --target wasm32-unknown-unknown --release`. Record each exit code and the wasm byte size in `TODO.md` Notes.
- [ ] Mark all tasks `[x]` in this file, commit as `docs(planning): mark spec errata plan complete`, then archive per the plan-first archive rule.

## Notes
- Verified baseline (probe, before any change): `1;;2` = 2 (harmless); `1.5L` errors as undeclared variable `L`; `x[5] <- 9` is silently ignored; `is.nan` unknown; `is.na(c("a", NA))` TypeError; `is.na(NaN)` TRUE for both NA and NaN.
- Risk (B3): changing `is_na_real` alters arithmetic and printing. Task 3 covers the vm and value changes together. Run the full `cargo test` after each commit in that group.
- Risk (B7): the extension-word encoding changes the instruction-length model. `isa.rs` has no codegen, so the change is layout-only.
- Deferred, not in this plan: prelude constructors (`integer`, `numeric`, `logical`, `character`, `rep`, `seq`, and the rest of spec §6.1), the `1:n` edge cases, the Kleene-complete prelude, and the unused `;;` term.
- Branch: created at implementation start from `master` as `fix/spec-inherited-bugs`. Commits are local only.
- Archive: on completion, move this file to `planning/TODO_20261114-1.md` (next available sequence for 2026-11-14), commit `docs(planning): archive spec errata TODO` separately.

## Discovered Tasks
(none yet)

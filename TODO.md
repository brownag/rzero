# TODO

## Goal
Remediate the adversarial-oracle findings in `r0-wasm`, add their verification predicates, and publish self-consistent v0.2 language and architecture specifications without adding dependencies or violating the four axioms.

## Tasks

### 1. Sandboxing and standards fixes
- [x] Move fuel checks ahead of allocations/materialization in range subsetting, assignment, binary evaluation, and range value conversion.
- [x] Make range length arithmetic overflow-safe and avoid incrementing the final range element.
- [x] Apply the oracle's NaN/NA missingness rules to `abs`, `cumsum`, and `%in%`; map out-of-range doubles to `NA_INT` during integer materialization.
- [x] Add the exact `[STATE-03]`, `[STD-02]`, `[STD-03]`, and `[STD-04]` verification predicates from `docs/ADVERSARIAL_ORACLE.md`.

### 2. Language/specification conformance fixes
- [x] Parse `NA_character_` as a character missing value and add the exact `[SPEC-01]` predicate.
- [x] Preserve integer results for integer `+`, `-`, `*`, `%%`, `%/%`, `pmin`, and `pmax`; report `ErrIntOverflow` on non-missing integer overflow; add the exact `[SPEC-02]` and `[SPEC-04]` predicates.
- [x] Parse zero-argument `return()` as `Null` and add the exact `[SPEC-03]` predicate.
- [x] Support escaped quotes and backslashes in strings and add the exact `[STATE-01]` predicate.
- [x] Ensure `Return` from a value subexpression unwinds the user function and add the exact `[STATE-04]` predicate.

### 3. Verification coverage
- [x] Add all required oracle predicates in unit tests or `tests/conformance.rs`, including `[STATE-03]`.
- [x] Run focused tests after each logical implementation group and preserve all four axioms.

### 4. v0.2 specifications
- [ ] Update `docs/LANGUAGE_SPEC.md` to v0.2, incorporating applicable errata B1–B6 and implementation realities; clearly list exactly 29 implemented and 40 planned/unimplemented intrinsics.
- [ ] Document integer overflow behavior and range bounds; remove normative claims for unsupported behavior.
- [ ] Update `docs/ARCHITECTURE.md` to v0.2, incorporating B7 and aligning crate, runtime, resource-control, and Wasm FFI descriptions with the implementation.

### 5. Final verification
- [ ] Run `cargo test`.
- [ ] Run `cargo clippy --all-targets -- -D warnings`.
- [ ] Run `cargo build --target wasm32-unknown-unknown --release` and report the resulting `.wasm` byte size.
- [ ] Confirm `Cargo.toml` has no added dependencies and report exact verification output.

## Notes
- Branch: remain on the existing clean `master` branch.
- Planning artifact: track `TODO.md`; archive it after completion per the plan-first workflow.
- Baseline on this checkout: `cargo test` passes 60 unit tests and 38 conformance tests; clippy passes. This differs from the older oracle baseline.
- Preserve the four axioms in `docs/ADVERSARIAL_ORACLE.md` §1. No dependency additions.
- No commits or remote actions until individually verified logical changes are ready; no pushes or other remote actions are authorized.

# Orchestration 2 State (`planning/STATE_2.md`)

**Goal:** G6 — Constructor, Coercion, and Shape Intrinsics (`EXECUTION_PLAN_2.md`)
**Branch:** `master`

## Baseline (Pre-Wave 1)
- [x] `cargo test --quiet`: 72 unit tests passed, 48 conformance tests passed (0 failed)
- [ ] `cargo clippy --all-targets -- -D warnings` (run at Wave 1 end)
- [ ] `cargo build --target wasm32-unknown-unknown --release` (run at Wave 1 end)

## Wave Board

| Wave | Task ID | Title | Status | Commit |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `2.1.1` | Constructors & shape (`numeric`, `integer`, `logical`, `character`, `nrow`, `ncol`) | completed | `f2dd868` |
| 2 | `2.2.1` | Coercions (`as.numeric`, `as.integer`, `as.logical`) | completed | `460f419` |
| 3 | `2.3.1` | Sequence & repetition (`rep`, `rep_len`, `seq`) | completed | `614b89e` |
| 4 | `2.4.1` | Conformance tests for all 12 G6 intrinsics | completed | `46ada06` |
| 4 | `2.4.2` | Update `docs/LANGUAGE_SPEC.md` §7.1 for G6 | pending | — |

## Task Reports

*(Workers append reports below under `## <id>`)*

## 2.1.1
- **status:** completed
- **files changed:** `src/vm.rs` (lines 344–349, 794–827, 1009–1011, 1279–1304, 1767–1869)
- **code commit:** `f2dd868` ("feat(vm): implement constructor and shape intrinsics")
- **tests:** `cargo test --lib` (77 passed, 0 failed); `cargo test` (125 passed: 77 unit, 48 conformance, 0 failed)
- **deviations:** none
- **gotchas:** Constructors enforce pre-allocation fuel consumption; `length` accepts scalar int/dbl/range >= 0 and errors on length != 1, negative, NA, NaN, Inf.

## 2.2.1
- **status:** completed
- **files changed:** `src/vm.rs` (lines 350–352, 1013–1015, 1310–1470, 2038–2146)
- **code commit:** `460f419` ("feat(vm): implement coercion intrinsics (as.numeric, as.integer, as.logical)")
- **tests:** `cargo test --lib` (82 passed, 0 failed); `cargo test` (130 passed: 82 unit, 48 conformance, 0 failed)
- **deviations:** none
- **gotchas:** Coercions charge fuel before output allocation; string coercion converts invalid/NA strings to NA; range directly materializes without double-charging fuel.

## 2.3.1
- **status:** completed
- **files changed:** `src/vm.rs` (lines 350–355, 832–1418, 1596–1601, 2053–2078, 2748–2914)
- **code commit:** `614b89e` ("feat(vm): implement sequence and repetition intrinsics (rep, rep_len, seq)")
- **tests:** `cargo test --lib` (87 passed, 0 failed); `cargo test` (135 passed: 87 unit, 48 conformance, 0 failed)
- **deviations:** none
- **gotchas:** `rep`/`rep_len`/`seq` enforce pre-allocation fuel consumption; `length.out` takes precedence over `times` in `rep`; empty vectors error with positive `length.out`.

## 2.4.2
- **status:** completed
- **files changed:** `docs/LANGUAGE_SPEC.md` (lines 229–277)
- **code commit:** `11c6a30` ("docs(spec): document G6 intrinsics in language specification")
- **tests:** `cargo test --quiet` (87 unit passed, 48 conformance passed, 0 failed)
- **deviations:** none
- **gotchas:** none

## 2.4.1
- **status:** completed
- **files changed:** `tests/conformance.rs` (lines 463–749)
- **code commit:** `46ada06` ("test(conformance): add conformance tests for 12 G6 intrinsics")
- **tests:** `cargo test --test conformance` (55 passed, 0 failed); `cargo test` (142 passed: 87 unit, 55 conformance, 0 failed)
- **deviations:** none
- **gotchas:** In R0 script evaluation, empty vectors/null are expressed as `c()` rather than `NULL` keyword; missing argument errors follow `<func>: missing argument <index>`.


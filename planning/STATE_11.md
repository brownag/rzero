# STATE_11: G12 (`.R0 -> .R` Transpiler)

## Baseline (2026-11-15)
- `cargo test --test gnu_r`: 3 passed, 0 failed (`canon_fmt_g15_edge_cases`, `canon_matches_on_fixed_values`, `r0_corpus_matches_gnu_r`)

## Wave Board

| Wave | Task ID | Title | Status | Commit |
|---|---|---|---|---|
| 1 | `11.1.1` | Implement `transpile_to_r` module & export | done | `fcf03b2` |
| 2 | `11.2.1` | Add R0-only corpus & `gnu_r.rs` equivalence test | done | `dc5c585` |

## Task Reports

## 11.1.1
- **Status:** done (orchestrator-verified).
- **Files:** `src/lib.rs` (+1, `pub mod transpile;`); `src/transpile.rs` (new, 219 lines, 7 unit tests).
- **Commit:** `fcf03b2` feat(transpile): add token-level R0 to GNU R transpiler.
- **Tests:** `cargo test --lib transpile` 7 passed, 0 failed. `cargo build --target wasm32-unknown-unknown --release` exit 0. Worker reported clippy 0 warnings and fmt clean.
- **Deviations:** none.
- **Gotchas:** API is `crate::lexer::tokenize`. Output normalized (comments dropped, `fn` to `function`). `=` inside `(`/`[`, `<-` elsewhere. `%>%` emitted as `|>`. Whole-number doubles print without `.0` (`1:10` preserved). Type stripping drops exactly one token after `:` or `->`.

## 11.2.1
- **Status:** done (orchestrator-verified).
- **Files:** `tests/r0/g12_transpile.R0` (new, 16 lines); `tests/gnu_r.rs` (+106; new `r0_transpiled_matches_gnu_r`; `r0_corpus_matches_gnu_r` skips `g12_*`).
- **Commit:** `dc5c585` test(gnu_r): add transpiled R0 equivalence check against GNU R.
- **Tests:** `cargo test --test gnu_r -- --test-threads=1`: 4 passed, 0 failed. `cargo test --lib transpile`: 7 passed. wasm release build exit 0. Worker reported clippy --tests and fmt clean.
- **Deviations:** the spec's "`fn` inside `list(...)`" is not in the corpus. R0 rejects anonymous `fn` at runtime (vm.rs:396), so it is covered by a transpile-then-`parse(text=)` check in the new test instead.
- **Gotchas:** R0 errors on a `g12_*` file fail the test; they are not skipped.

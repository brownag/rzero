# STATE_12: Anonymous Functions and Closures (G24, G25, G26, G27, G28, G30, G32, G33, G34)

Branch: `g12-transpiler` (local). Orchestration 12 is not started.

## Baseline (2026-11-15, before wave 1)
- `cargo test --lib`: 134 passed, 0 failed.
- `cargo test --test conformance`: 80 passed, 0 failed.
- `cargo test --test gnu_r -- --test-threads=1`: 4 passed, 0 failed (about 20s; needs `Rscript`).
- `cargo build --target wasm32-unknown-unknown --release`: exit 0 (verified in plan 11).
- `cargo fmt --check` and `cargo clippy`: clean at plan 11 completion; re-check at wave 6.

## Wave Board

| Wave | Task ID | Title | Status | Commit |
|---|---|---|---|---|
| 1 | `12.1.1` | `R0Val` function variant (G24) | pending | — |
| 1 | `12.1.2` | Lexer `\` introducer and transpiler test (G25, G30) | pending | — |
| 2 | `12.2.1` | Unnamed function expressions evaluate to values (G26) | pending | — |
| 2 | `12.2.2` | Function values are not printable (G34) | pending | — |
| 3 | `12.3.1` | `Expr::Call` takes a callee expression (G27, part 1) | pending | — |
| 4 | `12.3.2` | Calls through function values (G27, part 2) | pending | — |
| 5 | `12.4.1` | Closure scoping rule (G28) | pending | — |
| 6 | `12.5.1` | Corpus `g_anon_*` and gnu_r equivalence (G32) | pending | — |
| 6 | `12.5.2` | Spec for anonymous functions, closures, relaxed Axiom 1 (G33) | pending | — |

## Task Reports

## 12.1.2
- Status: done, commit c631c08 (`feat(lexer): lex \( as function introducer`).
- `src/lexer.rs`: guarded arm `'\\' if` next non-space/tab byte is `(` pushes `Tok::Fn`; otherwise the existing "Unexpected character" arm.
- Tests added: lexer `backslash_lambda_is_fn`, `bare_backslash_errors`; transpile `backslash_lambda`.
- `cargo test --lib lexer`: 13 passed, 0 failed.
- `cargo test --lib transpile`: 8 passed, 0 failed.
- `cargo fmt --check`, `cargo clippy`, wasm32 release build: clean.
- Diff: only src/lexer.rs (+16) and src/transpile.rs (+5, test module).

## 12.1.1
- Status: done, commit 532f9f4 (`feat(value): add R0Val::Fn function variant`).
- `src/value.rs`: `pub type UserFnValue = Box<UserFn>`; variant `R0Val::Fn(UserFnValue)`; arms in `na_of_type_like` (Null), `len` (1), Display (`<function>`, placeholder for 12.2.2).
- `src/vm.rs`: `UserFn` fields `pub(crate)` (struct was already `pub`).
- Only 3 match sites broke, all in value.rs; no vm.rs arms needed. No PartialEq (R0Val has none).
- `cargo check`: ok. `cargo test --lib`: 137 passed, 0 failed (134 baseline + 3 from 12.1.2).
- `cargo clippy`: 0 warnings. `cargo fmt --check`: clean. wasm32 release build: ok.
- Diff: src/value.rs (+8/-1), src/vm.rs (+3/-3).

## 12.2.2
- Status: done. Commit: 399fefd (test-only, src/value.rs).
- Display arm for R0Val::Fn (`<function>`) already existed from 12.1.1 (532f9f4); no source change.
- Added unit test `fn_prints_as_placeholder`.
- src/main.rs prints the final value via `println!("{}", res)`; works unchanged.
- cargo test --lib value: 16 passed, 0 failed.
- cargo test --lib: 138 passed, 0 failed.
- cargo build: ok.

## 12.2.1
- Status: done, commit 65c09b5 (feat(vm)).
- src/vm.rs: `Expr::FnDef` in value position now yields `R0Val::Fn` (params, body, env.clone()). Named `f <- fn` path unchanged.
- tests/conformance.rs: +4 tests (in list, paren, passed as arg, returned from list).
- Tests: conformance 84 passed (was 80), lib 138 passed, clippy and fmt clean. gnu_r not run.
- Finding: the parser has no paren node, so `v <- (fn(x) x)` still parses as a named definition (v goes to `functions`, not env; `g(v)` gives Undeclared variable). Binding a bare function value to a variable is not possible yet. The arg test passes the function inline `g((fn(x) x))` and via a list variable. Parser untouched per scope.
- Value-position functions are not callable yet (12.3.x).

## 12.3.1
- Status: done, commit a21d926 (refactor(ast)).
- src/ast.rs: `Expr::Call.func` is now `Box<Expr>`.
- src/parser.rs: calls parsed as a postfix op (`Tok::LParen`, bp 21) via new `parse_call_args`; `Ident` prefix arm now just `Expr::Ident`; `%in%` desugar and parser test strings updated; +1 test `call_on_call` (`f(1)(2)`).
- src/vm.rs: call site matches `Ident` callee (dispatch unchanged); other callees error "callee not supported yet" before args are evaluated.
- transpile.rs has no `Expr::Call` use; untouched.
- Tests: lib 139 passed (138 + 1), conformance 84 passed, clippy 0 warnings, fmt clean. gnu_r not run.
- Note: `l$f(1)` and `f(1)(2)` now parse (were parse errors); they error at eval until 12.3.2.

## 12.3.2
- Status: done, commit c249a4b (feat(vm)).
- src/vm.rs: Ident eval falls back to `self.functions` as `R0Val::Fn`. Call evaluates any callee: Ident checks env Fn, then functions, then intrinsics; other callees must yield Fn else "not a function".
- tests/conformance.rs: +7 tests (bound fn value as arg incl. `v <- (fn(x) x)`, `l$f(1)`, `l[["f"]](1)`, immediate call, `make(2)(3)`, defaults/named args, non-function callee, missing name).
- Deviation: `index_scalar` (the `[[x]]` path, not `subset2`) extended so `[[` on list/df accepts a single string index (name lookup, reuses "Unknown column" error). Approved by orchestrator; needed for `l[["f"]]`.
- Tests: conformance 90 passed (was 84), lib 139 passed, clippy and fmt clean. gnu_r not run.

## 12.4.1
- Commit 644cec0 (feat(vm)): top-level fns use live globals; nested fns snapshot the frame.
- Added `UserFn.top_level`, `R0Vm.depth`, `sync_globals`, `run_user`; value.rs test literal got `top_level: true` (only change there).
- Tests: conformance 91 pass (was 90), lib 139 pass; wasm release build, fmt, clippy clean. gnu_r not run.
- Existing tests changed: none (no test pinned the snapshot result).
- Added test: closure_scoping_toplevel_live_nested_snapshot (late reassignment 2, call-via-another-fn 2, make(2)(3)==5, param shadows global).

## 12.5.2
- Status: done, commit be70e9f (docs(spec)). Only docs/LANGUAGE_SPEC.md changed.
- Edited: §0 syntax line (fn, \(...), : type, -> type), Axiom 1 (transpile_to_r contract), §3.2 (Declaration, Scoping live/snapshot, Closure restriction, Function values not printable/comparison undefined), §6.2 keyword list (added \), §6.3 FnDef-must-be-RHS clause.
- Approved extra: lines 227 and 206 fixes (minimal).
- grep "must be bound": no match. grep "Axiom 1": line 13 shows relaxed wording.
- gnu_r not run (deferred to orchestrator after 12.5.1).
- Note: tests/gnu_r.rs had uncommitted changes from another worker; left unstaged.

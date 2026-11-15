# Orchestration 5 State (`planning/STATE_5.md`)

**Goal:** G9 — `break`/`next` and DataFrame/list bracket subsetting (read + write) (`EXECUTION_PLAN_5.md`)
**Branch:** `main`

## Baseline (Pre-Wave 1)
- [x] `cargo test --quiet`: 112 unit, 70 conformance passed (0 failed)
- [x] `cargo build --target wasm32-unknown-unknown --release`: exit 0
- [ ] `cargo clippy --all-targets -- -D warnings`: FAILS (6 `unnecessary_sort_by` in `src/vm.rs`) — fixed by `5.1.1`

## Wave Board

| Wave | Task ID | Title | Status | Commit |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `5.1.1` | Clippy residue + `break`/`next` | done | `335f143`, `87a3061` |
| 2 | `5.2.1` | Bracket extraction `x[i, j]`, `x[j]` | pending | — |
| 3 | `5.3.1` | Bracket assignment `x[i, j] <- v`, `x[j] <- v` | pending | — |
| 4 | `5.4.1` | Conformance tests for G9 | pending | — |
| 4 | `5.4.2` | Spec update for G9 | pending | — |

## Task Reports

## 5.1.1
- Status: done.
- Files: src/vm.rs (sort_by_key at order/rank Int/Lgl arms; ControlFlow::Break/Next, outside_loop helper, loops, call_user, extract_scalar_cond, eval_script; 6 new tests at end of mod tests), src/lexer.rs (Tok::Break/Next + keywords), src/ast.rs (Expr::Break/Next), src/parser.rs (prefix arms).
- Commits: 335f143 fix(vm) clippy; 87a3061 feat(vm) break/next.
- Tests: `cargo test --lib` 118 passed, 0 failed (112 + 6 new); `cargo test --lib break_` 6/6; `cargo clippy --all-targets -- -D warnings` exit 0.
- Deviations: none.
- Gotchas: break/next inside an if/while condition expression errors ("used outside a loop") even inside a loop (known gap; extract_scalar_cond has no ConditionFlow variant) -- note in spec / later task. Function escape reuses the same message text. Loop var from `1:n` is Int (affects test expectations). Conformance suite not run (out of scope).

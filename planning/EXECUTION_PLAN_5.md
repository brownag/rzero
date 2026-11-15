# Execution Plan 5 (`planning/EXECUTION_PLAN_5.md`)

## Goal & Scope

**Goal (G9):** Loop control statements (`break`, `next`) and DataFrame/list bracket subsetting — 2D `df[i, j]` / `df[mask, ]` / `df[, j]` and 1-index `df[j]` / `lst[j]` — are implemented for both extraction and assignment across `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, and `src/vm.rs`, with conformance coverage and spec updates. Also clears the G8 clippy residue.

**In scope**
- `break` / `next` in `for` and `while` (incl. nested; innermost loop only).
- 2D extraction `x[i, j]` on `Df` with empty-slot = all.
- 1-index extraction `x[j]` on `Df` / `List` (column subset, returns same container type).
- Assignment `x[i, j] <- v` on `Df` and `x[j] <- v` on `Df` / `List`.
- 6 `clippy::unnecessary_sort_by` errors in `src/vm.rs` (`order`/`rank` Int/Lgl paths).
- Conformance tests; `docs/LANGUAGE_SPEC.md` updates (§ near line 51 and line 87).

**Out of scope**
- `repeat` loops; labelled breaks.
- Growing frames via assignment (out-of-range index or unknown name → error).
- 2D indexing on atomic vectors / matrices; `drop =` argument; nested assignment targets (`a$b[i, j] <- v`).
- `ControlFlow` refactors beyond adding variants (G5 territory).

**Verbatim constraints:** "agree with rec" (drop=TRUE); "both" (read + write 2D); "yes for reading and writing" (1-index `df[j]`); "full" (full index set).

## Decision Log
- **2026-11-14 (drop):** `df[, j]` with a single column returns the bare column vector (R `drop = TRUE`); multiple columns return `Df`. `df[i, ]` always returns `Df`. `df[j]` (1-index) always returns `Df`/`List` (never drops).
- **2026-11-14 (assignment):** 2D assignment `df[i, j] <- v` in scope.
- **2026-11-14 (1-index):** `df[j]` / `lst[j]` in scope for read and write.
- **2026-11-14 (index kinds):** Full R set — positive ints, negative ints (exclusion; mixing signs errors), logical masks (recycled to dim; NA in mask errors), character names (column slot only; unknown name errors), empty slot = all. Assignment recycles `v` if `len(target) % len(v) == 0`, else error. Out-of-range / unknown-name writes error (no growth). Column type on assignment follows existing `subset_assign` type rules per column.
- **2026-11-14 (tracking):** Planning artifacts tracked; `docs(planning):` commits separate from code (precedent: `679c3d6`).
- **2026-11-14 (models):** Workers on `flash`; escalate to `pro` per ladder.
- **2026-11-14 (break/next semantics):** `break`/`next` outside a loop is a runtime error (`Runtime Error: 'break' used outside a loop`); a `break`/`next` escaping a user function body is also an error. Each consumes 1 fuel (via `eval`'s existing charge). Loop value after `break` is the last completed body value (or `Null`), matching existing loop return convention.

## Recon / Hazards
- `src/vm.rs` is 5,164 lines — grep anchors only. All `src/vm.rs` work is serialized (Waves 1–3).
- `enum ControlFlow { Val, Return }` (`vm.rs:26`) is matched exhaustively at **25** sites as `ret @ ControlFlow::Return(_) => return Ok(ret)`. Adding `Break`/`Next` requires updating every site; the cleanest form is `other => return Ok(other)` for non-`Val` arms, with loops (`Expr::For` ~270, `Expr::While` ~285) consuming `Break`/`Next`, and user-function call sites converting stray `Break`/`Next` into an error.
- Clippy residue: only the `Int` and `Lgl` arms of `order`/`rank` are flagged. The `Chr` (`String`) arms use identical text — a blanket find/replace breaks compilation (E0507). Edit Int/Lgl arms individually; use `sort_by_key(|a| a.1)` / `sort_by_key(|a| std::cmp::Reverse(a.1))` (both stable, preserve tie order).
- Parser: `[` handled at `parser.rs:~76` (`Tok::LBrack` → `Expr::Index1`). Need comma-aware parsing with optional empty slots. Check how `,` is tokenized (`Tok::Comma`).
- Assignment: `Expr::Assign` → `Expr::Index1` path at `vm.rs:~149` calls `subset_assign` (`vm.rs:692`); only `Ident` base supported — keep that restriction.
- Existing helpers: `subset` (`vm.rs:604`), `index_scalar` (668), `field` (681), `subset_assign` (692); `R0Val::Df { names, cols }` / `R0Val::List` (`value.rs:99-101`), constructor `R0Val::df` (`value.rs:~119`).
- Baseline: `cargo test` 112 unit + 70 conformance pass; WASM release builds; clippy **fails** (6 errors, fixed in 5.1.1).

## Wave Table

| Wave | Task | Title | Files | Depends on |
| :--- | :--- | :--- | :--- | :--- |
| 1 | `5.1.1` | Clippy residue + `break`/`next` | `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/vm.rs` | — |
| 2 | `5.2.1` | Bracket extraction `x[i, j]`, `x[j]` on Df/List | `src/parser.rs`, `src/ast.rs`, `src/vm.rs` | `5.1.1` |
| 3 | `5.3.1` | Bracket assignment `x[i, j] <- v`, `x[j] <- v` | `src/vm.rs` | `5.2.1` |
| 4 | `5.4.1` | Conformance tests for G9 | `tests/conformance.rs` | `5.3.1` |
| 4 | `5.4.2` | Spec update for G9 | `docs/LANGUAGE_SPEC.md` | `5.3.1` |

## Task Specs

### Task `5.1.1`: Clippy residue + `break` / `next`
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/vm.rs`
- **Depends on:** —
- **Do not touch:** `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `"return" => Tok::Return` (lexer ~243); `Tok::Return` in `parser.rs`; `Return(Box<Expr>)` in `ast.rs`; `enum ControlFlow`, `ControlFlow::Return`, `Expr::For`, `Expr::While`, `"order" =>`, `"rank" =>`, `sort_by(|a, b|` in `vm.rs`
- **Spec:**
  1. Fix the 6 `clippy::unnecessary_sort_by` errors in the `Int`/`Lgl` arms of `order` and `rank` only (see Hazards). Verify `cargo clippy --all-targets -- -D warnings` is clean before continuing.
  2. Lexer: keywords `break` → `Tok::Break`, `next` → `Tok::Next`.
  3. AST: `Expr::Break`, `Expr::Next`. Parser: parse both as prefix expressions with no operand.
  4. VM: add `ControlFlow::Break` and `ControlFlow::Next`; update all match sites; `Expr::For`/`Expr::While` handle `Break` (exit loop) and `Next` (continue); `Expr::Block` propagates them; user-function invocation converts escaping `Break`/`Next` to an error; top-level `eval_script` converts them to `Runtime Error: '<kw>' used outside a loop`.
  5. Unit tests in `src/vm.rs`: break/next in for and while, nested loops (innermost only), break inside `if`, outside-loop error, escape-from-function error, interaction with `return`.
- **Acceptance:** `cargo test --lib` passes and `cargo clippy --all-targets -- -D warnings` exits 0.

### Task `5.2.1`: Bracket extraction `x[i, j]` and `x[j]` on Df/List
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/parser.rs`, `src/ast.rs`, `src/vm.rs`
- **Depends on:** `5.1.1`
- **Do not touch:** `src/lexer.rs` (unless `Tok::Comma` missing — then STOP and report), `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `Tok::LBrack =>` in `parser.rs` (~76); `Index1` in `ast.rs`; `Expr::Index1 { target, idx }`, `fn subset(`, `R0Val::Df`, `R0Val::List` in `vm.rs`
- **Spec:**
  1. AST: `Expr::Index2 { target: Box<Expr>, i: Option<Box<Expr>>, j: Option<Box<Expr>> }`.
  2. Parser: inside `[...]`, if a top-level `,` appears produce `Index2` with optional empty slots (`x[, j]`, `x[i, ]`, `x[, ]`); otherwise keep `Index1`. `x[i, j, k]` is a parse error.
  3. VM helper (e.g. `resolve_index(idx: Option<&R0Val>, n: usize, names: Option<&[String]>) -> Result<Vec<usize>, String>`) implementing the full index set from the Decision Log; charge fuel proportional to result length.
  4. `Index2` eval on `Df`: select rows across chosen columns; single column + `j` given → bare vector (drop); else `Df`. Non-`Df` target → `TypeError`.
  5. Extend `subset` so `Index1` on `Df`/`List` selects columns (returns same container type, no drop).
  6. `Expr::Assign` with an `Index2` target must return a clear "not yet supported" error (filled in by 5.3.1) so exhaustive matches compile.
  7. Unit tests in `src/vm.rs` covering each index kind, drop rule, empty slots, errors (mixed signs, NA mask, unknown name, out of range).
- **Acceptance:** `cargo test --lib` passes; `cargo clippy --all-targets -- -D warnings` exits 0.

### Task `5.3.1`: Bracket assignment `x[i, j] <- v` and `x[j] <- v`
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `src/vm.rs`
- **Depends on:** `5.2.1`
- **Do not touch:** `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `tests/conformance.rs`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `Expr::Assign`, `Expr::Index1 { target: base, idx }` (assign path ~149), `fn subset_assign`, `resolve_index` (from 5.2.1), `Index2` in `vm.rs`
- **Spec:**
  1. Replace the 5.2.1 placeholder: `Ident[i, j] <- v` on `Df` writes selected cells column-by-column, reusing `subset_assign` per column for type rules; `v` recycled across the selected cells (column-major) if lengths divide, else error.
  2. `Ident[j] <- v` on `Df`/`List`: if `v` is `Df`/`List`, assign its columns in order (count must match or recycle by divisibility); otherwise assign `v` (recycled to `nrow` for `Df`) into each selected column.
  3. Out-of-range index or unknown name → error (no growth). Preserve `Df` rectangularity.
  4. Charge fuel proportional to cells written.
  5. Unit tests: each index kind for writes, recycling success/failure, type coercion per column, list column replacement, errors.
- **Acceptance:** `cargo test --lib` passes; `cargo clippy --all-targets -- -D warnings` exits 0.

### Task `5.4.1`: Conformance tests for G9
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `tests/conformance.rs`
- **Depends on:** `5.3.1`
- **Do not touch:** `src/**`, `docs/LANGUAGE_SPEC.md`
- **Grep anchors:** `fn assert_val`, `fn eval_err`, `conformance_g8_` (end of file)
- **Spec:**
  1. Add `conformance_g9_*` tests: break/next (for, while, nested, outside-loop error); 2D extraction (all index kinds, drop rule, empty slots); `df[j]`/`lst[j]` extraction; 2D and 1-index assignment (recycling, errors); fuel exhaustion on large frames.
  2. Run full gate: `cargo test`, `cargo clippy --all-targets -- -D warnings`, `cargo build --target wasm32-unknown-unknown --release`.
- **Acceptance:** All 3 commands exit 0. If a gate fails on `src/` code, STOP and report (do not edit `src/`).

### Task `5.4.2`: Spec update for G9
- **Repo:** `/home/andrew/AI/rzero`
- **Files in scope:** `docs/LANGUAGE_SPEC.md`
- **Depends on:** `5.3.1`
- **Do not touch:** `src/**`, `tests/conformance.rs`
- **Grep anchors:** `Single-bracket subsetting` (~51), `Unimplemented control flow` (~87)
- **Spec:**
  1. Replace the "not supported" sentence (~51) with the extraction/assignment rules from the Decision Log (index kinds, drop, recycling, no growth).
  2. Replace the "Unimplemented control flow" line (~87) with `break`/`next` semantics (innermost loop, outside-loop error, function-escape error, loop value).
- **Acceptance:** `cargo test --quiet` passes; spec no longer lists `break`, `next`, `x[i]` on df/list, or `x[i, j]` as unsupported.

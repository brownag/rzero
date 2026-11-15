# EXECUTION_PLAN_12: Anonymous Functions and Closures (G24, G25, G26, G27, G28, G30, G32, G33, G34)

Status: DRAFT, pending user approval. Not started.

## Goal & Scope

- **Goal**: R0 gains first-class function values. Unnamed `fn(...)`, `function(...)`, and `\(...)` evaluate to functions that can be bound, stored in lists, returned from functions, passed as arguments, and called through variables, list elements, or directly. Closures work. The transpiler turns every such program into GNU R that gives the same result under `R0Vm`.
- **In scope**:
  - G24: `R0Val` function variant.
  - G25: lexer accepts `\` as a function introducer.
  - G26: unnamed function expressions evaluate to values.
  - G27: calls through any callee expression (variable, list element, direct).
  - G28: closures, with the scoping rule in the decision log.
  - G30: the transpiler emits `function` for `\(...)`. Satisfied by G25, since `\` lexes to the same token as `fn`; covered by a transpiler test in 12.1.2.
  - G32: corpus `tests/r0/g_anon_*.R0` runs equal under GNU R (via the transpiler) and `R0Vm`.
  - G33: `docs/LANGUAGE_SPEC.md` documents the forms, the relaxed Axiom 1, and the scoping rule.
  - G34: printing a function value is an error; canonical comparison of function values is undefined.
- **Out of scope (explicitly deferred)**:
  - G10: the `R0Vec` descriptor and 63-opcode engine. Its design will account for function values later.
  - G31: higher-order base functions (`lapply`, `sapply`, `Map`, `Filter`, `Reduce`, `do.call`).
  - Source printing of functions (GNU R's deparse format).
  - `λ` (UTF-8 lambda). Pruned.
  - Any `<<-` or mutable environment (already absent from R0 by design).
- **Verbatim constraints**:
  - User (2026-11-15): "R0 is no longer required to be a strict GNU R subset. Its contract is that `transpile_to_r` turns every R0 program into GNU R that evaluates identically under `R0Vm`."
  - User (2026-11-15): "returning a function is an important feature to have, but source printing is not necessary."
  - User (2026-11-15): "The crate is `cdylib` and `rlib`, and the WASM build must stay clean." (from AGENTS.md and plan 11)

---

## Decision Log

- **2026-11-15 — Relaxed contract**: R0 is not a strict GNU R subset. Axiom 1 becomes: every R0 program transpiles to valid GNU R that evaluates identically under `R0Vm`, within the supported subset, unless it hits an R0 strictness trap. (Source: GOALS.md G33.)
- **2026-11-15 — G24 folded into G10's design**: G10 is an unimplemented proposal (`docs/ARCHITECTURE.md` §§4–8; `src/isa.rs` scaffold only), so G10 stays out. The function value is built on the current `R0Val`.
- **2026-11-15 — Closure scoping, top level**: a function defined at top level looks up free variables live, at call time, in the global environment. This matches GNU R for top-level functions, including when a top-level variable is reassigned after the function is defined. This changes named-function behaviour (currently a snapshot). Existing tests that pin the snapshot result are updated in task 12.4.1 and the change is recorded in the spec.
- **2026-11-15 — Closure scoping, nested**: a function defined inside another function snapshots the enclosing frame's variables at creation. `make <- fn(k) fn(x) x + k` gives `make(2)(3) == 5`. Known divergence from GNU R: a later change to a variable of the enclosing frame is not seen. This is recorded as a restriction in the spec (task 12.5.2).
- **2026-11-15 — Function values**: returnable, storable, passable. Printing one is an error (`R0` does not print source). Canonical comparison of function values is undefined, so corpus files end with plain values, never a function.
- **2026-11-15 — Planning artifacts**: tracked in Git. Each commit is `docs(planning):`, separate from code.
- **2026-11-15 — Re-scope (found in 12.2.1)**: the parser keeps no parentheses node, so `v <- (fn(x) x)` takes the named-definition path (`src/vm.rs:140`) and stores into `functions`, not the environment. Fix in task 12.3.2, not the parser: an identifier that is not a variable resolves in `self.functions` and yields `R0Val::Fn`. Named and bound functions can then be passed as values.
- **2026-11-15 — Calls are expressions**: `Expr::Call` takes a callee expression instead of a name (task 12.3.1). This is the largest structural change in the plan.
- **Proposed (approve in Phase 6)**: workers `sonnet`, at most 2 concurrent (only where file scopes are disjoint, which the wave table guarantees), escalation per wave-orchestration's ladder.
- **Proposed (approve in Phase 6)**: acceptance per wave = targeted tests, then at wave end the serial suites `cargo test`, `cargo test --test gnu_r -- --test-threads=1`, and `cargo build --target wasm32-unknown-unknown --release`.

---

## Recon Log (facts from the repo, 2026-11-15)

- Baseline green: `cargo test`: lib 134, conformance 80, gnu_r 4. Branch `g12-transpiler`, clean except `planning/GOALS.md`.
- `src/vm.rs` 6885 lines; `src/value.rs` 405; `src/parser.rs` 440; `src/lexer.rs` 475; `src/ast.rs` (`Expr` at line 37); `src/transpile.rs`.
- `R0Val::` appears about 1012 times in `src/vm.rs`. Adding a variant may produce many non-exhaustive match errors. Use `cargo check` to find them; add the minimal arm (error or fallback) and nothing more.
- `UserFn` (`src/vm.rs:14`): `params`, `body`, `captured_env: HashMap<String, R0Val>` (private fields). `captured_env` is a snapshot taken at definition (`src/vm.rs:146` `captured_env: env.clone()`).
- Named function definition: `src/vm.rs:138-156` (`Expr::Assign` with `Expr::FnDef` value stores into `self.functions`). Anonymous: `src/vm.rs:396` errors.
- Call dispatch: `src/vm.rs:383-395` (`Expr::Call`, looks up `self.functions` by name, then intrinsics). `Expr::Call.func: String` at `src/ast.rs:69-72`.
- Parser: function definition at `src/parser.rs:284`; calls at `src/parser.rs:290` (`matches!(self.peek(), Tok::LParen)` on identifiers).
- Lexer: `"function" | "fn" => Tok::Fn` (`src/lexer.rs:239`); `\` rejected at `src/lexer.rs:355`.
- Transpiler: `Tok::Fn` → `function` (`src/transpile.rs:109`). So `\` lexed as `Tok::Fn` needs no transpiler code change.
- Display for `R0Val`: `src/value.rs:227`.
- Test infrastructure: `tests/conformance.rs` (80 tests); `tests/gnu_r.rs` (4 tests, needs `Rscript`, about 20s); corpus `tests/r0/` (90 files). `r0_transpiled_matches_gnu_r` selects `g12_*` by prefix (task 12.5.1 generalises it).
- No `docs/decisions/` directory: no ADRs to honour.
- Verified fact: named functions snapshot their environment today. `x <- 1; f <- fn() x; x <- 2; f()` gives 1 in R0 and 2 in GNU R.

---

## Wave & Dependency Table

| Wave | Task ID | Title | Files in Scope | Depends On |
|---|---|---|---|---|
| 1 | `12.1.1` | `R0Val` function variant (G24) | `src/value.rs` (edit), `src/vm.rs` (edit, minimal match arms and `UserFn` visibility) | — |
| 1 | `12.1.2` | Lexer `\` introducer and transpiler test (G25, G30) | `src/lexer.rs` (edit), `src/transpile.rs` (edit, test module only) | — |
| 2 | `12.2.1` | Unnamed function expressions evaluate to values (G26) | `src/vm.rs` (edit), `tests/conformance.rs` (edit) | `12.1.1`, `12.1.2` |
| 2 | `12.2.2` | Function values are not printable (G34) | `src/value.rs` (edit) | `12.1.1` |
| 3 | `12.3.1` | `Expr::Call` takes a callee expression (G27, part 1) | `src/ast.rs` (edit), `src/parser.rs` (edit), `src/vm.rs` (edit, call site only) | `12.2.1` |
| 4 | `12.3.2` | Calls through function values (G27, part 2) | `src/vm.rs` (edit), `tests/conformance.rs` (edit) | `12.3.1` |
| 5 | `12.4.1` | Closure scoping rule (G28) | `src/vm.rs` (edit), `tests/conformance.rs` (edit) | `12.3.2` |
| 6 | `12.5.1` | Corpus `g_anon_*` and gnu_r equivalence (G32) | `tests/r0/g_anon_*.R0` (create), `tests/gnu_r.rs` (edit) | `12.2.1`, `12.3.2`, `12.4.1`, `12.2.2` |
| 6 | `12.5.2` | Spec for anonymous functions, closures, relaxed Axiom 1 (G33) | `docs/LANGUAGE_SPEC.md` (edit) | `12.4.1`, `12.2.2` |

Disjointness check: every wave has pairwise-disjoint must-edit files. Wave 1: value.rs+vm.rs vs lexer.rs+transpile.rs. Wave 2: vm.rs+conformance.rs vs value.rs. Wave 6: corpus+gnu_r.rs vs LANGUAGE_SPEC.md. Waves 3–5 are one task each.

---

## Hazard Notes

- `R0Val` match sites (about 1012 `R0Val::` uses in `src/vm.rs`). A new variant can break exhaustive matches. Add the smallest arm that compiles and keep behaviour unchanged for existing variants.
- `UserFn` fields are private. `R0Val::Fn` needs `UserFn` visible from `src/value.rs`. Make the minimum `pub(crate)` change; do not restructure `UserFn`.
- `Expr::Call` change (12.3.1) touches parser and VM. Every existing call must still resolve identically; the full conformance suite is the regression gate.
- Named functions change behaviour in 12.4.1 (top-level live lookup). Conformance tests that pin the old snapshot result must be updated in that task and reported in STATE_12.md.
- `Rscript` tests (`cargo test --test gnu_r`) must run serially (`-- --test-threads=1`) at each wave end.
- The GNU R equivalence harness uses `tests/gnu_r/canon.R`. A corpus file must not end with a function value.

---

## Task Specs

### Task 12.1.1: `R0Val` function variant (G24)
- **id**: `12.1.1`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `src/value.rs`, `src/vm.rs` (minimal: `UserFn` visibility and any non-exhaustive match arms the compiler requires)
  - *do not touch*: `src/parser.rs`, `src/lexer.rs`, `src/ast.rs`, `src/transpile.rs`, `tests/`
- **depends-on**: none
- **anchors**:
  - `src/value.rs:91` (`pub enum R0Val`)
  - `src/vm.rs:14` (`pub struct UserFn`, private fields)
  - `src/value.rs:227` (`impl fmt::Display for R0Val`)
- **spec**: Add a variant `Fn(UserFnValue)` to `R0Val`. The payload is the function's params, body, and captured environment. Make `UserFn` (or a wrapper) usable from `value.rs` with `pub(crate)` field visibility. Derive or implement `Clone` and `PartialEq` as the existing variants do, or document why not. Add the minimal arms the compiler requires. Do not add evaluation behaviour in this task.
- **acceptance**:
  - `cargo check` exits 0.
  - `cargo test --lib` passes (no new failures).
  - `cargo build --target wasm32-unknown-unknown --release` exits 0.

### Task 12.1.2: Lexer `\` introducer and transpiler test (G25, G30)
- **id**: `12.1.2`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `src/lexer.rs`, `src/transpile.rs` (test module only)
  - *do not touch*: `src/vm.rs`, `src/value.rs`, `src/parser.rs`, `tests/`
- **depends-on**: none
- **anchors**:
  - `src/lexer.rs:239` (`"function" | "fn" => Tok::Fn`)
  - `src/lexer.rs:355` (`Unexpected character` for `\`)
  - `src/transpile.rs:109` (`Tok::Fn => pending_fn = true`)
- **spec**: Lex `\` as `Tok::Fn` when it is followed by `(` (after optional whitespace). Anywhere else, keep the existing "Unexpected character" error. Add lexer unit tests: `\(x) x + 1` lexes like `fn(x) x + 1`; a bare `\` not followed by `(` still errors. Add a transpiler unit test in `src/transpile.rs`: `transpile_to_r("f <- \\(x: dbl) -> dbl x + 1")` produces `function(x)` with the types stripped.
- **acceptance**:
  - `cargo test --lib lexer` passes.
  - `cargo test --lib transpile` passes.

### Task 12.2.1: Unnamed function expressions evaluate to values (G26)
- **id**: `12.2.1`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `src/vm.rs`, `tests/conformance.rs`
  - *do not touch*: `src/value.rs` (beyond the `12.1.1` variant), `src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `tests/gnu_r.rs`
- **depends-on**: `12.1.1`, `12.1.2`
- **anchors**:
  - `src/vm.rs:396` (`Expr::FnDef { .. } => Err("Anonymous functions must be bound...")`)
  - `src/vm.rs:138-156` (named definition: builds `UserFn`, stores in `self.functions`)
  - `src/vm.rs:146` (`captured_env: env.clone()`)
- **spec**: `Expr::FnDef` in value position evaluates to `R0Val::Fn` holding params, body, and a snapshot of `env` (the same capture as named functions today). Named definitions (`f <- fn(...)`) keep their current path unchanged. `list(f = fn(x) x + 1)` and `(fn(x) x)` no longer error. Add conformance tests: an unnamed function stored in a list, passed as an argument (as a variable), and returned from a `list` without error.
- **acceptance**:
  - `cargo test --test conformance` passes.
  - `cargo test --lib` passes.

### Task 12.2.2: Function values are not printable (G34)
- **id**: `12.2.2`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `src/value.rs`
  - *do not touch*: `src/vm.rs`, `tests/`
- **depends-on**: `12.1.1`
- **anchors**:
  - `src/value.rs:227` (`impl fmt::Display for R0Val`)
- **spec**: `Display` for `R0Val::Fn` writes a fixed placeholder, `<function>`. It does not print source. Any code path that formats a final result for printing (search `Display` use in `src/main.rs`) must still work. Add a unit test in `src/value.rs` asserting the placeholder.
- **acceptance**:
  - `cargo test --lib value` passes.
  - `cargo test --lib` passes.

### Task 12.3.1: `Expr::Call` takes a callee expression (G27, part 1)
- **id**: `12.3.1`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `src/ast.rs`, `src/parser.rs`, `src/vm.rs` (call site only)
  - *do not touch*: `src/value.rs`, `src/lexer.rs`, `src/transpile.rs`, `tests/`
- **depends-on**: `12.2.1`
- **anchors**:
  - `src/ast.rs:69-72` (`Expr::Call { func: String, args }`)
  - `src/parser.rs:290` (call parsing after an identifier)
  - `src/vm.rs:383-395` (call dispatch)
- **spec**: Change `Expr::Call.func` from `String` to a callee `Box<Expr>`. The parser parses a call on any postfix callee (`expr(args)`), so `f(1)` still produces `Call { func: Ident("f") }`. In the VM, an `Ident` callee dispatches exactly as before (`self.functions`, then intrinsics). Any other callee errors with "callee not supported yet" for now, so that 12.3.2 adds it. No change in behaviour for existing scripts. The full conformance suite is the regression gate.
- **acceptance**:
  - `cargo test --test conformance` passes (all 80+ existing tests unchanged).
  - `cargo test --lib` passes.

### Task 12.3.2: Calls through function values (G27, part 2)
- **id**: `12.3.2`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `src/vm.rs`, `tests/conformance.rs`
  - *do not touch*: `src/ast.rs`, `src/parser.rs`, `src/value.rs`, `tests/gnu_r.rs`
- **depends-on**: `12.3.1`
- **anchors**:
  - `src/vm.rs:383-395` (call dispatch, now on a callee expression)
  - `src/vm.rs:478` (`call_user`)
- **spec**: Evaluate the callee expression. If it yields `R0Val::Fn`, call it through `call_user`-style binding (same argument matching and defaults as named functions). Supported callee forms: an identifier bound to a function value, a list element (`l$f`, `l[["f"]]`), an immediate call `(fn(x) x)(2)`, and a call result `make(2)(3)`. Non-function callees error as "not a function". Add conformance tests for each form.
- **acceptance**:
  - `cargo test --test conformance` passes.
  - `cargo test --lib` passes.

### Task 12.4.1: Closure scoping rule (G28)
- **id**: `12.4.1`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `src/vm.rs`, `tests/conformance.rs`
  - *do not touch*: `src/value.rs`, `src/parser.rs`, `src/ast.rs`, `tests/gnu_r.rs`, `docs/`
- **depends-on**: `12.3.2`
- **anchors**:
  - `src/vm.rs:14-18` (`UserFn`, `captured_env` comment "Immutable closure capture")
  - `src/vm.rs:138-156` (named definition snapshot)
  - `src/vm.rs:478` (`call_user`, builds `local_env` from `captured_env`)
- **spec**: Implement the decision-log scoping rule. A function defined at top level has no snapshot: at call time its free variables resolve in the current global environment. A function defined inside another function snapshots the enclosing frame at creation (the existing behaviour). Parameters and locals always shadow. Named top-level functions change behaviour: `x <- 1; f <- fn() x; x <- 2; f()` now returns 2. Find and update the conformance tests that pin the old result, and report each one in the task report. Add tests: the late-reassignment case, `make <- fn(k) fn(x) x + k` with `make(2)(3) == 5`, and parameters shadowing globals.
- **acceptance**:
  - `cargo test --test conformance` passes.
  - `cargo test --lib` passes.

### Task 12.5.1: Corpus and GNU R equivalence (G32)
- **id**: `12.5.1`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *can create*: `tests/r0/g_anon_*.R0` (corpus files, each a few statements)
  - *must edit*: `tests/gnu_r.rs`
  - *do not touch*: `src/`, `docs/`, `tests/conformance.rs`
- **depends-on**: `12.2.1`, `12.2.2`, `12.3.2`, `12.4.1`
- **anchors**:
  - `tests/gnu_r.rs` (`r0_transpiled_matches_gnu_r`, which selects files by the `g12_` prefix)
  - `tests/gnu_r/canon.R`
- **spec**: Add corpus files covering: unnamed `fn`, `function`, and `\` forms; a closure built by a factory; a list holding functions, called by `l$f(...)`; a function passed as an argument; a default argument on an anonymous function; and a top-level late-reassignment case. Each file's last expression is a plain value (never a function). Generalise `r0_transpiled_matches_gnu_r` to select the `g12_` and `g_anon_` prefixes. Keep `r0_corpus_matches_gnu_r` skipping both. An R0 error on any of these files is a failure, not a skip.
- **acceptance**:
  - `cargo test --test gnu_r -- --test-threads=1` passes.

### Task 12.5.2: Spec for anonymous functions, closures, relaxed Axiom 1 (G33)
- **id**: `12.5.2`
- **repo**: `/home/andrew/AI/rzero`
- **files-in-scope**:
  - *must edit*: `docs/LANGUAGE_SPEC.md`
  - *do not touch*: `src/`, `tests/`, `planning/`
- **depends-on**: `12.4.1`, `12.2.2`
- **anchors**:
  - `docs/LANGUAGE_SPEC.md:5` (the "Valid R0 programs use standard R syntax" line)
  - `docs/LANGUAGE_SPEC.md:13` (Axiom 1)
  - `docs/LANGUAGE_SPEC.md:70-80` (§3 and §3.2 "Declaration", the "must be bound" rule)
- **spec**: Rewrite Axiom 1 to the relaxed contract (see GOALS.md G33 wording). Update §1's syntax line to list the R0-only forms: `fn`, `\(...)`, `: type`, `-> type`. Rewrite §3.2 "Declaration" for anonymous functions, the scoping rule (top-level live, nested snapshot), and function values (returnable, storable, passable). Add a restriction: a nested closure does not see later changes to its enclosing frame's variables. Add: function values cannot be printed (`<function>`); canonical comparison of function values is undefined. Keep the existing text for other sections unchanged.
- **acceptance**:
  - `grep -n "must be bound" docs/LANGUAGE_SPEC.md` returns no match.
  - `grep -n "Axiom 1" docs/LANGUAGE_SPEC.md` shows the relaxed wording.
  - `cargo test --test gnu_r -- --test-threads=1` still passes (no code changed).

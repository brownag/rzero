# EXECUTION PLAN 1 -- R0 (R-Zero) r0-wasm scaffold, extraction, and conformance

Orchestration 1. Plan artifacts: `EXECUTION_PLAN_1.md`, `PROMPT_TEMPLATE_1.md`, `STATE_1.md`, `tasks/1.W.id.todo.md`.
Repo: `/home/andrew/AI/rzero` (single repo, branch `master`, no commits yet).

---

## Goal & Scope

### Goal
Turn the R0 design conversation in `planning/INITIAL_SPEC.md` into a working zero-dependency Rust crate `r0-wasm` at the repo root. The crate has:
- the two spec documents extracted into `docs/`,
- a modularized lexer, parser, and vector evaluator for the R0 prototype,
- a `cargo test` conformance suite,
- a `wasm32-unknown-unknown` release build with linear-memory FFI exports.

### In scope (from `planning/INITIAL_PROMPT.md`)
- **Phase 1:** `Cargo.toml` (edition 2021, `crate-type = ["cdylib", "rlib"]`, release profile `opt-level="z"`, `lto=true`, `codegen-units=1`, `panic="abort"`). Extract the Language Spec to `docs/LANGUAGE_SPEC.md` and the Architecture Spec to `docs/ARCHITECTURE.md`.
- **Phase 2:** modules `lib.rs`, `main.rs`, `value.rs`, `lexer.rs`, `ast.rs`, `parser.rs`, `vm.rs`, `isa.rs`. Layout as the prompt specifies.
- **Phase 3:** `tests/conformance.rs` covering the 7 groups: newline suppression, pipe desugaring, Kleene logic, strict broadcasting, 1-based indexing and OOB, pedon/approx integration, fuel metering.
- **Phase 4:** Wasm FFI in `lib.rs`; `cargo build --target wasm32-unknown-unknown --release`; report exact `.wasm` byte size; `cargo test`; `cargo run --release`; `cargo clippy -- -D warnings`.

### Out of scope (explicit deferrals)
- `arena.rs`, `compile.rs`, `simd_kernels.rs` from Architecture §7. The prompt's layout omits them. Deferred to a later orchestration. The architecture doc still describes them.
- Strip-mined tile VM, SSA bytecode codegen, FUSE_TILE execution, `TILE_*` kernels running. `isa.rs` is layout and encoding only (decision D5).
- `R0Vec` used by the evaluator. Only the struct, its size/alignment asserts, and a round-trip test are in scope (D5).
- Arrow validity bitmaps (`null_ptr` mode), zero-copy host arena, and `r0_alloc_pinned` pinned-arena semantics. Documented in ARCHITECTURE only.
- Spec's full 48-or-69-name locked prelude. Only the intrinsics the prototype implements are in scope, plus named-argument binding they need (D15). Remaining names stay documented as unimplemented.
- Out-of-memory trap (`R0Trap::OutOfMemory`). Not in the prompt's trap-code list.
- Size optimization to the 200 KB target (report only, D11). No `wasm-opt`; `wasm-opt` is not installed.
- Any push, PR, or remote action. CLAUDE.md forbids remote writes.

### Verbatim constraints (quoted from INITIAL_PROMPT.md)
- "Do not use external crates in the core library (`zero-dependency` pure Rust)."
- "Zero external runtime dependencies in `Cargo.toml` (`std` is fine for the initial host/Wasm prototype, or `core`+`alloc` where practical)."
- "Do not water down or omit any semantics from the conversation prototype."
- "Keep error messages deterministic and prefixed with trap codes (`ErrDimensionMismatch`, `ErrInvalidCondition`, `ErrMixedSubscript`, `ErrBoundsTrap`, `ErrFuelExhausted`)."
- "committing cleanly after each milestone."
- "`cargo clippy -- -D warnings` (clean with zero warnings)"
- "If `wasm32-unknown-unknown` target is installed, build the release `.wasm` binary and report its exact byte size."

### Mapping check (Phase 1 step 8)
Every sentence of INITIAL_PROMPT maps to a task, a decision, or an out-of-scope line. The only unmapped items were spec-only subsystems (arena, compile, SIMD kernels, Arrow bitmaps). Those are now explicit deferrals above.

---

## Recon log (facts from the repo and the environment)

| Fact | Evidence | Consequence |
|---|---|---|
| Repo has no commits; branch `master`; untracked `planning/` | `git status`, `git log` fails on empty branch | Baseline = nothing built. First commit comes from task 1.1.1. |
| No `Cargo.toml`, `src/`, `docs/`, `tests/`, `AGENTS.md`, `docs/decisions/` | `ls -la`, `find` | No ADR constraints. No project conventions file: use Rust 2021 defaults. |
| Toolchain: cargo/rustc 1.99.1, clippy 0.1.99 | `cargo --version` etc. | Clippy gate is available. |
| `wasm32-unknown-unknown` target installed | `rustup target list --installed` | Phase 4 wasm build is runnable. |
| Node v22.23.5 at `/usr/bin/node`; no `wasm-opt` | `command -v` | Wasm exports can be smoke-tested in Node without adding JS files. |
| `planning/INITIAL_SPEC.md` is 2409 lines, a transcript export | `wc -l` | Extraction needs repair (E1-E4). Line anchors below refer to it. |
| Spec prototype does not compile as written | `rustc`: `E0507 cannot move out of *op` (BinOp not Copy) | Port adds `Copy` to `BinOp` (E7). |
| Spec pedon script traps as written | Run: `Right-hand side of '|>' pipe must be a function call` | Rewrite the script to `sum(...) / total_thick` (E5). |
| Spec's printed pedon outputs don't match its own data | Run prototype with E5 + E7 fixes: 25.666, 21.7381, 29.0526, 28.7059; hand-checked | Tests assert computed values (decision D13). Spec printed numbers listed as E6. |
| Prototype intrinsics ignore named args except `na.rm` | `eval_call`, `pos_args` filter | `approx(..., xout = _)` cannot bind; named binding added (E8, D15). |
| Prototype builds `list()` and `data.frame()` as one variant | `"data.frame" \| "list"` arm | Split into distinct types (D12, E10). |

---

## Decision log

Every line is dated 2026-11-14. Each decision was put to the user and answered, unless marked **default**.

- **D1 Crate location:** repo root (`/home/andrew/AI/rzero`), not an `r0-wasm/` subfolder. `planning/` sits beside it. Answered: repo root.
- **D2 Spec extraction:** repair broken spans in place (E1-E4), and log each repair here, not in the docs. Answered: "fix anything that is messed up." **Default:** repairs are recorded in each worker's `STATE_1.md` report.
- **D3 Typed subsetting:** subsetting, `x[[i]]`, OOB NA, and subset assignment preserve Dbl/Int/Lgl/Chr element types as spec §3.4 requires. OOB NA is `NA of typeof(x)`. Answered: "Preserve types."
- **D4 std, not no_std:** the crate uses `std`. `f64::floor`/`ln`/`powf` and `HashMap` are not in `core` without libm, and zero-dependency forbids libm. Answered: std.
- **D5 ABI/ISA depth:** `R0Vec` gets its struct, size/align asserts, and a round-trip test. `isa.rs` gets the opcode enum, encoding helpers, and `TileKernel` names, with no codegen. Arena, compile, and SIMD kernels are deferred. Answered: "Layout + stubs."
- **D6 Wasm FFI:** linear-memory trio: `r0_alloc(len)`, `r0_free(ptr,len)`, `r0_eval(ptr,len) -> i32`, plus `r0_result_ptr()` and `r0_result_len()`. Answered: "Linear-memory trio."
- **D7 Plan tracking:** planning artifacts are tracked, committed in separate `docs(planning):` commits, never mixed into code commits. Answered: "Track."
- **D8 Parallelism:** at most 2 workers per wave, only with disjoint files. Shared `vm.rs` and `tests/conformance.rs` run serially. Answered: "Max 2, mostly serial."
- **D9 Models:** orchestrator on the frontier model. Workers run `claude-haiku-5-5`. A task that fails verification twice re-runs on `claude-sonnet-5-5`. Answered: "Haiku, escalate to Sonnet."
- **D10 Gates:** each wave runs `cargo test` and `cargo clippy --all-targets -- -D warnings`, serially. The wasm build and size report run once, in wave 13. Answered: "test+clippy each wave, wasm last."
- **D11 Size budget:** report only. The 200 KB target from the Architecture preamble is a note, not a gate. Answered: "Report only."
- **D12 list vs df:** distinct types. `list()` has no equal-length rule. `data.frame()` enforces equal-length columns. `$` works on both. Answered: "Distinct types."
- **D13 Pedon expectations:** tests assert computed values: 25.666 (0-100 cm), approx 21.7381 / 29.0526 / 28.7059. Tolerance 1e-3. Answered: "Computed values."
- **D14 Clippy command (default):** the per-wave and final gate runs `cargo clippy --all-targets -- -D warnings`. That is stricter than the prompt's `cargo clippy -- -D warnings`, which it includes. Final report states both pass.
- **D15 Named intrinsic args (default):** intrinsics bind named arguments by name, e.g. `approx(xs, ys, xout = ...)`. `na.rm` keeps its current behavior. Needed by E9.
- **D16 Commit cadence (default):** one local Conventional Commit per task, after its targeted acceptance passes. Per wave, the orchestrator runs the serial suite. No pushes. Subjects carry no wave or task numbers.
- **D17 `%in%` (default, corrected):** the prompt says the parser performs "`%in%` desugaring", and spec §3.1 says `x %in% table` → `Intrinsic(InSet, [x, table])`. The parser rewrites `x %in% table` to the intrinsic call `%in%`(x, table). The VM implements that intrinsic with the prototype's NA-aware set-membership logic. Spec E12 records the difference from the prototype's `BinOp::InOp`.
- **D18 Pipe example (default):** prompt's `x |> approx(y, xout = _)` is shorthand. It supplies only two positional arguments, and `approx` needs knots and values. The test uses `x |> approx(xs, ys, xout = _)`. Recorded as E9.
- **D19 `sum(lgl)` (default):** returns Int, per spec §6.4 ("count TRUE as 1L"). `mean(lgl)` returns Dbl. The prototype returned Dbl (E11).
- **D20 Supersession (ADR):** no ADRs exist in `docs/decisions/`. No supersession applies.

---

## Errata (spec repairs; each task reports its own, these are the known ones)

- **E1 Broadcasting "Case 3" truncated** (INITIAL_SPEC.md lines 166-168). Restore: if LA ≠ LB and both LA > 1 and LB > 1, trap `ErrDimensionMismatch`.
- **E2 Kleene NOT SIMD block broken** (lines 674-681). Contains "wait:" self-correction and a wrong formula. Replace with the NOT_LUT / `i8x16_swizzle` formula that matches `KLEENE_NOT_LUT = [1,0,2,0]`.
- **E3 ISA diagram garbled** (lines 565-571). Replace with an explicit bit-field table: opcode[31:24], A[23:16], B[15:8], C[7:0].
- **E4 Prelude count** (heading line 306 says "48 core intrinsics"; the listed names total about 69 by manual count). Correct the heading to the count the doc actually lists. The worker verifies the count and reports it.
- **E5 Pedon script precedence trap** (line 2256: `(…) |> sum() / total_thick`). Under spec §4 `/` binds tighter than `|>`. Rewrite as `sum(valid_thick * hz$clay[valid]) / total_thick`.
- **E6 Printed pedon outputs** (lines 2292-2296: 25.661, 22.0208, 29.0127, 28.6095, fuel 99854). Replace with computed outputs: 25.666, 21.7381, 29.0526, 28.7059, fuel 99809, and show `list` not `<R0 df>` (D12).
- **E7 Prototype does not compile:** `BinOp` must derive `Copy` (line ~1210).
- **E8 Named args ignored:** `eval_call` only handles `na.rm`. Fixed by D15.
- **E9 Prompt pipe example** (INITIAL_PROMPT line 46): `x |> approx(y, xout = _)` lacks knots. Test form per D18.
- **E10 list/df conflated.** Fixed by D12.
- **E11 sum(lgl) returns Dbl.** Fixed by D19.
- **E12 `%in%` form** (spec §3.1 says `Intrinsic(InSet)`; prototype uses `BinOp::InOp`). Port follows the spec and the prompt's "%in% desugaring" (D17).
- **E14 `c()` rejects NA with numbers** (found by 1.8.1, `pipe_implicit_first_arg`). `NA` lexes as Lgl, so `c(1, NA, 3)` gives TypeError. Spec §3.2 forbids implicit logical coercion only in arithmetic, and §6.1 says `c()` is numeric-promotable. Fix (task 1.8.1a): in `c()`, an all-NA Lgl argument promotes to the target numeric type (Dbl or Int NA). Non-NA TRUE/FALSE mixed with numbers stays TypeError.

---

## Wave table

Max 2 tasks per wave. Tasks in one wave touch disjoint files. Waves run in order.

| Wave | Tasks (parallel) | Depends on | Files touched |
|---|---|---|---|
| W1 | 1.1.1 scaffold; 1.1.2 LANGUAGE_SPEC | -- | Cargo.toml, src/*.rs stubs, .gitignore; docs/LANGUAGE_SPEC.md |
| W2 | 1.2.1 value.rs; 1.2.2 ARCHITECTURE | W1 | src/value.rs; docs/ARCHITECTURE.md |
| W3 | 1.3.1 lexer.rs; 1.3.2 ast.rs | W2 | src/lexer.rs; src/ast.rs |
| W4 | 1.4.1 parser.rs; 1.4.2 isa.rs | W3 | src/parser.rs; src/isa.rs |
| W5 | 1.5.1 vm core | W4 | src/vm.rs |
| W6 | 1.5.2 vm indexing | W5 | src/vm.rs (serial after W5) |
| W7 | 1.5.3 vm intrinsics + named args | W6 | src/vm.rs (serial) |
| W8 | 1.6.1 lib + main; 1.8.1 conformance C1 | W7 | src/lib.rs, src/main.rs; tests/conformance.rs (new) |
| W9 | 1.7.1 wasm FFI; 1.8.2 conformance C2 | W8 | src/lib.rs; tests/conformance.rs (serial after W8) |
| W10 | 1.8.3 conformance C3 indexing | W9 | tests/conformance.rs |
| W11 | 1.8.4 conformance C4 pedon | W10 | tests/conformance.rs |
| W12 | 1.8.5 conformance C5 fuel | W11 | tests/conformance.rs |
| W13 | 1.9.1 gate: wasm build, size, docs sync | W12 | docs/ARCHITECTURE.md; no code |

**Disjointness check:** W1-W4 pairs share no files. W5-W7 share `vm.rs`, so they are serial despite the wave split (each is its own wave). W8's pair is lib+main vs tests. W9's pair is lib vs tests, and both lib and tests are serial with W8 by wave number. W10-W12 are serial because they share `tests/conformance.rs`.

---

## Task specs

Fields: id, repo, files-in-scope (must edit / can create), depends-on, spec prose, anchors, constraints, acceptance.

Repo for every task: `rzero` (`/home/andrew/AI/rzero`).
Scratch folder for workers: `.development/` (added to `.gitignore` in 1.1.1; never committed).
Spec anchors refer to `planning/INITIAL_SPEC.md` by line number. Prototype code: lines 735-2290. Grep the anchor text rather than reading whole files.

### 1.1.1 Scaffold crate

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: none. Can create: `Cargo.toml`, `Cargo.lock` (generated), `.gitignore`, `src/lib.rs`, `src/main.rs`, `src/value.rs`, `src/lexer.rs`, `src/ast.rs`, `src/parser.rs`, `src/vm.rs`, `src/isa.rs`.
- **Depends on:** none.
- **Spec:**
  - `Cargo.toml`: `[package] name = "r0-wasm"`, `version = "0.1.0"`, `edition = "2021"`, no `[dependencies]` section. `[lib] crate-type = ["cdylib", "rlib"]`. `[profile.release] opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`.
  - `src/lib.rs`: crate doc comment, then `pub mod value; pub mod lexer; pub mod ast; pub mod parser; pub mod vm; pub mod isa;` Every module is declared here, once, so later tasks never edit `lib.rs` for module wiring.
  - Each module file is a stub: a `//!` doc line naming the module. No code.
  - `src/main.rs`: `fn main() {}` with a `//!` doc comment. Filled in by 1.6.1.
  - `.gitignore`: lines `/target` and `.development/`.
- **Anchors:** none (scaffolding, no spec code).
- **Constraints:** no `[dependencies]` of any kind. Do not create `docs/` or `tests/` (other tasks own them).
- **Acceptance:** `cargo build` and `cargo clippy --all-targets -- -D warnings` both pass. Then local commit (D16).

### 1.1.2 Extract LANGUAGE_SPEC

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: none. Can create: `docs/LANGUAGE_SPEC.md`.
- **Depends on:** none.
- **Spec:** source is `planning/INITIAL_SPEC.md` lines 111-394 (Language Specification v0.1 through §7 Side-by-Side). Copy §1-§7 (and the Side-by-Side example) into `docs/LANGUAGE_SPEC.md`, keeping the EBNF (lines 240-302), precedence table (220-238), broadcasting rules (156-168), indexing rules (194-218), and intrinsics (304-368).
  - Repairs (D2): E1 (lines 166-168 Case 3). E4 (heading line 306 count). Report every repair in your STATE report, with original and corrected text.
  - Strip transcript noise: the "**User prompt:**" and "**Response:**" lines, and the Gemini URL. Keep headings.
  - Title: `# R0 Language Specification v0.1`. Add one status note under it: intrinsics beyond the prototype set are specified but not yet implemented (D15).
- **Anchors:** `grep -n "Formal EBNF Grammar"`, `grep -n "Core Intrinsics Table"`, `grep -n "Strict Broadcasting"`.
- **Constraints:** touch only `docs/LANGUAGE_SPEC.md`. Preserve the EBNF text exactly except for E1/E4.
- **Acceptance:** `grep -q "Formal EBNF Grammar" docs/LANGUAGE_SPEC.md && grep -q "Program *=" docs/LANGUAGE_SPEC.md && grep -q "Strict Broadcasting" docs/LANGUAGE_SPEC.md`. Worker reports the counted number of intrinsic names.

### 1.2.1 value.rs

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/value.rs`. Can create: none.
- **Depends on:** 1.1.1.
- **Spec:** port prototype lines 735-904 (NA sentinels, Kleene LUTs, `R0Val`, `materialize_int`, `promote_dbl`, `Display`).
  - Keep: `NA_REAL_BITS = 0x7ff8_0000_0000_07a2`, `NA_INT = i32::MIN`, `Lgl = u8` with `LGL_FALSE=0`, `LGL_TRUE=1`, `LGL_NA=2`, `KLEENE_AND_LUT`, `KLEENE_OR_LUT`, `KLEENE_NOT_LUT`, `is_na_real`, `na_real`.
  - `R0Val` variants: `Dbl(Vec<f64>)`, `Int(Vec<i32>)`, `Lgl(Vec<Lgl>)`, `Chr(Vec<Option<String>>)`, `Range { start, end, step }`, `Df { names, cols }`, **`List { names, cols }` (D12, new)**, `Null`.
  - Add `R0Val::df(names, cols) -> Result<R0Val, String>`: enforces equal column lengths, returns `"ErrDimensionMismatch: data.frame column 'x' has length N, expected M"`. `List` has no such constructor check.
  - Add helper `R0Val::na_of_type_like(&self, n: usize) -> R0Val` producing `n` NAs of the same variant (D3, OOB NA).
  - **`R0Vec` (D5):** `#[repr(C, align(16))] pub struct R0Vec { pub tag_flags: u32, pub len: u32, pub payload: u64 }`. Add `const _: () = assert!(size_of::<R0Vec>() == 16); const _: () = assert!(align_of::<R0Vec>() == 16);`. Define the tag bits from spec §2.1 (`TAG_DBL..TAG_DF` in bits 0..3, `IS_SCALAR_UNBOXED` bit 4, `IS_HOST_PINNED` bit 5, `HAS_NULL_BITMAP` bit 6, refcount in bits 16..31) as constants. Add an `R0Vec::tag(&self)` accessor only. No constructors, no allocation code.
  - Fix `Display` for `List` and `Df` (`<R0 list: N fields>` vs `<R0 df: R rows x C cols>`).
- **Anchors:** `grep -n "pub const NA_REAL_BITS"`, `grep -n "pub enum R0Val"`, `grep -n "impl fmt::Display for R0Val"`. Spec §2.1 R0Vec: `grep -n "R0Vec" planning/INITIAL_SPEC.md`.
- **Reference implementation:** prototype lines 738-904 (`value` block).
- **Constraints:** do not touch lexer, parser, vm, or isa. Keep `is_na_real` as `is_nan()` (prototype behavior); note it in Notes.
- **Acceptance:** `cargo test --lib value::` and `cargo clippy --all-targets -- -D warnings`. Unit tests must include: `R0Vec` size and align are 16; a df with unequal columns returns the exact `ErrDimensionMismatch` text; `KLEENE_NOT_LUT` maps 0->1, 1->0, 2->2.

### 1.2.2 ARCHITECTURE doc

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: none. Can create: `docs/ARCHITECTURE.md`.
- **Depends on:** 1.1.1 (only for the module list in §7).
- **Spec:** source is `planning/INITIAL_SPEC.md` lines 398-718 (Architecture §1-§7). Copy the full architecture spec into `docs/ARCHITECTURE.md`. Preserve: memory-layout diagram (425-435), `R0Vec` struct (441-455), FUSE_TILE opcode sequence (537-551), ISA families (573-621), v128 notes (627-689), sandboxing (691-699).
  - Repairs: E2 (lines 674-681 "wait:" block: replace with the NOT_LUT swizzle formula). E3 (lines 565-571 ISA diagram: bit-field table). Report each repair in STATE.
  - §7 module layout: replace the listed tree with the layout as built (the prompt's layout), and mark `arena.rs`, `compile.rs`, `simd_kernels.rs` as "Deferred (not in this plan)".
  - Strip transcript noise (User prompt/Response lines).
- **Anchors:** `grep -n "Architectural Decision"`, `grep -n "FUSE_TILE_BEGIN"`, `grep -n "Core Opcode Families"`, `grep -n "Crate Module Layout"`.
- **Constraints:** touch only `docs/ARCHITECTURE.md`.
- **Acceptance:** `grep -q "FUSE_TILE_BEGIN" docs/ARCHITECTURE.md && grep -q "R0Vec" docs/ARCHITECTURE.md && ! grep -q "wait:" docs/ARCHITECTURE.md`.

### 1.3.1 lexer.rs

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/lexer.rs`. Can create: none.
- **Depends on:** 1.2.1 (`Lgl`, `LGL_*`, `na_real`, `NA_INT`).
- **Spec:** port prototype lines 905-1205 (`Tok`, `suppresses_newline`, `tokenize`). Keep the newline-suppression rules exactly: suppressed after binary operators, commas, `(`, `[`, `[[`, `{`, `else`; newlines inside `(` or `[` depth > 0 are suppressed; `;` and statement newlines emit `Term`. Keep `fn` and `function` as aliases. `_` is `Placeholder`. `NaN` and `NA_real_` map to `na_real()`, `Inf` to `f64::INFINITY`.
- **Anchors:** `grep -n "pub fn tokenize"`, `grep -n "fn suppresses_newline"`.
- **Reference implementation:** prototype lines 905-1205.
- **Constraints:** `tokenize` returns `Result<Vec<Tok>, String>`. Error texts stay plain and deterministic.
- **Acceptance:** `cargo test --lib lexer::` (unit tests: `1 +\n2` yields no `Term`; `(1\n+2)` yields no `Term`; `x <- 1\ny` yields one `Term`).

### 1.3.2 ast.rs

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/ast.rs`. Can create: none.
- **Depends on:** 1.2.1 (`Lgl`).
- **Spec:** port prototype lines 1207-1240 (`BinOp`, `UnaryOp`, `Expr`). `BinOp` derives `Clone, Copy, Debug, PartialEq` (E7). Keep every variant.
- **Anchors:** `grep -n "pub enum BinOp"`, `grep -n "pub enum Expr"`.
- **Constraints:** data only, no logic.
- **Acceptance:** `cargo build` and `cargo clippy --all-targets -- -D warnings`.

### 1.4.1 parser.rs

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/parser.rs`. Can create: none.
- **Depends on:** 1.3.1, 1.3.2.
- **Spec:** port prototype lines 1241-1527 (`Parser`, `parse_program`, `parse_block`, `parse_expr`, `parse_prefix`, `postfix_bp`, `infix_bp`, `desugar_pipe`).
  - Verify the binding-power table against spec §4 (`grep -n "Operator Precedence"`). Record any deviation in Notes.
  - `desugar_pipe`: if any argument, named or positional, is `_`, replace it with the LHS. Otherwise insert the LHS first. RHS that isn't a call is an error, exact text: `"Right-hand side of '|>' pipe must be a function call"`.
  - `x %in% table` desugars to the intrinsic call `%in%`(x, table), not a binary operator (D17). Remove `Tok::InOp` from the binary-operator match arm.
- **Anchors:** `grep -n "fn infix_bp"`, `grep -n "fn desugar_pipe"`.
- **Reference implementation:** prototype lines 1241-1527.
- **Constraints:** do not touch the lexer or AST. Parse errors stay plain text.
- **Acceptance:** `cargo test --lib parser::`. Unit tests: `1 + 2 * 3` tree shape; `-2^2` equals `-(2^2)`; `x |> f(y)` becomes `f(x, y)`; `x |> f(y, z = _)` replaces the placeholder.

### 1.4.2 isa.rs

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/isa.rs`. Can create: none.
- **Depends on:** 1.1.1 (no code dependencies).
- **Spec:** Architecture §4 (`grep -n "Core Opcode Families" planning/INITIAL_SPEC.md`, lines 561-621). Implement:
  - `#[repr(u8)] pub enum Opcode` with discriminants for every opcode the spec names explicitly. Family wildcards like `TILE_ADD_*` expand to `_VV`, `_VS`, `_SV` variants. Record the final count in STATE (spec says "~64").
  - `pub enum TileKernel` for the kernel names in §4 0x20-0x3F (ADD, SUB, MUL, DIV, PMIN, PMAX, POW, EXP, LOG, SQRT, ABS, ROUND, CMP_EQ, CMP_LT, CMP_LE, AND_LGL, OR_LGL, NOT_LGL, IS_NA, IFELSE).
  - `pub struct Instr(pub u32)` with `new(op, a, b, c)`, `opcode()`, `a()`, `b()`, `c()`, `bx() -> u16` (B and C combined). Encoding per the corrected bit-field table (E3): opcode[31:24], A[23:16], B[15:8], C[7:0].
  - `impl TryFrom<u8> for Opcode` rejects undefined discriminants.
- **Anchors:** `grep -n "Core Opcode Families" planning/INITIAL_SPEC.md` (lines 561-621); bit-field fix E3 (lines 565-571).
- **Constraints:** no codegen, no execution. Layout and encoding only (D5).
- **Acceptance:** `cargo test --lib isa::` (round-trip encode/decode; undefined byte rejected; a discriminant falls in the family range it belongs to).

### 1.5.1 vm core

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/vm.rs`. Can create: none.
- **Depends on:** 1.2.1, 1.4.1.
- **Spec:** port the core of prototype lines 1529-1886: `R0Vm` (globals, functions, fuel), `consume_fuel`, `eval_script`, `eval`, `eval_val`, `extract_scalar_cond`, `broadcast_len`, `eval_binary`. Literals, identifiers, blocks, `<-`/`=` assignment to identifiers, function definitions, `if`/`while`/`for`, `return`, `&&`/`||` short-circuit, unary `-`/`+`/`!`, numeric arithmetic, comparisons, Kleene `&`/`|` via LUT.
  - Fuel: `consume_fuel` errors with the exact prefix `ErrFuelExhausted:`. Default fuel is 100_000 (prototype `R0Vm::new(100_000)`).
  - Strict broadcasting: `broadcast_len` returns `ErrDimensionMismatch: ...` per E1 (spec §3.1 Case 3 restored: LA≠LB, both >1 -> trap).
  - Control flow: `if`/`while` condition must be length-1 and non-NA, else `ErrInvalidCondition:`.
  - **Stubs (replaced by 1.5.2 and 1.5.3, D5 layering):** `Expr::Index1`, `Expr::IndexScalar`, `Expr::Field`, index assignment, and `Expr::Call` dispatch are wired to `fn subset`, `fn index_scalar`, `fn field`, `fn subset_assign`, `fn call_intrinsic`, each returning `Err("ErrNotImplemented: <name>")` in this task only.
- **Anchors:** `grep -n "pub struct R0Vm"`, `grep -n "fn broadcast_len"`, `grep -n "fn eval_binary"`.
- **Reference implementation:** prototype lines 1529-1886.
- **Constraints:** do not touch `parser.rs`, `lexer.rs`, or `value.rs`. Do not implement subsetting here (that is 1.5.2).
- **Acceptance:** `cargo test --lib vm::`. Unit tests: fuel halts `while (TRUE) { x <- 1 }` with `ErrFuelExhausted`; `1 + c(10, 20)` succeeds; `c(1,2,3,4) + c(10,20)` returns `ErrDimensionMismatch`; `FALSE & NA` is FALSE; `TRUE & NA` is NA.

### 1.5.2 vm indexing

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/vm.rs`. Can create: none.
- **Depends on:** 1.5.1 (same file, serial).
- **Spec:** replace the three `ErrNotImplemented` stubs for subsetting with type-preserving code (D3, D12). Reference: prototype lines 1888-1977 (`eval_subset`, `subset_assign`), 1681-1710 (`IndexScalar`, `Field`), 1620-1633 (index assignment).
  - `x[i]`: positive indices (OOB or NA -> NA of typeof(x)); logical mask (length N or 1; TRUE selects, NA yields NA of typeof(x), length mismatch -> `ErrDimensionMismatch:`); negative drop; mixed positive/negative or NA with negative -> `ErrMixedSubscript:`.
  - `x[[i]]`: length-1, 1 ≤ i ≤ N, non-NA; else `ErrBoundsTrap:`. Returns unboxed same type. For `Range`, returns Int.
  - Range subsetting returns Int.
  - `x$name`: works on `Df` and `List`. Unknown name -> `Unknown column '$name'`.
  - Subset assignment `x[mask] <- v` and `x[idx] <- v`: keeps the target's type; promotes Int target to Dbl when assigning Dbl (spec §3.2). Assigning incompatible types (e.g., Lgl into Dbl) traps with `TypeError:`. RHS length must be 1 or match.
- **Anchors:** `grep -n "fn eval_subset"`, `grep -n "fn subset_assign"`.
- **Constraints:** `vm.rs` only. Do not change intrinsics (1.5.3).
- **Acceptance:** `cargo test --lib vm::`. Unit tests: `x[99L]` on Dbl is NA Dbl; `(1:3)[99L]` is NA Int; `x[-2L]` drops; `x[c(1,-2L)]` traps `ErrMixedSubscript`; `x[[5L]]` on length 3 traps `ErrBoundsTrap`; `x[x < 0] <- 0` assigns.

### 1.5.3 vm intrinsics + named args

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/vm.rs`. Can create: none.
- **Depends on:** 1.5.2 (same file, serial).
- **Spec:** replace the `ErrNotImplemented` stub in `call_intrinsic` with the prototype's intrinsic set (prototype lines 1978-2235). Implemented: `c`, `length`, `seq_len`, `seq_along`, `is.na`, `which`, `ifelse`, `pmin`, `pmax`, `abs`, `sqrt`, `exp`, `log`, `log10`, `floor`, `ceiling`, `sum`, `mean`, `min`, `max`, `cumsum`, `diff`, `approx`, `data.frame`, `list`.
  - **Named args (D15, E8):** bind named arguments by name for intrinsics (`approx`'s `xout`, `data.frame`/`list` names, `na.rm`). Unknown named arg for an intrinsic -> `UnknownArgument: <name> in <fn>`.
  - `%in%` intrinsic (target of the 1.4.1 desugar, D17): set membership, prototype NA behavior (NA matches NA). Returns Lgl, length of `x`.
  - Types: `c` keeps the common type (Int+Int -> Int; Int+Dbl -> Dbl; mixing Lgl with numeric -> `TypeError:`). `ifelse` returns the common type of `yes`/`no`. `sum(lgl)` and `sum(int)` return Int; `sum(dbl)` returns Dbl (D19). `mean` always Dbl.
  - `data.frame` calls `R0Val::df` (1.2.1). `list` builds `R0Val::List`.
  - Unknown function: `Unknown function or intrinsic: '<name>'`.
- **Anchors:** `grep -n "fn call_intrinsic"`, `grep -n "\"approx\" =>"`.
- **Reference implementation:** prototype lines 1978-2235.
- **Constraints:** `vm.rs` only. Do not implement the spec-only intrinsics (unimplemented names stay "Unknown function"). Record the list of implemented names in STATE.
- **Acceptance:** `cargo test --lib vm::`. Unit tests: `c(1L, 2L)` is Int; `sum(c(TRUE, TRUE))` is Int 2; `approx(c(1,2,3), c(10,20,30), xout = 2.5)` is 25; `list(a = 1)` is `List`; `data.frame(a = c(1,2), b = 1)` traps `ErrDimensionMismatch`.

### 1.6.1 lib + main

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/lib.rs`, `src/main.rs`. Can create: none.
- **Depends on:** 1.5.3.
- **Spec:**
  - `lib.rs`: add `pub use vm::R0Vm; pub use value::R0Val;`. Add `pub const DEFAULT_FUEL: i64 = 100_000;` and `pub fn eval_script(src: &str) -> Result<R0Val, String>` creating a fresh `R0Vm::new(DEFAULT_FUEL)`. Keep the module declarations from 1.1.1.
  - `main.rs`: port prototype lines 2240-2290. The pedon script uses the E5 rewrite. Print the result with `Display`. Print the broadcast trap. Exit code 0 on success, 1 on any trap.
- **Anchors:** `grep -n "fn main"` in spec: line 2240.
- **Constraints:** no FFI yet (1.7.1). `lib.rs` changes here are API only.
- **Acceptance:** `cargo run --release` exits 0 and prints `25.666`, `21.7381 29.0526 28.7059`, `14.5 NA NA`, and the `ErrDimensionMismatch` trap text. `cargo test --lib` passes.

### 1.7.1 wasm FFI

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `src/lib.rs`. Can create: none.
- **Depends on:** 1.6.1.
- **Spec:** D6, linear-memory trio. Add to `lib.rs`:
  - `#[no_mangle] pub unsafe extern "C" fn r0_alloc(len: usize) -> *mut u8` (returns a `Vec<u8>` buffer leaked with `ManuallyDrop` or `mem::forget`).
  - `#[no_mangle] pub unsafe extern "C" fn r0_free(ptr: *mut u8, len: usize)`.
  - `#[no_mangle] pub unsafe extern "C" fn r0_eval(ptr: *const u8, len: usize) -> i32`: reads UTF-8 source from linear memory. Returns 0 on success, 2 on invalid UTF-8, 1 on R0 trap. Stores the output (the `Display` text on success, the error text on trap) in a `thread_local! { static RESULT: RefCell<Vec<u8>> }`.
  - `#[no_mangle] pub extern "C" fn r0_result_ptr() -> *const u8` and `r0_result_len() -> usize`.
  - Each `unsafe` block has a `// SAFETY:` comment. No `static mut`.
- **Anchors:** none in the spec; this is new code.
- **Constraints:** `lib.rs` only. No JS glue files in the repo. `panic = "abort"` is in Cargo.toml, so no `catch_unwind`.
- **Acceptance:** `cargo test --lib` passes; `cargo build --target wasm32-unknown-unknown --release` succeeds; `cargo clippy --all-targets -- -D warnings` passes.

### 1.8.1 conformance C1: newline and pipe

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: none. Can create: `tests/conformance.rs` (this task creates the file, the first of five test groups).
- **Depends on:** 1.5.3.
- **Spec:** a test helper `fn eval(src) -> R0Val` using `r0_wasm::R0Vm` directly, plus `fn eval_err(src) -> String`. Tests named with prefix `newline_` or `pipe_`:
  - `newline_binary_op_continues`: `1 +\n 2` is 3.
  - `newline_statement_terminates`: `1\n2` is 2.
  - `newline_inside_parens_suppressed`: `sum(c(1,\n 2))` is 3.
  - `newline_inside_brackets_suppressed`: `x <- c(10,20,30)\n x[\n2]` is 20.
  - `newline_multiline_pipeline`: `c(1,2,3) |>\n sum()` is 6.
  - `pipe_implicit_first_arg`: `x <- c(1, NA, 3)\n x |> sum(na.rm = TRUE)` is 4.
  - `pipe_placeholder_substitution`: `xs <- c(1,2,3)\n ys <- c(10,20,30)\n xs |> approx(xs, ys, xout = _)`. Form per D18: the placeholder fills `xout`, the piped value. Expected: at xs=(1,2,3) interpolated values are 10, 20, 30.
  - `pipe_rhs_not_call`: `1 |> 2` errors with `Right-hand side of '|>' pipe must be a function call`.
- **Anchors:** spec §5 EBNF `Term`, `PipeExpr`; spec §4 precedence table.
- **Constraints:** only `tests/conformance.rs`. Do not change `src/`. If a test fails because of a `src/` bug, stop and report (escalation rule 4).
- **Acceptance:** `cargo test --test conformance newline_` and `cargo test --test conformance pipe_`. Each filter run reports its own pass count.

### 1.8.2 conformance C2: Kleene and broadcast

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `tests/conformance.rs`. Can create: none.
- **Depends on:** 1.8.1 (same file, serial).
- **Spec:** tests named `kleene_` and `broadcast_`:
  - `kleene_and_full_table`: all 9 pairs of {TRUE, FALSE, NA} for `&`, checked against the spec §3.3 truth table. Exact expected values: `FALSE & NA` FALSE; `TRUE & NA` NA; `NA & NA` NA.
  - `kleene_or_full_table`: all 9 pairs for `|`. `TRUE | NA` TRUE; `FALSE | NA` NA.
  - `kleene_not`: `!TRUE` FALSE, `!FALSE` TRUE, `!NA` NA.
  - `broadcast_scalar_vector`: `1 + c(10, 20)` equals `c(11, 21)`.
  - `broadcast_equal_length`: `c(1, 2) + c(3, 4)` equals `c(4, 6)`.
  - `broadcast_mismatch_traps`: `c(1, 2, 3, 4) + c(10, 20)` errors starting `ErrDimensionMismatch`.
  - `kleene_if_na_traps`: `if (NA) 1` errors starting `ErrInvalidCondition`.
  - `kleene_if_vector_traps`: `if (c(TRUE, FALSE)) 1` errors starting `ErrInvalidCondition`.
- **Anchors:** spec §3.3 (`grep -n "Three-Valued Logic"`), §3.1 (`grep -n "Strict Broadcasting"`).
- **Constraints:** only `tests/conformance.rs`.
- **Acceptance:** `cargo test --test conformance kleene_` and `cargo test --test conformance broadcast_`.

### 1.8.3 conformance C3: indexing

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `tests/conformance.rs`. Can create: none.
- **Depends on:** 1.8.2 (same file, serial).
- **Spec:** tests named `index_` and `assign_`, covering spec §3.4 and the prompt's list, with prototype output as reference:
  - `index_positive`: `x <- c(10,20,30)\n x[1]` is 10.
  - `index_oob_na_padding`: `x[99L]` is NA of Dbl type, and for `1:3`-based Int, NA Int (D3).
  - `index_negative_drop`: `x[-2L]` is `c(10,30)`.
  - `index_mixed_subscript_traps`: `x[c(1L, -2L)]` errors starting `ErrMixedSubscript`.
  - `index_logical_mask`: `x[x > 20]` is `c(30)`.
  - `index_scalar`: `x[[1L]]` is 10.
  - `index_scalar_oob_traps`: `x[[99L]]` errors starting `ErrBoundsTrap`.
  - `assign_subset_mask`: `x <- c(-1,2,-3)\n x[x < 0] <- 0` equals `c(0,2,0)`.
  - `index_preserves_int_type`: `y <- 1:3\n y[2L]` is Int.
- **Anchors:** spec §3.4 (`grep -n "1-Based Indexing"`).
- **Constraints:** only `tests/conformance.rs`.
- **Acceptance:** `cargo test --test conformance index_` and `cargo test --test conformance assign_`.

### 1.8.4 conformance C4: pedon integration

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `tests/conformance.rs`. Can create: none.
- **Depends on:** 1.8.3 (same file, serial).
- **Spec:** the pedon script from `src/main.rs` (E5 form, D13 expectations). Tests named `pedon_`:
  - `pedon_depth_weighted_clay_0_100`: `$wt_clay_0_100cm` within 1e-3 of 25.666.
  - `pedon_approx_25_50_85`: `$interp_25_50_85` within 1e-3 of `[21.7381, 29.0526, 28.7059]`.
  - `pedon_remaining_oob`: `$remaining_oob` equals `[14.5, NA, NA]`.
  - `pedon_result_is_list`: the top-level value is `R0Val::List` with three names.
- **Anchors:** spec line 2244 onward (`depth_weighted_mean`), 2259 onward (`data.frame`).
- **Constraints:** only `tests/conformance.rs`. Copy the script text into the test, do not `include_str!` from `main.rs`.
- **Acceptance:** `cargo test --test conformance pedon_`.

### 1.8.5 conformance C5: fuel

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `tests/conformance.rs`. Can create: none.
- **Depends on:** 1.8.4 (same file, serial).
- **Spec:** test `fuel_infinite_loop_halts`: `while (TRUE) { x <- 1 }` with `R0Vm::new(10_000)` errors with `ErrFuelExhausted`. Also `fuel_finite_loop_ok`: `i <- 0\n while (i < 5) { i <- i + 1 }` completes and `i` is 5. Also a test that `R0Vm::new(100)` with a long vector op errors with `ErrFuelExhausted`.
- **Anchors:** spec §6 (`grep -n "Deterministic Resource Controls"`).
- **Constraints:** only `tests/conformance.rs`.
- **Acceptance:** `cargo test --test conformance fuel_`.

### 1.9.1 final gate

- **Repo:** rzero (`/home/andrew/AI/rzero`). **Temp folder:** `.development/`.
- **Files in scope:** must edit: `docs/ARCHITECTURE.md` (only if the layout note needs a correction). Can create: none.
- **Depends on:** 1.8.5 and 1.7.1.
- **Spec:**
  1. `cargo test` (all tests pass).
  2. `cargo run --release` (exit 0; output matches 1.6.1).
  3. `cargo clippy -- -D warnings` and `cargo clippy --all-targets -- -D warnings` (D14). Both pass.
  4. `cargo build --target wasm32-unknown-unknown --release`. Report exact byte size: `stat -c %s target/wasm32-unknown-unknown/release/r0_wasm.wasm`. Report only, no gate (D11).
  5. Node smoke test, run inline (not committed): `node -e` loads the `.wasm`, calls `r0_alloc`, writes `pedon`-style source, calls `r0_eval`, reads `r0_result_ptr`/`len`, and prints the result text. Expected to print the same numbers as 1.6.1.
  6. If `ARCHITECTURE.md` layout note disagrees with the real `src/` tree, fix the note.
- **Anchors:** none in the spec. Reference values: 1.6.1 acceptance text, 1.7.1 export list.
- **Constraints:** no new files in the repo. Smoke-test script stays in `.development/` (ignored).
- **Acceptance:** all six steps pass. Report each command with its exit code and output tail.

---

## Review record (Phase 5, two passes)

Both passes ran inline, one after the other. Neither used parallel sub-agents, so the two are not fully independent.

**Spec axis** (checked against INITIAL_PROMPT.md and D1-D20):
- Phase 1 steps 1-3 map to 1.1.1, 1.1.2, 1.2.2. Phase 2 modules map to 1.1.1 (stubs), 1.2.1, 1.3.1, 1.3.2, 1.4.1, 1.4.2, 1.5.1-1.5.3, 1.6.1. Phase 3 groups 1-7 map to 1.8.1-1.8.5. Phase 4 maps to 1.7.1 and 1.9.1.
- Constraints trace: zero dependencies (1.1.1); "do not water down" (D3, D12, D19); trap prefixes (1.2.1, 1.5.x tests); "commit after each milestone" (D16).
- Every decision D1-D20 is honored in at least one task.
- **Finding, fixed:** the first draft dropped the prompt's "%in% desugaring" and kept `%in%` as a binary op. Corrected in D17, E12, 1.4.1, and 1.5.3.
- **Finding, fixed:** the first draft added an `R0Vec` constructor the spec doesn't name. Removed.
- **Open:** D14-D19 are defaults, not user answers. The approval reply covers them.

**Standards axis** (checked against wave-orchestration's contract):
- Right-sized: 1.5.1 and 1.5.3 are the largest, since each ports a long prototype range. Each is one range-port with named anchors, so each should fit in 12 micro-plan steps. If a worker needs more, split by `eval` vs `eval_binary` (same file, still serial).
- Files-in-scope honest: `lib.rs` is edited in W1 (1.1.1), W8 (1.6.1), and W9 (1.7.1), always serial by wave. `vm.rs` is serial W5-W7. `tests/conformance.rs` is created in W8 and serial W8-W12. `docs/ARCHITECTURE.md` is created W2 and edited W13.
- Disjointness: W1-W4 pairs share no files. W8 pair (lib+main vs tests) and W9 pair (lib vs tests) are disjoint.
- Acceptance: every task has a targeted command. Every task has a Repo and Temp-folder line. Anchors present for every task.
- Escalation and the serial-suite rule are present.

## Hazards (for workers)

- **Shared-file serialization:** `src/vm.rs` and `tests/conformance.rs` are edited by more than one task. Never run two tasks that touch the same file at once.
- **Spec traps:** do not copy the spec's pedon script, its printed outputs, or its pipe example. E5, E6, E9 apply.
- **Prototype `BinOp` is not `Copy`:** derive it (E7).
- **`is_na_real` is `is_nan`:** a NaN produced by arithmetic is also NA here. Documented, not changed.
- **Trap text is part of the contract:** tests match prefixes like `ErrDimensionMismatch`. Do not reword.
- **`approx` needs ascending knots;** the prototype does not check. Out of scope to check.

## Escalation (per wave-orchestration)

- Two off-spec micro-plans: the orchestrator writes the micro-TODO itself.
- Acceptance fails after one retry: re-run the task on `claude-sonnet-5-5`. No third Haiku attempt.
- Worker needs an out-of-scope file: re-scope this plan. Do not widen permissions ad hoc.

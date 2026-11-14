# STATE 1 -- orchestration 1 (R0 / r0-wasm)

Resume point. A fresh orchestrator reads `EXECUTION_PLAN_1.md` first, then this file.

## Baseline (recorded before wave 1)

| Check | Result (2026-11-14) |
|---|---|
| Repo | `/home/andrew/AI/rzero`, branch `master`, **zero commits** |
| Untracked | `planning/` (INITIAL_PROMPT.md, INITIAL_SPEC.md; planning artifacts are tracked, D7) |
| Crate | none yet (no Cargo.toml, src/, docs/, tests/) |
| Toolchain | cargo/rustc 1.99.1, clippy 0.1.99 |
| wasm target | `wasm32-unknown-unknown` installed |
| Node | v22.23.5 (`/usr/bin/node`) for the wasm smoke test |
| wasm-opt | not installed (size is report-only, D11) |
| Test suites | none. Green baseline is empty; first real baseline is after 1.1.1 |
| Prototype status | spec prototype does not compile as written (E7); pedon script traps (E5); printed outputs wrong (E6) |
| ADRs | `docs/decisions/` absent. No supersession |

## Wave board

| Wave | Task | Status |
|---|---|---|
| W1 | 1.1.1 scaffold crate | done (d041856) |
| W1 | 1.1.2 LANGUAGE_SPEC doc | done (13be3a2) |
| W2 | 1.2.1 value.rs | done (eb737fd) |
| W2 | 1.2.2 ARCHITECTURE doc | done (ee9c99f) |
| W3 | 1.3.1 lexer.rs | done (09ddc82) |
| W3 | 1.3.2 ast.rs | done (1a3a9bf) |
| W4 | 1.4.1 parser.rs | done (60d7daf) |
| W4 | 1.4.2 isa.rs | done (c3213be) |
| W5 | 1.5.1 vm core | done (1da436a) |
| W6 | 1.5.2 vm indexing | done (34e260c) |
| W7 | 1.5.3 vm intrinsics + named args | done (0ab7778) |
| W8 | 1.6.1 lib + main | done (0ce01eb) |
| W8 | 1.8.1 conformance C1 (newline, pipe) | done (bc04e8a; fix 975d6ef) |
| W9 | 1.7.1 wasm FFI | done (ef98e9b) |
| W9 | 1.8.2 conformance C2 (Kleene, broadcast) | done (4054693) |
| W10 | 1.8.3 conformance C3 (indexing) | pending |
| W11 | 1.8.4 conformance C4 (pedon) | pending |
| W12 | 1.8.5 conformance C5 (fuel) | pending |
| W13 | 1.9.1 final gate | pending |

Status values: `pending`, `running`, `done`, `blocked`, `escalated`. Commit SHAs go in each task's report.

## Task reports

_Workers append here under `## <task-id>`. Orchestrator records code commit SHAs and wave-end suite results._

## 1.1.1

- status: done (orchestrator-verified)
- files: Cargo.toml, Cargo.lock, .gitignore, src/lib.rs, src/main.rs, six stub modules (src/value.rs, lexer.rs, ast.rs, parser.rs, vm.rs, isa.rs)
- code commit: d041856 feat: scaffold r0-wasm crate
- tests: cargo build (debug, release) pass; cargo clippy --all-targets -- -D warnings pass; cargo test 0 tests, 0 failures
- deviations: none

## 1.1.2

- status: done (orchestrator-verified)
- files: docs/LANGUAGE_SPEC.md (created)
- code commit: 13be3a2 docs: add R0 language specification
- checks: acceptance greps pass; EBNF body identical to spec lines 245-301; zero transcript noise; zero HTML entities
- errata: E1 Case 3 bullet restored; E4 heading 48 -> 69 (counted 69: 6.1 18, 6.2 7, 6.3 19, 6.4 13, 6.5 12); extra: ### added to 3.4 heading
- tests: no code change; cargo test 0 tests; clippy clean

## Wave 1 gate

- wave 1 complete. Serial cargo test and clippy --all-targets -D warnings both pass.

## 1.2.1

- status: done (orchestrator-verified)
- files: src/value.rs only
- code commit: eb737fd feat(value): add runtime value representation and R0Vec descriptor
- tests: cargo test --lib value:: 5 passed; clippy --all-targets -D warnings clean
- coordinator answers applied: R0Vec::tag -> u32 masked bits 0..3; TAG_LIST = 6; na_of_type_like Range->Int NA, Df/List/Null->Null; is_empty added; range len unchanged, pinned by test range_len_matches_materialized_len
- extra test beyond plan: r0vec_tag_masks_bits_0_to_3
- deviations: none

## 1.2.2

- status: done (orchestrator-verified)
- files: docs/ARCHITECTURE.md (created)
- code commit: ee9c99f docs: add R0 architecture specification
- checks: acceptance greps pass; no HTML entities; zero transcript noise; "Deferred (not in this plan)" x3
- errata: E2 Kleene NOT block -> i8x16_swizzle with KLEENE_NOT_LUT; E3 ISA diagram -> bit-field table
- status note: std (not no_std); exports r0_alloc, r0_free, r0_eval, r0_result_ptr, r0_result_len (no decision IDs)
- deviations: none

## Wave 2 gate

- wave 2 complete. Serial cargo test --lib (5 passed) and clippy --all-targets -D warnings clean.

## 1.3.2

- status: done (orchestrator-verified)
- files: src/ast.rs
- code commit: 1a3a9bf feat(ast): add BinOp, UnaryOp and Expr AST types
- tests: cargo build ok; clippy --all-targets -D warnings clean; cargo test --lib 5 passed
- deviations: BinOp derives Copy, PartialEq (E7); Expr keeps spec derives (Clone, Debug)

## 1.3.1

- status: done (orchestrator-verified)
- files: src/lexer.rs
- code commit: 09ddc82 feat(lexer): add R-subset tokenizer with newline suppression
- tests: cargo test --lib 12 passed (lexer 7); clippy --all-targets -D warnings clean
- fixes applied (approved): nested bracket stack for ]]; newline suppressed after if/while/for/function header ')'; unexpected-char message decodes real char; is_none_or instead of map_or
- plan size: 14 steps (over the 12-step guideline; orchestrator approved with fixes)
- discovered, not fixed: (a) doubled ';' after Term emits a second Term; (b) 1.5L lexes as DblLit + Ident("L")

## Wave 3 gate

- wave 3 complete. Serial cargo test --lib (12 passed) and clippy --all-targets -D warnings clean.

## 1.4.2

- status: done (orchestrator-verified)
- files: src/isa.rs
- code commit: c3213be feat(isa): add Opcode, TileKernel and Instr encoding
- tests: isa:: 3 tests pass; clippy --all-targets -D warnings clean
- errata E13 applied: tile block 0x20-0x44 (37 incl. TILE_SUM_ACCUM); reduce/cumulative 0x45-0x4C; subset 0x50-0x57; total 63 named opcodes
- OPEN ISSUE (not blocking; no codegen uses it yet): FOR_RANGE_STEP layout A=r_iter, B=r_limit(8-bit), Bx=offset16 overlaps B/C, because Bx = B<<8|C. Decide before any codegen task: e.g., limit in the following word, or offset in Bx with limit in a constant-pool index.

## 1.4.1

- status: done (orchestrator-verified)
- files: src/parser.rs
- code commit: 60d7daf feat(parser): add Pratt parser for R subset
- tests: cargo test --lib 23 passed (parser 8); clippy --all-targets -D warnings clean
- approved deviations: unary -/+ operand bp 20 (spec §4; prototype 17 wrongly bound : and %% inside unary); x %in% t -> call %in%(x, t) (D17); all _ placeholders replaced (prototype)

## Wave 4 gate

- wave 4 complete. Serial cargo test --lib (23 passed) and clippy --all-targets -D warnings clean.

## 1.5.1

- status: done (orchestrator-verified)
- files: src/vm.rs
- code commit: 1da436a feat(vm): add evaluator core with fuel and strict broadcasting
- tests: cargo test 30 passed (vm 7); clippy --all-targets -D warnings clean
- approved deviation: Expr::Call checks user functions first (needed by the pedon script in 1.8.4); call_intrinsic only for non-user names
- discovered: (a) R0Val has no PartialEq; tests compare Debug output. Keep that approach in 1.5.2/1.5.3 (no value.rs change). (b) micro-plan had 15 checkboxes vs 12-step guideline; executed as written. (c) stub tests for field and index_scalar are not in 1.5.1; they are covered when 1.5.2 replaces those stubs.

## Wave 5 gate

- wave 5 complete. Serial cargo test (30 passed) and clippy --all-targets -D warnings clean.

## 1.5.2

- status: done (orchestrator-verified)
- files: src/vm.rs
- code commit: 34e260c feat(vm): add type-preserving indexing and subset assignment
- tests: cargo test 36 passed (vm 13); clippy --all-targets -D warnings clean
- approved answers: NA mask skipped on assignment; out-of-range assignment ignored; [[i]] on List/Df returns column; [i] on List/Df is TypeError; mask/RHS length mismatch ErrDimensionMismatch; assignment into Null/Df/List is TypeError
- discovered (open): (a) out-of-range positive assignment ignored; R extends with NA padding; decide before 1.5.x closes; (b) [ on Null base returns Null (R behavior, not explicitly approved); (c) $ unknown-name message now "Unknown column '$name'" (spec text; prototype appended " in data.frame")

## 1.5.3

- status: done (orchestrator-verified)
- files: src/vm.rs
- code commit: 0ab7778 feat(vm): add intrinsics and named-argument binding
- tests: cargo test 39 passed (vm 16); clippy --all-targets -D warnings clean
- discovered (open, need decision): (1) pmin/pmax return Dbl for Int input, inconsistent with Int-preserving abs and min/max; (2) min/max of empty Int/Lgl returns Dbl +/-Inf; (3) c() with no non-NULL args returns Null (R behavior, unplanned); (4) extra messages: TooManyArguments, DuplicateArgument, ErrIntOverflow, approx length mismatch is ErrDimensionMismatch; (5) named args on c() ignored, not rejected (prototype behavior); (6) is.na on Chr still errors (prototype behavior); (7) %in% with Null table is empty; Null left operand still errors

## Wave 7 gate

- wave 7 complete. Serial cargo test (39 passed) and clippy --all-targets -D warnings clean.

## 1.6.1

- status: done (orchestrator-verified)
- files: src/lib.rs, src/main.rs
- code commit: 0ce01eb feat(api): expose eval_script and pedon demo in main
- verified: cargo run --release exit 0; output 25.666 / 21.7381 29.0526 28.7059 / 14.5 NA NA; fuel 99809; broadcast trap printed; cargo test --lib 39 passed; clippy clean

## 1.8.1 (in progress)

- BLOCKED on src bug E14 (c() rejects NA mixed with numbers). Erratum E14 added (a22dad3). Re-scoped as 1.8.1a (fix in src/vm.rs). tests/conformance.rs remains uncommitted until 1.8.1a lands.

## 1.8.1

- status: done (orchestrator-verified)
- files: tests/conformance.rs (8 tests: newline_ x4, pipe_ x3 ... see file)
- code commit: bc04e8a test(conformance): add newline and pipe conformance tests
- blocker resolved via E14 / task 1.8.1a

## 1.8.1a

- status: done (orchestrator-verified)
- files: src/vm.rs (common_concat)
- code commit: 975d6ef fix(vm): promote all-NA logical in c() to numeric
- tests: cargo test 41 lib + 8 conformance passed; clippy --all-targets -D warnings clean

## Wave 8 gate

- wave 8 complete. Serial cargo test (41 lib + 8 conformance) and clippy --all-targets -D warnings clean.

## 1.7.1

- status: done (orchestrator-verified)
- files: src/lib.rs
- code commit: ef98e9b feat(api): add wasm linear-memory FFI
- tests: cargo test --lib 45 passed; wasm32 release build ok; clippy --lib --bins -D warnings clean
- size (report only, D11): release r0_wasm.wasm = 253154 bytes (spec target 200 KB; not a gate)
- deviation: success test expects "[1] 3" (Display), not "3"; clippy needed const in thread_local
- note: full clippy --all-targets currently fails only on tests/conformance.rs (in-progress 1.8.2 edits), to be checked at wave gate

## 1.8.2

- status: done (orchestrator-verified)
- files: tests/conformance.rs
- code commit: 4054693 test(conformance): add Kleene and broadcast conformance tests
- tests: kleene_ 5 pass; broadcast_ 3 pass; conformance total 16 pass

## Wave 9 gate

- wave 9 complete. Serial cargo test (45 lib, 16 conformance) and clippy --all-targets -D warnings clean. git status clean.

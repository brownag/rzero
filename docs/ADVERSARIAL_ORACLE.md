# `r0-wasm` Adversarial Code Review Oracle

> **Ground-Truth Baseline:** Audited against `r0-wasm` on branch `fix/spec-inherited-bugs` (`HEAD` `0d5ee99`), `cargo test` passing 57 unit tests + 38 conformance tests (`0` failures), `docs/LANGUAGE_SPEC.md` (v0.1 + Errata B1–B6), and `docs/ARCHITECTURE.md` (v0.1 + Erratum B7).

This document is the authoritative grading rubric and attack playbook for downstream auditing agents performing four specialized code reviews on `r0-wasm`:
1. **Subtractive Review** (Dead weight, premature abstraction, binary bloat, and non-essential complexity)
2. **Standards & Idioms Review** (IEEE 754, Two's Complement overflow, Wasm32 ABI alignment, zero-dependency Rust safety)
3. **Spec-vs-Implementation Conformance Review** (Silent semantic drift between `docs/LANGUAGE_SPEC.md` and `src/*.rs`)
4. **State-Machine & Invariant Review** (Lexer newline-suppression automaton, Pratt parser binding power transitions, VM fuel/stack/type states)

---

## Section 1: Project Intent & Non-Negotiable Axiom Ledger

All code changes, refactors, and bug fixes in `r0-wasm` are governed by four hard boundary conditions. Any change that violates any of these axioms must be **automatically rejected**.

* **Axiom 1 (Valid GNU R Subset):** Every valid R0 script (modulo optional `: type` / `-> type` annotations) must parse and evaluate to the exact same result in base GNU R (`Rscript --vanilla`), unless it triggers an explicit R0 strictness trap (`ErrDimensionMismatch`, `ErrInvalidCondition`, `ErrMixedSubscript`, `ErrBoundsTrap`).
* **Axiom 2 (Sub-200 KB Wasm Footprint & Zero Dependencies):** Zero third-party crates in `[dependencies]` (`Cargo.toml` has no `[dependencies]` section). No heavy regex engines, parser generators, or panic-formatting bloat in hot paths. `[profile.release]` enforces `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`.
* **Axiom 3 (Deterministic Sandboxing):** No unbounded loops, no un-metered allocations or $O(N^2)$ intrinsic loops without fuel charging, and no host panics (`unwrap()` / slice index out-of-bounds) reachable from arbitrary user source strings.
* **Axiom 4 (Value Semantics without GC):** Mutation (`x[i] <- v`) must never alias visible variables.

---

## Section 2: Lens 1 — Subtractive Review Protocol (Bloat, Allocation & Complexity Elimination)

### 2.1 Prototype Shortcut Debt vs. True Minimalism
* **Environment Cloning on Every Function Call (`src/vm.rs:108`, `src/vm.rs:248`, `src/vm.rs:65`):**
  * `R0Vm::eval` (`Expr::Assign` for `Expr::FnDef`, line 108) clones the entire `env: HashMap<String, R0Val>` into `UserFn::captured_env`.
  * `R0Vm::call_user` (line 248) clones `ufn.captured_env` into `local_env` on every invocation.
  * `R0Vm::eval_script` (line 65) clones `self.globals` into `env` and assigns it back on completion.
  * **Impact:** $O(V)$ heap allocations per function call where $V$ is the number of live globals, rather than lexical frame slots or copy-on-write variable bindings.
* **Unnecessary `Vec<f64>` Materialization in `eval_binary` (`src/vm.rs:328-329`):**
  * `a.promote_dbl()?` and `b.promote_dbl()?` allocate full `Vec<f64>` clones even when `a` or `b` is already `R0Val::Dbl` (via `v.clone()` in `src/value.rs:180`), and even for `Int` vectors that could be operated on directly or streamed.
* **Unnecessary `Vec<Option<R0Val>>` Allocation in `bind_args` (`src/vm.rs:556-579`):**
  * `bind_args` allocates two `Vec`s (`slots` and `positional`) on every single intrinsic call (`length`, `abs`, `sum`, etc.).

### 2.2 AST & Parser Footprint
* **Token Cloning on Every Advance (`src/parser.rs:20-26`):**
  * `Parser::next(&mut self) -> Tok` calls `self.peek().clone()`, cloning `String` payloads inside `Tok::Ident(String)` and `Tok::StrLit(String)` on every token consumed, plus another `.clone()` in named-argument lookahead (`src/parser.rs:239`).
* **Dead ISA & ABI Scaffolds (`src/isa.rs`, `src/value.rs:48-85`):**
  * `Opcode`, `TileKernel`, and `Instr` in `src/isa.rs` (236 lines) and `R0Vec` in `src/value.rs:48-85` are layout/encoding definitions only (per `ARCHITECTURE.md` / `D5`) and are not wired to any compiler or VM execution path. They are intentional scaffolds, but any *new* dead variants or unused helpers added outside `isa.rs`/`R0Vec` must be rejected.

### 2.3 Kill Criteria Table

| Target Pattern | File & Location | Issue | Action |
|---|---|---|---|
| `v.clone()` in `promote_dbl` / `materialize_int` | `src/value.rs:159`, `src/value.rs:180` | Clones `Vec<i32>` / `Vec<f64>` on read-only paths | Borrow `&[f64]` / `&[i32]` via slice view when already `Dbl`/`Int` |
| `self.peek().clone()` | `src/parser.rs:20-26`, `src/parser.rs:239` | Clones `Tok::Ident`/`Tok::StrLit` strings | Use `std::mem::replace` with `Tok::Eof` or index-based `&Tok` |
| `env.clone()` on `FnDef` & `call_user` | `src/vm.rs:108`, `src/vm.rs:248` | Full `HashMap<String, R0Val>` clone per call | Capture only free variables or use scoped frame stack |
| `Vec<String>` allocations in `Display` | `src/value.rs:207-220` | Allocates a `String` per element to print | Stream directly to `fmt::Formatter` with `write!` |
| `BinOp::InOp` | `src/ast.rs:10`, `src/vm.rs:307-310` | Dead AST variant; parser desugars `%in%` to `Expr::Call` | Remove `BinOp::InOp` or document as unreachable |

---

## Section 3: Lens 2 — Standards, Arithmetic & Wasm ABI Audit Protocol

### 3.1 IEEE 754 `NA_real_` vs `NaN` Semantics
* **Payload Preservation (`src/value.rs:9-24`):**
  * `NA_REAL_BITS = 0x7ff8_0000_0000_07a2` (low 32-bit word `1954`).
  * `is_na_real(x)` checks `x.is_nan() && (x.to_bits() & 0xffff_ffff) == 1954`.
  * `is_missing(x)` checks `x.is_nan()`.
* **Audit Check (`[STD-01]`):** In `src/vm.rs:336`, `eval_binary` checks `if is_na_real(va) || is_na_real(vb) { na_real() }`, which preserves `NA_real_` when an operand is `NA_real_`, and lets hardware IEEE 754 propagate `NaN` when an operand is `NaN`. However, in `src/vm.rs:866` (`abs`), `src/vm.rs:899` (`cumsum`), and `src/vm.rs:953` (`%in%`), `is_na_real` is used instead of `is_missing`, meaning `NaN` falls through to `v.abs()`, `run + v`, or `e == t` instead of being treated as missing where required!

### 3.2 Two's Complement Integer Traps (`NA_INT = i32::MIN`)

* **`[STD-02]` (CRITICAL) — Unary Negation `-i32::MIN` Overflow**
  * **Location:** `src/vm.rs:146-148` (`R0Vm::eval`, `Expr::Unary(UnaryOp::Neg, ...)`).
  * **Triggering R0 Snippet:** `x <- -2147483647L - 1L` (if integer) or `(-2147483648L)` (if `NA_INT` is bypassed, or `abs(NA_INT)`). Note: `src/vm.rs:147` guards `if x == NA_INT { NA_INT } else { -x }`, which is safe for `NA_INT`, **but** `elem_at` (`src/vm.rs:38`) computes `start + (k as i32) * step` without overflow checks, and `Range::len()` (`src/value.rs:143`) computes `(((end - start) / step).abs() + 1) as usize`!
  * **Expected Behavior (Oracle):** `end - start` and `((end - start) / step).abs()` must not overflow `i32` or panic in debug builds / wrap in release builds.
  * **Actual Behavior:** `R0Val::Range { start: -2_000_000_000, end: 2_000_000_000, step: 1 }.len()` panics with `attempt to subtract with overflow` in debug mode and wraps to a bogus `usize` in release mode.
  * **Verification Predicate:**
    ```rust
    let r = R0Val::Range { start: -2_000_000_000, end: 2_000_000_000, step: 1 };
    assert_eq!(r.len(), 4_000_000_001);
    ```

* **`[STD-03]` (HIGH) — `Range::materialize_int` & `promote_dbl` `i32` Addition Overflow**
  * **Location:** `src/value.rs:166` (`materialize_int`) and `src/value.rs:191` (`promote_dbl`).
  * **Triggering R0 Snippet:** `x <- 2147483647L:2147483647L` (last iteration executes `cur += *step` where `cur == i32::MAX` and `step == 1`).
  * **Expected Behavior (Oracle):** Evaluating `2147483647L:2147483647L` must succeed and yield `[1] 2147483647L`.
  * **Actual Behavior:** On the final loop iteration (`0..n`), `cur += *step` executes `2147483647i32 + 1`, panicking in debug mode (`attempt to add with overflow`)!
  * **Verification Predicate:**
    ```rust
    let r = R0Val::Range { start: i32::MAX, end: i32::MAX, step: 1 };
    assert_eq!(r.materialize_int().unwrap(), vec![i32::MAX]);
    ```

* **`[STD-04]` (MEDIUM) — `Dbl` to `Int` Cast Truncation / Saturation (`x as i32`)**
  * **Location:** `src/value.rs:172` (`R0Val::materialize_int`, `R0Val::Dbl` arm).
  * **Triggering R0 Snippet:** `x <- c(1, 2, 3); x[1e20]`
  * **Expected Behavior (Oracle):** Out-of-range float indices (`> i32::MAX` or `< i32::MIN + 1`) should map to `NA_INT` (or OOB `None`), not saturate to `i32::MAX` (`2147483647`).
  * **Actual Behavior:** `1e20 as i32` saturates to `i32::MAX` in Rust, which happens to be OOB for small vectors, but `-1e20 as i32` saturates to `i32::MIN` (`NA_INT`).

### 3.3 Kleene Three-Valued Bitwise LUT Invariants
* **Location:** `src/value.rs:31-45` (`KLEENE_AND_LUT`, `KLEENE_OR_LUT`, `KLEENE_NOT_LUT`).
* **Invariant:** Inputs `a, b` are masked with `& 3` (`src/vm.rs:153`, `src/vm.rs:321-322`), restricting indices to `0..=3` for `NOT` and `0..=15` for `AND`/`OR` (`(a << 2) | b`), matching `i8x16_swizzle` lane indices `0..15`.
* **Audit Note:** `KLEENE_NOT_LUT` is `[u8; 4] = [1, 0, 2, 0]`; when `simd_kernels.rs` is implemented, it must be zero-padded to `[u8; 16]` for `i8x16_swizzle`.

### 3.4 16-Byte `R0Vec` Alignment & Wasm32 ABI
* **Location:** `src/value.rs:69-78`.
* **Verified:** Compile-time assertions `assert!(std::mem::size_of::<R0Vec>() == 16)` and `assert!(std::mem::align_of::<R0Vec>() == 16)` are present and pass on both 64-bit host and `wasm32-unknown-unknown`.

---

## Section 4: Lens 3 — Spec-vs-Implementation Differential Matrix

### 4.1 Differential Conformance Table

| Feature / Spec Section | Status | Notes / Location |
|---|---|---|
| Atomic types `dbl`, `int`, `lgl`, `chr` (§1.1) | `CONFORMANT` | `src/value.rs:85-97` |
| `NA_real_` vs `NaN` distinction (B3) | `CONFORMANT` | `src/value.rs:17-24`, `src/lexer.rs:237-238` |
| `NA_character_` literal (`NA_character_`) (§5) | `DIVERGENT (BUG)` | Missing from `src/lexer.rs:226-243`! Lexes as `Ident("NA_character_")` |
| `range`, `list`, `df` (§1.2) | `CONFORMANT` | `src/value.rs:91-95` |
| Strict broadcasting (§3.1) | `CONFORMANT` | `src/vm.rs:280-295` |
| `int` + `int` arithmetic preserves `int` (§3.2) | `DIVERGENT (BUG)` | `src/vm.rs:328-352` promotes all binary arithmetic to `Dbl`! |
| `lgl` arithmetic is `TypeError` (§3.2) | `CONFORMANT` | `promote_dbl` rejects `Lgl` |
| Kleene `&`, `\|`, `!`, `&&`, `\|\|` (§3.3) | `CONFORMANT` | `src/vm.rs:151-171`, `src/vm.rs:314-326` |
| 1-based indexing & OOB `NA` padding (§3.4) | `CONFORMANT` | `src/vm.rs:383-455` |
| 2D DataFrame subsetting `hz[mask, ]` (§1.2, §5) | `UNIMPLEMENTED` | `src/parser.rs:75-79` only parses 1D `x[idx]` |
| `break` and `next` statements (§5) | `UNIMPLEMENTED` | Not in `Tok`, `Expr`, `parser.rs`, or `vm.rs` |
| `return()` with no arguments (§5) | `DIVERGENT (BUG)` | `src/parser.rs:161-166` requires an `Expr` inside `return(...)` |
| `CmpExpr` non-chaining (§5) | `CONFORMANT` | `infix_bp` is left-associative `(9, 10)` |
| `.` identifier vs `.5` float (§5) | `CONFORMANT` | `src/lexer.rs:143` vs `src/lexer.rs:214` |
| `ifelse(cond, 1L, 2L)` type preservation | `CONFORMANT` | `src/vm.rs:832-842` uses `common_concat` + `gather` |

### 4.2 Concrete Spec-vs-Implementation Bugs

* **`[SPEC-01]` (HIGH) — `NA_character_` Literal Missing in Lexer**
  * **Location:** `src/lexer.rs:226-243` (`tokenize`).
  * **Triggering R0 Snippet:** `NA_character_`
  * **Expected Behavior (Oracle):** `LANGUAGE_SPEC.md` §5 defines `NA_LIT = "NA" | "NA_real_" | "NA_integer_" | "NA_character_" | "NaN" | "Inf"`. Must evaluate to `R0Val::Chr(vec![None])`.
  * **Actual Behavior:** Lexes as `Tok::Ident("NA_character_")` and fails with `Compile/Scope Error: Undeclared variable 'NA_character_'`.
  * **Verification Predicate:**
    ```rust
    assert_eq!(format!("{:?}", eval("NA_character_")), "Chr([None])");
    ```

* **`[SPEC-02]` (HIGH) — Integer Binary Arithmetic Silently Coerces `Int` to `Dbl`**
  * **Location:** `src/vm.rs:328-352` (`R0Vm::eval_binary`).
  * **Triggering R0 Snippet:** `1L + 2L`
  * **Expected Behavior (Oracle):** `LANGUAGE_SPEC.md` §3.2: `int -> dbl` promotion occurs only between `int` and `dbl`, or for `/` (real division). `1L + 2L` in GNU R returns `3L` (`R0Val::Int(vec![3])`).
  * **Actual Behavior:** `eval_binary` unconditionally calls `a.promote_dbl()?` and `b.promote_dbl()?`, returning `R0Val::Dbl(vec![3.0])`.
  * **Verification Predicate:**
    ```rust
    assert_eq!(format!("{:?}", eval("1L + 2L")), "Int([3])");
    ```

* **`[SPEC-03]` (MEDIUM) — `return()` Without Arguments Fails to Parse**
  * **Location:** `src/parser.rs:161-166` (`Parser::parse_prefix`, `Tok::Return`).
  * **Triggering R0 Snippet:** `f <- function() { return() }; f()`
  * **Expected Behavior (Oracle):** `LANGUAGE_SPEC.md` §5: `ReturnStmt = "return" "(" [ Expr ] ")" ;`. Should return `R0Val::Null`.
  * **Actual Behavior:** Calls `self.parse_expr(0)?` unconditionally, failing with `Unexpected token in expression: RParen`.
  * **Verification Predicate:**
    ```rust
    assert_eq!(format!("{:?}", eval("f <- function() { return() }; f()")), "Null");
    ```

* **`[SPEC-04]` (MEDIUM) — `pmin` / `pmax` Coerce `Int` Inputs to `Dbl`**
  * **Location:** `src/vm.rs:844-862` (`R0Vm::call_intrinsic`, `"pmin" | "pmax"`).
  * **Triggering R0 Snippet:** `pmin(1L, 2L)`
  * **Expected Behavior (Oracle):** GNU R returns `1L` (`Int`).
  * **Actual Behavior:** Calls `Self::as_f64` and returns `R0Val::Dbl(vec![1.0])`.
  * **Verification Predicate:**
    ```rust
    assert_eq!(format!("{:?}", eval("pmin(1L, 2L)")), "Int([1])");
    ```

### 4.3 Locked Prelude Intrinsic Coverage (29 Implemented / 40 Unimplemented of 69 Specified)
* **Implemented (29):** `c`, `length`, `seq_len`, `seq_along`, `data.frame`, `list`, `is.na`, `is.nan`, `abs`, `sqrt`, `exp`, `log`, `log10`, `floor`, `ceiling`, `pmin`, `pmax`, `ifelse`, `sum`, `mean`, `min`, `max`, `cumsum`, `diff`, `which`, `%in%`, `approx`. (Plus `c`, `data.frame`, `list` variadics).
* **Unimplemented (40):**
  * §6.1: `rep`, `rep_len`, `seq`, `numeric`, `integer`, `logical`, `character`, `as.numeric`, `as.integer`, `as.logical`, `nrow`, `ncol`.
  * §6.2: `is.finite`, `is.infinite`, `any`, `all`, `anyNA`.
  * §6.3: `sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2`.
  * §6.4: `prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax`.
  * §6.5: `which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval`.
* **Signature Mismatches on Implemented Intrinsics:**
  * `approx(x, y, xout, rule = 1L)`: `rule` parameter is missing (`src/vm.rs:768`).
  * `diff(x, lag = 1L)`: `lag` parameter is missing (`src/vm.rs:774`, `src/vm.rs:905-908`).

---

## Section 5: Lens 4 — State-Machine & Invariant Verification Matrix

### 5.1 Lexer Newline-Suppression & Delimiter Depth Automaton (`src/lexer.rs`)
* **State Variables:** `(i, tokens, stack: Vec<Open>, header_close: Option<usize>)`.
* **Verified Safe:**
  * Comments `# ... \n` (`src/lexer.rs:124-129`) `continue` without modifying `tokens`, so `1 + # comment\n 2` preserves `Tok::Plus` in `tokens.last()` and suppresses `\n`.
  * `if (cond)\n expr \n else \n expr`: `stack` tracks `Open::Header`; `header_close` suppresses `\n` after `)`, and `Parser::parse_prefix` (`src/parser.rs:172`) calls `self.eat_terms()` before checking `Tok::Else`.
* **State-Breaker Bugs:**

  * **`[STATE-01]` (HIGH) — Unescaped Quotes / Backslash Escapes in String Literals**
    * **Location:** `src/lexer.rs:179-192` (`tokenize`).
    * **Triggering R0 Snippet:** `"a\"b"`
    * **Expected Behavior (Oracle):** `\"` should be parsed as an escaped quote inside the string literal.
    * **Actual Behavior:** `while i < bytes.len() && bytes[i] != quote` stops at the `"` after `\`, leaving `b"` as trailing tokens and failing with `Unterminated string literal`.
    * **Verification Predicate:**
      ```rust
      assert!(tokenize(r#""a\"b""#).is_ok());
      ```

  * **`[STATE-02]` (MEDIUM) — Unbalanced Closing Delimiters Silently Ignored**
    * **Location:** `src/lexer.rs:308-321` (`tokenize`).
    * **Triggering R0 Snippet:** `1 )` or `1 ]`
    * **Expected Behavior (Oracle):** Unmatched `)` or `]` should be rejected cleanly.
    * **Actual Behavior:** `stack.pop()` returns `None` silently; `Parser` later rejects it, but mismatched `([)]` corrupts `stack` without a lexer error.

### 5.2 Pratt Parser & Pipe Desugaring State Machine (`src/parser.rs`)
* **Binding Power Verification:**
  * `UnaryOp::Neg` uses `self.parse_expr(20)` (`src/parser.rs:158`), and `Tok::Caret` has `(20, 19)` (`src/parser.rs:284`). Thus `-2^2` parses as `-(2^2) = -4`, matching GNU R.
* **Multiple `_` Placeholders:** `desugar_pipe` (`src/parser.rs:290-307`) replaces all `_` placeholders with `lhs.clone()`.

### 5.3 Evaluator Control-Flow, Aliasing & Fuel State Machine (`src/vm.rs`)

* **`[STATE-03]` (CRITICAL) — OOM Abort Before Fuel Check on Large `Range` / `seq_len`**
  * **Location:** `src/value.rs:156` (`materialize_int`), `src/value.rs:181` (`promote_dbl`), `src/vm.rs:319` (`eval_binary`).
  * **Triggering R0 Snippet:** `x <- 1:2000000000; x[1]`
  * **Expected Behavior (Oracle):** Axiom 3 requires `consume_fuel(n)` *before* `Vec::with_capacity(n)` so `ErrFuelExhausted` is returned instead of a host OOM abort.
  * **Actual Behavior:** `R0Val::Range { start: 1, end: 2_000_000_000, step: 1 }` is created in $O(1)$ (`vm.rs:305`). Then `x[1]` calls `subset` (`vm.rs:397`), which calls `base.materialize_int()?` **before** `self.consume_fuel(idx.len().max(n))?` (`vm.rs:407`)! `Vec::with_capacity(2_000_000_000)` aborts the process with OOM before fuel is ever checked.
  * **Verification Predicate:**
    ```rust
    let err = R0Vm::new(100_000).eval_script("x <- 1:2000000000; x[1]").unwrap_err();
    assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
    ```

* **`[STATE-04]` (HIGH) — `ControlFlow::Return` Leaked Across `eval_val` in `if`/`for`/`while`? (Verified Safe, but `eval_val` Swallows `Return` in Subexpressions)**
  * **Location:** `src/vm.rs:235-239` (`R0Vm::eval_val`).
  * **Triggering R0 Snippet:** `f <- function() { x <- return(5); 10 }; f()`
  * **Expected Behavior (Oracle):** `return(5)` should immediately exit `f()` with `5`, not assign `5` to `x` and continue to `10`.
  * **Actual Behavior:** `Expr::Assign` calls `self.eval_val(value, env)?`, which converts `ControlFlow::Return(5)` into `Ok(5)`, assigns `x <- 5`, and continues executing `10`!
  * **Verification Predicate:**
    ```rust
    assert_eq!(format!("{:?}", eval("f <- function() { x <- return(5); 10 }; f()")), "Dbl([5.0])");
    ```

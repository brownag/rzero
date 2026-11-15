# R0 Language Specification v0.2

> **Status:** Normative for the current `r0-wasm` Rust AST interpreter (`src/lexer.rs`, `src/parser.rs`, `src/ast.rs`, `src/value.rs`, `src/vm.rs`, `src/lib.rs`). R0 is a deterministic, strictly evaluated subset of GNU R. Features described as planned or future are not implemented in v0.2. No compiler, static type checker, loop-fusion backend, or Arrow ABI is active at runtime.

Valid R0 programs use standard R syntax except for optional `: type` parameter annotations, optional `-> type` return annotations, and the `fn` keyword alias for `function`. The crate requires zero third-party dependencies.

---

## 1. Project Axioms

All behavior in `r0-wasm` is governed by four axioms (`docs/ADVERSARIAL_ORACLE.md` §1):

1. **Axiom 1 (Valid GNU R Subset):** Every valid R0 script (modulo optional `: type` / `-> type` annotations and `fn` alias) evaluates to the same result as base GNU R (`Rscript --vanilla`) within the supported subset, unless it triggers an explicit R0 strictness trap (`ErrDimensionMismatch`, `ErrInvalidCondition`, `ErrMixedSubscript`, `ErrBoundsTrap`, `ErrIntOverflow`, `ErrFuelExhausted`).
2. **Axiom 2 (Compact Wasm Footprint & Zero Dependencies):** Zero third-party crates in `[dependencies]`. Release builds use `opt-level = "z"`, `lto = true`, `codegen-units = 1`, and `panic = "abort"`. The <200 KB figure is an aspirational target rather than a hard gate; the current release `.wasm` binary is 270,371 bytes (an order-of-magnitude reduction compared to 25 MB+ WebAssembly R runtimes).
3. **Axiom 3 (Deterministic Sandboxing):** Loops, vector operations, range materializations, and nested scans (`%in%`, `approx`) are metered by a deterministic fuel counter (`DEFAULT_FUEL = 100_000` in `eval_script` / `r0_eval`). Arbitrary user source strings must never trigger a host panic or un-metered allocation abort.
4. **Axiom 4 (Value Semantics without GC):** Values are owned Rust types (`R0Val`). Subset assignment (`x[i] <- v`) produces an updated value and rebinds the target identifier without mutating aliased bindings.

---

## 2. Values, Types, and Memory Representation

There are no separate scalar types at the language level: a scalar is a vector of length 1.

### 2.1 Atomic and Composite Values (`R0Val`)

| R0 Type | Runtime Representation (`R0Val`) | Missing Sentinel | Length (`R0Val::len()`) |
|---|---|---|---|
| `dbl` | `Dbl(Vec<f64>)` | `NA_real_` (`0x7ff8_0000_0000_07a2`, low 32-bit word `1954`); other NaNs remain distinct | `v.len()` |
| `int` | `Int(Vec<i32>)` | `NA_INT` (`i32::MIN`, `-2147483648`) | `v.len()` |
| `lgl` | `Lgl(Vec<u8>)` (`0` = `FALSE`, `1` = `TRUE`, `2` = `NA`) | `LGL_NA` (`2`) | `v.len()` |
| `chr` | `Chr(Vec<Option<String>>)` | `None` (`NA_character_`) | `v.len()` |
| `range` | `Range { start: i32, end: i32, step: i32 }` | None (endpoints are non-NA `i32`) | `0` if `step == 0`, else `usize::try_from((end as i64 - start as i64).abs() / (step as i64).abs() + 1).unwrap_or(usize::MAX)` |
| `df` | `Df { names: Vec<String>, cols: Vec<R0Val> }` | Per-column | `cols.first().map_or(0, R0Val::len)` |
| `list` | `List { names: Vec<String>, cols: Vec<R0Val> }` | Per-field | `cols.first().map_or(0, R0Val::len)` |
| `NULL` | `Null` | N/A | `0` |

### 2.2 Lazy Ranges and Materialization

- **`a:b` operator:** Evaluates `a` and `b` via integer materialization (`materialize_int`). Both bounds must have length 1 and must not be `NA_INT`; otherwise evaluation fails with `':' range operator requires non-NA length-1 integer bounds`. Sets `step = 1` when `start <= end` and `step = -1` when `start > end`.
- **`seq_len(n)` and `seq_along(x)`:**
  - `seq_len(n)` produces the 1-based sequence of length `n` (returning an empty integer sequence when `n == 0`, trapping on `n < 0` or `NA`, and returning `1:n` for `n >= 1`).
  - `seq_along(x)` produces the 1-based index sequence along `x` (`seq_len(length(x))`).
- **Overflow-safe length and materialization:** Range length and element calculation use widened `i64` arithmetic (`*start as i64 + i as i64 * *step as i64`), preventing `i32` subtraction or final-step addition overflow (e.g., `-2000000000L:2000000000L` and `2147483647L:2147483647L`).
- **Fuel and reservation checks:** Converting a `Range` to `Vec<i32>` (`materialize_int`) or `Vec<f64>` (`promote_dbl`) first charges `value.len()` fuel units in the VM and then calls `Vec::try_reserve_exact(n)`, returning `ErrFuelExhausted: range allocation exceeds available resources` if memory reservation fails.

### 2.3 Composite Types (`data.frame` and `list`)

- Both `data.frame(...)` and `list(...)` require all arguments to be named (`k = v`); unnamed arguments fail with `<func>() columns must be named (e.g. top = c(...))`.
- `data.frame(...)` enforces that every column has the same `.len()` as the first column; a mismatch traps with `ErrDimensionMismatch: data.frame column '<name>' has length <actual>, expected <expected>`.
- `list(...)` allows heterogeneous field lengths. Note that `R0Val::len()` on a `List` returns the length of its first field (or `0` if empty), not the number of fields.
- Field access (`x$name`) and 1-based column/element extraction (`x[[i]]`) are supported on `df` and `list`. Single-bracket subsetting (`x[i]`) and two-dimensional subsetting (`x[i, j]`) are not supported on `df` or `list`.

### 2.4 Display Formatting (`fmt::Display`)

- `Dbl`: `[1] ` followed by space-separated elements. `NA_real_` prints as `NA`, other NaNs print as `NaN`, infinities as `inf` / `-inf`, and finite numbers format with `{:.4}` with trailing `'0'`s and trailing `'.'` stripped.
- `Int`: `[1] ` followed by space-separated elements (`NA` for `NA_INT`, `<n>L` otherwise).
- `Lgl`: `[1] ` followed by `FALSE`, `TRUE`, or `NA`.
- `Chr`: `[1] ` followed by `"<s>"` for `Some(s)` or `NA` for `None`.
- `Range`: `[1] <start>:<end> (lazy range)`.
- `Df`: `<R0 df: <rows> rows x <cols> cols>` followed by lines ` $<name>: <col>`.
- `List`: `<R0 list: <fields> fields>` followed by lines ` $<name>: <col>`.
- `Null`: `NULL`.

---

## 3. Scope, Functions, and Evaluation

### 3.1 Environments and Variables

- **Strict evaluation:** Expressions and function arguments are evaluated eagerly left-to-right. Lazy promises, `<<-`, dynamic reflection, and mutable environments do not exist.
- **Variable lookup:** Variables are stored in `HashMap<String, R0Val>`. Referencing an unbound identifier fails at runtime with `Compile/Scope Error: Undeclared variable '<name>'`.
- **Script persistence:** `R0Vm::eval_script` clones `self.globals` into a working environment and writes the updated environment back to `self.globals` only if evaluation succeeds without error.

### 3.2 User-Defined Functions

- **Declaration:** Functions must be assigned to an identifier (`f <- function(...) ...` or `f <- fn(...) ...`). Evaluating an unassigned `FnDef` expression fails with `Anonymous functions must be bound to an identifier in R0`. A function declaration stores the `UserFn` in `self.functions` (separate from variable bindings in `env`) and evaluates to `Null`.
- **Closure capture:** At declaration time, `UserFn` captures a clone of the current variable environment (`captured_env: env.clone()`).
- **Locked prelude & dispatch:** Built-in intrinsics live in a locked prelude and cannot be shadowed by user function declarations; calls validate that all provided positional and named arguments bind to declared parameters (rejecting unknown, duplicate, or excess arguments).
- **Parameter binding:** Named arguments match declared parameter names first, remaining unbound parameters take positional arguments in order, and omitted optional parameters evaluate their default expressions in the call's local environment.
- **Type annotations:** Optional `: type` on parameters and `-> type` before the function body are accepted by the parser and ignored at runtime.

### 3.3 Control Flow

- **`if (cond) then_br [else else_br]` and `while (cond) body`:** `cond` must evaluate to a length-1 `Lgl` value that is `TRUE` or `FALSE`. `NA` traps with `ErrInvalidCondition: condition evaluated to NA in control flow`; any other type or length traps with `ErrInvalidCondition: if/while requires length-1 logical (lgl), got ...`. An `if` without `else` returns `Null` when `cond` is `FALSE`. A `while` loop returns the value of the last body evaluation, or `Null` if the body never ran.
- **`for (var in seq) body`:** `seq` must evaluate to an atomic vector (`Dbl`, `Int`, `Lgl`, `Chr`) or `Range`; `Df`, `List`, and `Null` fail with `'for' requires an atomic vector or range sequence`. For each element `0..seq.len()`, `var` is bound in the current environment to a length-1 vector containing that element (`Range` elements are extracted as `Int` in $O(1)$ without materializing the full range). Returns the last body value, or `Null` if `seq.len() == 0`.
- **`return([expr])`:** `return(expr)` immediately unwinds out of the enclosing user function (or top-level script) with the evaluated value, even when nested inside an assignment RHS, unary/binary operand, index expression, condition, loop, or argument list (`[STATE-04]`). `return()` with no argument evaluates to `Null` (`[SPEC-03]`).
- **Unimplemented control flow:** `break` and `next` are not implemented.

---

## 4. Operators, Broadcasting, Missing Values, and Indexing

### 4.1 Strict Broadcasting (`broadcast_len`)

For binary vector operations on operands of lengths `la` and `lb`:
1. **Zero-length rule:** If `la == 0` or `lb == 0`, the result length is `0`.
2. **Equal lengths:** If `la == lb`, the result length is `la`.
3. **Scalar broadcast:** If `la == 1`, the result length is `lb`; if `lb == 1`, the result length is `la`.
4. **Mismatch trap:** Otherwise (`la != lb`, `la > 1`, `lb > 1`), execution traps with `ErrDimensionMismatch: Strict broadcasting forbids recycling length <la> against length <lb>`.

### 4.2 Unary and Binary Arithmetic

- **Unary `+x`:** Charges fuel for `x.len()` and returns numeric/integer operands unchanged (materializing or preserving integer ranges).
- **Unary `-x`:** Charges fuel for `x.len()`. Accepts `Dbl` (negates each element, preserving `NA_real_` and `NaN` bit payloads) and `Int` / `Range` (preserves `NA_INT`, negates other `i32` values). Any non-numeric type fails with `Unary '-' requires numeric or integer vector`. Note that `-1:3` parses as `(-1):3` (`Range { start: -1, end: 3, step: 1 }`).
- **Integer-preserving binary arithmetic (`[SPEC-02]`):** When both operands are `R0Val::Int`, `+`, `-`, `*`, `%%`, and `%/%` preserve `R0Val::Int`:
  - If either element is `NA_INT`, the result element is `NA_INT`.
  - For `%%` and `%/%`, if the divisor is `0`, the result element is `NA_INT`.
  - Otherwise, computation uses widened `i64` intermediates with R's floor-based division/modulo convention (`quotient` rounded toward $-\infty$, remainder `lhs - quotient * rhs`).
  - If the non-missing `i64` result lies outside `i32::MIN + 1 ..= i32::MAX`, evaluation traps with `ErrIntOverflow: integer arithmetic result out of range`.
- **Floating-point promotion and division/exponentiation:**
  - `/` and `^` always promote `Int`/`Range` operands via `promote_dbl` and return `R0Val::Dbl`.
  - Mixed `Int`/`Dbl` or any arithmetic involving a `Range` operand promotes both operands via `promote_dbl` and returns `R0Val::Dbl`.
  - In `Dbl` binary arithmetic (`+`, `-`, `*`, `/`, `^`, `%%`, `%/%`), if the first operand element is missing (`NA_real_` or `NaN`), its exact bit payload is returned; else if the second is missing, its exact payload is returned (`B8`). Note: `Dbl` `%%` uses IEEE remainder (`va % vb`) and `%/%` uses `(va / vb).floor()`.
  - Logical (`Lgl`), character (`Chr`), `Df`, `List`, and `Null` operands are rejected in arithmetic with `TypeError: Cannot implicitly promote ... to numeric (dbl)`.

### 4.3 Relational Comparisons (`==`, `!=`, `<`, `<=`, `>`, `>=`)

- Both operands are promoted via `promote_dbl` (accepting `Dbl`, `Int`, and `Range`; rejecting `Lgl`, `Chr`, `Df`, `List`, and `Null` with `TypeError`).
- Operands are broadcast with `broadcast_len`.
- If either element is missing (`NA_real_` or `NaN`), the result element is `LGL_NA`; otherwise it is `LGL_TRUE` or `LGL_FALSE`.
- Because comparisons produce `Lgl` and cannot take `Lgl` inputs, chained comparisons such as `1 < 2 < 3` parse left-associatively as `(1 < 2) < 3` and fail at runtime with `TypeError`.

### 4.4 Logical Operators (`!`, `&`, `|`, `&&`, `||`)

- **Vectorized `!`, `&`, `|`:** Require `R0Val::Lgl` operands (other types fail with `Logical '!' requires logical (lgl) vector` or `Logical '&' and '|' require logical (lgl) operands`). Apply three-valued Kleene truth tables (`KLEENE_NOT_LUT`, `KLEENE_AND_LUT`, `KLEENE_OR_LUT`):
  - `FALSE & NA` $\to$ `FALSE`; `TRUE & NA` $\to$ `NA`
  - `TRUE | NA` $\to$ `TRUE`; `FALSE | NA` $\to$ `NA`
  - `!NA` $\to$ `NA`
- **Short-circuit scalar `&&` and `||`:** Evaluate the left operand via `extract_scalar_cond` (requiring length-1 non-NA `Lgl`, else trapping with `ErrInvalidCondition`). If `&&` sees `FALSE` or `||` sees `TRUE`, the right operand is not evaluated. Otherwise the right operand is evaluated with `extract_scalar_cond`.

### 4.5 Missing Values (`NA_real_` vs `NaN`)

- `NA_real_` is the IEEE 754 quiet NaN with bits `0x7ff8_0000_0000_07a2` (`is_na_real(x)` checks `x.is_nan() && (x.to_bits() & 0xffff_ffff) == 1954`).
- `is_missing(x)` is true for both `NA_real_` and any other `NaN` (`x.is_nan()`).
- `is.na(x)` returns `TRUE` for `NA_real_`, `NaN`, `NA_integer_`, `NA` (`Lgl`), and `NA_character_` (`None`), and `FALSE` for all elements of a `Range`.
- `is.nan(x)` returns `TRUE` only for `Dbl` elements that are `NaN` and not `NA_real_`; it returns `FALSE` for `NA_real_` and for all elements of `Int` or `Range` (`B3`, `B4`). Calling `is.nan` on `Lgl`, `Chr`, `Df`, `List`, or `Null` fails with `Unsupported type for is.nan()`.

### 4.6 Indexing (`x[i]`, `x[[i]]`, `x$name`) and Subset Assignment (`x[i] <- v`)

- **Vector subsetting `x[i]`:**
  - `NULL[i]` returns `NULL`. `Df` and `List` fail with `TypeError: '[' requires an atomic vector; use '[[' or '$' on df/list`. A `Range` base is materialized to `Int`.
  - **Logical mask (`i` is `Lgl`):** Mask length must equal `x.len()` or `1`, else `ErrDimensionMismatch`. `TRUE` selects the element, `NA` inserts a typed missing value, and `FALSE` omits the element.
  - **Numeric/integer indices (`i` is `Int`, `Dbl`, or `Range`):** Converted via `materialize_int` (`Dbl` values truncate toward zero; `NaN`, `NA_real_`, or values outside `i32::MIN + 1 ..= i32::MAX` become `NA_INT` (`[STD-04]`)).
    - **Negative drop mode:** If any index is `< 0` (and not `NA_INT`), mixing with any positive (`> 0`) or `NA_INT` index traps with `ErrMixedSubscript`. Negative indices drop 1-based positions `(-k)` within `1..=n` (`0` and out-of-range negative indices are ignored).
    - **Positive / `NA` gather mode:** Indices in `1..=n` select `x[k - 1]`; `0`, `> n`, or `NA_INT` produce a typed missing value (`NA_real_`, `NA_INT`, `LGL_NA`, or `None`).
- **Scalar / element extraction `x[[i]]`:**
  - Converts `i` via `materialize_int`. Requires `i` to have length 1, not be `NA_INT`, and satisfy `1 <= i <= x.len()`; otherwise traps with `ErrBoundsTrap: '[[i]]' requires a valid 1-based scalar index`.
  - On `Df` or `List`, returns the `i`-th column/field (`R0Val`). On atomic vectors or `Range`, returns the `i`-th element as a length-1 vector.
- **Field projection `x$name`:**
  - Supported on `Df` and `List`. Returns a clone of the matching column/field, or fails with `Unknown column '$<name>'`. On other types, fails with `'$' projection requires a df or list`.
- **Subset assignment `x[i] <- v`:**
  - Target must be a simple identifier `x[i]` (nested targets like `x[1][1] <- v` or `d$a[1] <- v` fail with `Complex nested subset assignment target not supported` or `Invalid assignment target`).
  - `x` must be an atomic vector or `Range` (`Range` materializes to `Int`); `Null`, `Df`, and `List` fail with `TypeError`.
  - Type compatibility: `x` and `v` must have the same atomic type, or one may be `Int` and the other `Dbl` (in which case the `Int` side promotes to `Dbl`). Other combinations fail with `TypeError: incompatible types in subset assignment`.
  - **Out-of-bounds extension (`B1`):** For non-logical indices, if the maximum positive index `maxk` exceeds `x.len()`, `x` is extended to `maxk` with typed missing values before assignment.
  - **Slot selection:** For a logical mask (length `x.len()` or `1`), `TRUE` positions are selected and both `FALSE` and `NA` positions are skipped. For numeric/integer indices, indices in `1..=n` map to `Some(k - 1)` and `NA_INT` or `<= 0` map to `None` (skipped, consuming one RHS slot when RHS is non-scalar; negative indices do not drop elements in `subset_assign`).
  - **RHS length:** `v.len()` must be `1` or equal to the number of selected slots (`slots.len()`), else `ErrDimensionMismatch`.

---

## 5. Operator Precedence and Syntactic Rewrites

The Pratt parser (`src/parser.rs`) implements these binding powers from highest to lowest:

| Precedence | Operators | Associativity | Binding Power `(l_bp, r_bp)` / Notes |
|---|---|---|---|
| 1 | `f(...)`, `x$name`, `x[i]`, `x[[i]]` | Left / Postfix | Function call parsed in prefix for `IDENT(...)`; postfix `[`, `[[`, `$` at bp `21` |
| 2 | `^` | Right | `(20, 19)` |
| 3 | Unary `+`, `-` | Prefix (right) | Prefix bp `20`; `-2^2` parses as `-(2^2)` |
| 4 | `:` | Left | `(19, 20)`; `-1:3` parses as `(-1):3` |
| 5 | `%%`, `%/%`, `%in%` | Left | `(17, 18)`; `x %in% t` rewrites to `Call("%in%", [x, t])` |
| 6 | `*`, `/` | Left | `(15, 16)` |
| 7 | `+`, `-` | Left | `(13, 14)` |
| 8 | `\|>`, `%>%` | Left | `(11, 12)`; rewrites at parse time |
| 9 | `==`, `!=`, `<`, `<=`, `>`, `>=` | Left | `(9, 10)` |
| 10 | `!` | Prefix (right) | Prefix bp `7` |
| 11 | `&`, `&&` | Left | `(5, 6)` |
| 12 | `\|`, `\|\|` | Left | `(3, 4)` |
| 13 | `<-`, `=` | Right | `(2, 1)` |

### Pipe Desugaring (`|>` and `%>%`)
- The right-hand side of `|>` or `%>%` must parse as a function call `f(...)`; otherwise parsing fails with `Right-hand side of '|>' pipe must be a function call`.
- If one or more top-level arguments of `f(...)` are the placeholder `_`, every such `_` is replaced with `lhs.clone()`. Otherwise, `lhs` is inserted as the first positional argument.
- Using `_` outside a pipe target fails at evaluation time with `Placeholder '_' is only valid inside a '|>' pipe`.
- Note: `%>%` is accepted as a lexer alias for `|>`; in base GNU R without packages, use `|>`.

---

## 6. Lexical Rules and Grammar

### 6.1 Newline Suppression and Delimiters (`src/lexer.rs`)
- Semicolons (`;`) always emit `Tok::Term`.
- Newlines (`\n`) emit `Tok::Term` unless suppressed by any of:
  1. An unclosed `(`, `[`, or `[[` on the delimiter stack (note: `{` does **not** push onto the delimiter stack, so newlines inside `{ ... }` terminate statements as expected).
  2. Immediately following the closing `)` of an `if (...)`, `while (...)`, `for (...)`, or `function(...)` / `fn(...)` header (`header_close`).
  3. An empty token stream or a preceding token that is `Tok::Term`, a binary/prefix operator, `<-`, `=`, `->`, `|>`, `%>%`, `(`, `[`, `[[`, `{`, `,`, or `else`.
- `[[` pushes `Open::Brack2`; `]]` is tokenized as `Tok::RBrack2` only when `Open::Brack2` is at the top of the delimiter stack, allowing nested indexing such as `x[[y[1]]]` and `a[b[1]]`.

### 6.2 Literals and Identifiers
- **Numbers:** Decimal digits with optional `.` fractional part and optional `e`/`E` exponent (`+`/`-`), or a leading `.` followed by digits (`.5`). An `L` suffix on an integer literal produces `Tok::IntLit(i32)` (overflow outside `i32` is a lex error). An `L` suffix on a floating-point literal (`1.5L`, `1e3L`) fails with `ParseError: float literal cannot have L suffix` (`B2`).
- **Strings:** Single- or double-quoted UTF-8 strings supporting `\"`, `\'`, and `\\` escapes (`[STATE-01]`).
- **Keywords & Sentinels:** `function`, `fn`, `if`, `else`, `for`, `in`, `while`, `return`, `TRUE`, `FALSE`, `NA`, `NA_real_`, `NA_integer_`, `NA_character_` (`[SPEC-01]`), `NaN`, `Inf`, and `_`. (`NULL` is not a lexer keyword; `Null` values arise from `return()`, `c()`, failed `if` without `else`, or function declarations.)
- **Identifiers:** Start with ASCII letter, `_`, or `.` (when not followed by a digit), followed by ASCII alphanumeric, `_`, or `.`.

### 6.3 Grammar Summary (`src/parser.rs`)

```ebnf
Program       = { Term } { Expr { Term } } ;
Block         = "{" { Term } { Expr { Term } } "}" ;
Expr          = ReturnExpr | IfExpr | ForExpr | WhileExpr | FnExpr | InfixOrPostfixExpr ;
ReturnExpr    = "return" "(" [ Expr ] ")" ;
IfExpr        = "if" "(" Expr ")" Expr { Term } [ "else" Expr ] ;
ForExpr       = "for" "(" IDENT "in" Expr ")" Expr ;
WhileExpr     = "while" "(" Expr ")" Expr ;
FnExpr        = ( "function" | "fn" ) "(" [ ParamList ] ")" [ "->" TOKEN ] Expr ;
ParamList     = Param { "," Param } ;
Param         = IDENT [ ":" TOKEN ] [ ( "<-" | "=" ) Expr ] ;
CallExpr      = IDENT "(" [ ArgList ] ")" ;
ArgList       = Arg { "," Arg } ;
Arg           = [ IDENT ( "<-" | "=" ) ] Expr ;
```

At evaluation time, `Assign { target, value }` requires `target` to be either `Ident(name)` or `Index1 { target: Ident(name), idx }`, and `FnDef` must be the RHS of `Ident(fname) <- FnDef`.

---

## 7. Intrinsics Inventory

### 7.1 Audit of Dispatched Intrinsics (70 Callable Names / 72 Capability Entries)

`R0Vm::call_intrinsic` (`src/vm.rs`) dispatches **70 distinct callable function names** (27 initial v0.2 intrinsics plus 12 G6 constructor, shape, coercion, repetition, and sequence intrinsics, plus 21 G7 predicate, logical, trig, rounding, and statistical/cumulative intrinsics, plus 10 G8 search, sorting, deduplication, and binning intrinsics). Earlier v0.1 and oracle documents (`docs/ADVERSARIAL_ORACLE.md` §4.3) refer to "29 implemented / 40 unimplemented of 69 specified": that historical 29-item tally counted variadic `c(...)`, named variadic `data.frame(...)`/`list(...)`, and integer-preserving `pmin`/`pmax` as separate capability entries while grouping numeric `pmin`/`pmax` together. Both views are shown below so the exact callable surface is unambiguous; all 69 specified R0 intrinsics (70 callable names across 72 capability entries) are now fully implemented.

#### Argument Binding Rules (`bind_args`)
- `c(...)` accepts any number of arguments (argument names are ignored).
- `data.frame(...)` and `list(...)` accept any number of arguments, all of which must be named.
- All other intrinsics bind named arguments by exact parameter name first, then fill remaining unbound positional slots up to `npos`. Passing an unknown argument name fails with `UnknownArgument: <name> in <func>`, duplicating a named argument fails with `DuplicateArgument: <name> in <func>`, and exceeding `npos` positional arguments fails with `TooManyArguments: <func> takes at most <npos> positional arguments`.
- **Important:** For `sum`, `mean`, `min`, `max`, `any`, `all`, `prod`, `var`, `sd`, `median` (`params = ["x", "na.rm"], npos = 1`) and `pmin`, `pmax` (`params = ["x", "y", "na.rm"], npos = 2`), `na.rm` is a named-only boolean flag (`na.rm = TRUE` or `na.rm = FALSE`, defaulting to `FALSE`). When provided, `na.rm` must be a non-empty `Lgl` vector whose first element is not `NA` (if `length(na.rm) > 1`, the first element is used; non-logical inputs trap with `TypeError`, and empty or `NA` first elements trap with `ErrInvalidCondition`).
- For `seq` (`params = ["from", "to", "by", "length.out", "along.with"], npos = 4`), `along.with` is a named-only argument since `npos = 4`.
- For `round` (`params = ["x", "digits"], npos = 2`), `digits` defaults to `0` if omitted; accepts a finite scalar integer or numeric (truncated to integer).
- For `atan2` (`params = ["y", "x"], npos = 2`), note parameter order: `y` first, then `x` (matching GNU R `atan2(y, x)`).
- For `diff` (`params = ["x", "lag"], npos = 2`), `lag` defaults to `1`; accepts a scalar integer or numeric $\ge 1$.
- For `approx` (`params = ["x", "y", "xout", "rule"], npos = 4`), `rule` defaults to `1`; accepts a scalar or length-2 vector of values in `{1, 2}` (`rule = 1` yields `NA` outside the range, `rule = 2` extrapolates boundary values).
- For `sort` and `order` (`params = ["x", "decreasing"], npos = 2`), `decreasing` defaults to `FALSE`; requires a length-1 non-NA `Lgl` value (`TRUE` or `FALSE`).
- For `match` (`params = ["x", "table", "nomatch"], npos = 3`), `nomatch` defaults to `NA_integer_`; accepts a scalar integer or numeric.
- For `tabulate` (`params = ["bin", "nbins"], npos = 2`), `nbins` defaults to $\max(1, \max(\text{bin}))$, ignoring values $\le 0$ and `NA`/`NaN`; if provided, must be a scalar finite non-negative integer or numeric.
- For `findInterval` (`params = ["x", "vec"], npos = 2`), both arguments are positional; `vec` must be sorted non-decreasingly with no missing values.

#### Implemented Callable Intrinsics (70 Names)

| # | Callable Name(s) | Signature (`npos`) | Behavior & Return Type |
|---|---|---|---|
| 1 | `c` | `c(...)` (variadic) | Concatenates atomic values (`Dbl`, `Int`, `Range`, `Lgl`, `Chr`, `Null`). `Null` arguments are skipped (`c()` $\to$ `Null`). `Int`/`Range` + `Dbl` promotes to `Dbl`. All-`NA` `Lgl` promotes to `NA` of the target type (`Chr` (`B5`), `Dbl`, or `Int`); non-`NA` `Lgl` mixed with `Chr` or numbers, or `Chr` mixed with numbers, fails with `TypeError`. Rejects `Df` and `List`. |
| 2 | `data.frame` | `data.frame(k = v, ...)` | Constructs `R0Val::Df`; requires named arguments and equal column lengths (`ErrDimensionMismatch` otherwise). |
| 3 | `list` | `list(k = v, ...)` | Constructs `R0Val::List`; requires named arguments. |
| 4 | `length` | `length(x)` (`1`) | Returns `R0Val::Int(vec![x.len() as i32])`. |
| 5 | `seq_len` | `seq_len(x)` (`1`) | Materializes scalar non-negative integer `n`; returns empty `Int` when `n == 0` and `Range { start: 1, end: n, step: 1 }` when `n >= 1`. |
| 6 | `seq_along` | `seq_along(x)` (`1`) | Returns empty `Int` when `x.is_empty()` and `Range { start: 1, end: x.len() as i32, step: 1 }` otherwise. |
| 7 | `is.na` | `is.na(x)` (`1`) | Returns `Lgl` vector: `TRUE` for `NA_real_`, `NaN`, `NA_INT`, `LGL_NA`, and `None` (`NA_character_`, `B6`); `FALSE` for all `Range` elements. Rejects `Df`, `List`, `Null`. |
| 8 | `is.nan` | `is.nan(x)` (`1`) | Returns `Lgl` vector: `TRUE` only for non-`NA_real_` `NaN` in `Dbl` (`B3`, `B4`); `FALSE` for `NA_real_`, `Int`, and `Range`. Rejects `Lgl`, `Chr`, `Df`, `List`, `Null`. |
| 9 | `which` | `which(x)` (`1`) | Requires `Lgl` input; returns 1-based `Int` indices where `x == TRUE`. |
| 10 | `ifelse` | `ifelse(test, yes, no)` (`3`) | Requires `test: Lgl`. When `test.len() > 0`, `yes` and `no` must have length `1` or `test.len()` (`ErrDimensionMismatch` otherwise). Unifies `yes` and `no` via `common_concat`; yields `yes` on `TRUE`, `no` on `FALSE`, and typed `NA` on `LGL_NA`. |
| 11–12 | `pmin`, `pmax` | `pmin(x, y, na.rm = FALSE)` (`2` pos + named `na.rm`) | When both `x` and `y` are `Int`, preserves `Int` (`[SPEC-04]`). Otherwise converts `Int`/`Range`/`Dbl`/`Lgl` via `as_f64` and returns `Dbl`. With `na.rm = FALSE`, single missing returns that missing operand and both missing returns the second operand (`B9`); with `na.rm = TRUE`, single missing returns the non-missing operand and both missing returns `NA_INT` / `NA_real_`. |
| 13 | `abs` | `abs(x)` (`1`) | Preserves `Int` for `Int` input (`NA_INT` $\to$ `NA_INT`). For `Dbl`/`Range`/`Lgl`, converts via `as_f64` and returns `Dbl`, preserving `NA_real_` and `NaN` bit payloads (`[STD-01]`). |
| 14–19 | `sqrt`, `exp`, `log`, `log10`, `floor`, `ceiling` | `f(x)` (`1`) | Promotes `Dbl`/`Int`/`Range` via `promote_dbl` (rejects `Lgl`, `Chr`, etc.) and returns `Dbl`. Any missing input (`NA_real_` or `NaN`) maps to `NA_real_`. |
| 20–23 | `sum`, `mean`, `min`, `max` | `f(x, na.rm = FALSE)` (`1` pos + named `na.rm`) | For `Int`, `Lgl`, or `Range` input, `sum`, `min`, and `max` preserve `Int` (`sum` traps with `ErrIntOverflow` on `i32` overflow; if no non-NA elements remain in `min`/`max`, returns `Dbl([Inf])` / `Dbl([-Inf])`). `mean` always returns `Dbl` (`NA_real_` if count is 0). For `Dbl` input with `na.rm = FALSE`, `min`/`max` return `NA_real_` if any `NA_real_` is present, else the first `NaN` (`B10`); `sum`/`mean` return the first missing value encountered (`B10`). |
| 24 | `cumsum` | `cumsum(x)` (`1`) | Promotes `Dbl`/`Int`/`Range` via `promote_dbl` and returns `Dbl`. Once a missing value (`NA_real_` or `NaN`) is encountered, that exact missing payload propagates to all subsequent positions (`[STD-01]`). |
| 25 | `diff` | `diff(x, lag = 1)` (`2` pos) | Promotes `Dbl`/`Int`/`Range` via `promote_dbl` and returns `Dbl` of differences `x[i + lag] - x[i]` (length `x.len().saturating_sub(lag)`). Supports scalar integer/numeric `lag >= 1` (default 1). Missing value precedence: preserves `x[i + lag]` payload if missing, else `x[i]`. Returns empty `Dbl` if `x` is `Null` or `length(x) <= lag`. Consumes `x.len()` fuel. |
| 26 | `approx` | `approx(x, y, xout, rule = 1)` (`4` pos) | Promotes `x`, `y`, `xout` via `promote_dbl` (`x.len() == y.len()` required, else `ErrDimensionMismatch`). Linearly interpolates `y` at `xout`. `rule` accepts length 1 or 2 with values in `{1, 2}` (default 1). For `xout < x[0]`, yields `y[0]` if `rule_left == 2` else `NA_real_`; for `xout > x[last]`, yields `y[last]` if `rule_right == 2` else `NA_real_`. Empty `x` or missing `xout` yields `NA_real_`. Consumes fuel for linear search and interpolation. |
| 27 | `%in%` | `x %in% table` (`2`) | Also callable as `` `%in%`(x, table) ``. For `(Chr, Chr)`, matches exact `Option<String>` equality (`NA_character_` matches `NA_character_`). Mixing `Chr` with non-`Chr` fails with `TypeError`. Otherwise converts via `as_f64` (`Null` table treated as empty) and matches `NA_real_` only to `NA_real_` and `NaN` only to `NaN` (`[STD-01]`). Returns `Lgl`. |
| 28–31 | `numeric`, `integer`, `logical`, `character` | `f(length = 0)` (`1`) | Constructs an atomic vector of specified length initialized to defaults (`0.0`, `0`, `FALSE`, `""`). Accepts non-negative finite scalar `length` (default `0`). Errors on negative, `NA`, `NaN`, `Inf`, or `length != 1`. Consumes `n` fuel units before allocation. |
| 32–33 | `nrow`, `ncol` | `f(x)` (`1`) | Returns row or column count as a scalar `Int` for `DataFrame` inputs (`0` if empty). For atomic vectors, lists, or `Null`, returns `Null` (matching GNU R behavior). Consumes 1 fuel unit. |
| 34–36 | `as.numeric`, `as.integer`, `as.logical` | `f(x)` (`1`) | Coerces atomic vector, range, or `Null` to target type. Consumes `x.len()` fuel units before allocation. Rejects `DataFrame` and `List` with `TypeError`.<br>• `as.numeric`: `Int`/`Range` promote to `Dbl`; `Lgl` maps `TRUE` $\to$ `1.0`, `FALSE` $\to$ `0.0`, `NA` $\to$ `NA_real_`; `Chr` parses float (`NA_real_` on parse failure or `NA`); `Null` yields empty `Dbl`.<br>• `as.integer`: `Range` materializes to `Int`; `Dbl` truncates toward zero (out-of-range, `NA_real_`, `NaN`, `Inf` $\to$ `NA_INT`); `Lgl` maps `TRUE` $\to$ `1`, `FALSE` $\to$ `0`, `NA` $\to$ `NA_INT`; `Chr` parses integer/float (`NA_INT` on failure); `Null` yields empty `Int`.<br>• `as.logical`: non-zero numbers map to `TRUE`, `0` to `FALSE`, `NA`/`NaN` to `LGL_NA`; `Chr` maps `"T"`/`"true"` (case-insensitive) to `TRUE`, `"F"`/`"false"` to `FALSE`, else `LGL_NA`; `Null` yields empty `Lgl`. |
| 37 | `rep_len` | `rep_len(x, length.out)` (`2`) | Replicates or truncates `x` cyclically to non-negative scalar `length.out`. Consumes `length.out` fuel units before allocation. Fails if `length.out > 0` and `x` is empty (`Null` or empty vector). Preserves input type (`Range` yields `Int`). |
| 38 | `rep` | `rep(x, times = 1, length.out = NULL, each = 1)` (`4`) | Replicates elements of atomic vector or range `x`. Supports scalar `times`, vector `times` (matching `x.len()`, only when `each == 1`), scalar `each`, and `length.out` (takes precedence over `times`). Consumes output length fuel before allocation. Errors on negative parameters, invalid combinations, or empty `x` with `length.out > 0`. |
| 39 | `seq` | `seq(from = 1, to = 1, by = ..., length.out = NULL, along.with = ...)` (`4` pos + named `along.with`) | Generates numeric sequences. Supports `seq(n)` (1-argument form returning `1..n` when $n \ge 1$, `1..n` step -1 when $n \le 0$, empty `Int` for empty vector, or `1..length(from)` when `from.len() > 1`), `seq(from, to)`, `seq(from, to, by = ...)`, `seq(from, to, length.out = ...)`, and `seq(along.with = ...)`. Returns `Range`, `Int`, or `Dbl`. Consumes output length fuel before allocation. Errors on zero `by`, conflicting arguments, or non-numeric inputs. |
| 40–41 | `is.finite`, `is.infinite` | `f(x)` (`1`) | Returns `Lgl` vector matching input length (`x.len()`). For `Dbl`: `is.finite` is `TRUE` for finite non-missing floats (not `Inf`, `-Inf`, `NaN`, `NA_real_`); `is.infinite` is `TRUE` for `Inf` and `-Inf`. For `Int`: `is.finite` is `TRUE` for non-`NA_INT`, `FALSE` for `NA_INT`; `is.infinite` is all `FALSE`. For `Lgl`: `is.finite` is `TRUE` for `TRUE`/`FALSE`, `FALSE` for `LGL_NA`; `is.infinite` is all `FALSE`. For `Range`: `is.finite` is all `TRUE`; `is.infinite` is all `FALSE`. For `Chr`: all `FALSE`. For `Null`: empty `Lgl`. Rejects `Df` and `List` with `TypeError`. Consumes `x.len()` fuel. |
| 42 | `anyNA` | `anyNA(x)` (`1`) | Scalar `Lgl` reduction. Returns `TRUE` if any element of atomic vector `x` is missing (`NA_real_`, `NaN`, `NA_INT`, `LGL_NA`, or `NA_character_` (`None`)), else `FALSE`. Returns `FALSE` for empty vector, `Range`, and `Null`. Rejects `Df` and `List` with `TypeError`. Consumes `x.len()` fuel. |
| 43–44 | `any`, `all` | `f(x, na.rm = FALSE)` (`1` pos + named `na.rm`) | Scalar `Lgl` reductions implementing GNU R 3-valued Kleene logic for logical/numeric vectors (`Lgl`, `Int`, `Range`, `Dbl`, `Null`). Rejects `Chr`, `Df`, and `List` with `TypeError`.<br>• `any`: Returns `TRUE` if any element is truthy (non-zero / `TRUE`). If none are truthy, returns `LGL_NA` if any missing value was encountered and `!na.rm`, else `FALSE`. Returns `FALSE` on empty/`Null`.<br>• `all`: Returns `FALSE` if any element is falsy (zero / `FALSE`). If none are falsy, returns `LGL_NA` if any missing value was encountered and `!na.rm`, else `TRUE`. Returns `TRUE` on empty/`Null`. Consumes `x.len()` fuel. |
| 45–50 | `sin`, `cos`, `tan`, `acos`, `asin`, `atan` | `f(x)` (`1`) | Trigonometric functions. Promotes `Dbl`/`Int`/`Range` via `promote_dbl` (`Null` returns empty `Dbl`; rejects `Lgl`, `Chr`, `Df`, `List` with `TypeError`). Returns `Dbl`. Preserves missing value payloads (`NA_real_`, `NaN`). Out-of-domain inputs (e.g. `asin(2.0)`) produce IEEE `NaN`. Consumes `x.len()` fuel. |
| 51 | `trunc` | `trunc(x)` (`1`) | Truncates float values towards zero (`v.trunc()`). Promotes `Dbl`/`Int`/`Range` via `promote_dbl` (`Null` returns empty `Dbl`; rejects `Lgl`, `Chr`, `Df`, `List` with `TypeError`). Returns `Dbl`. Preserves `NA_real_` and `NaN` payloads. Consumes `x.len()` fuel. |
| 52 | `round` | `round(x, digits = 0)` (`2` pos) | Floating-point rounding with IEEE 754 round-to-even (`round_ties_even`). Promotes `x` via `promote_dbl` (`Null` returns empty `Dbl`). `digits` must be a scalar integer or numeric (finite, non-NA; defaults to 0). Rounds ties to nearest even number at $10^{-\text{digits}}$. Preserves `NA_real_`, `NaN`, and `Inf`. Clamped to identity for $\text{digits} \ge 300$ and `0.0` for $\text{digits} \le -300$. Consumes `x.len()` fuel. |
| 53 | `atan2` | `atan2(y, x)` (`2` pos) | Two-argument arctangent. Parameter order: `y` first, then `x` (matching R `atan2(y, x)`). Promotes `y` and `x` via `promote_dbl`. Returns empty `Dbl` if either argument is empty or `Null`. Strictly enforces R recycling: longer vector length must be an exact integer multiple of shorter vector length (`ErrDimensionMismatch` otherwise). Missing value precedence: preserves `y` payload if missing, else `x` payload if missing. Consumes $\max(\text{len}(y), \text{len}(x))$ fuel. |
| 54 | `prod` | `prod(x, na.rm = FALSE)` (`1` pos + named `na.rm`) | Product reduction. Converts `Int`, `Range`, `Dbl`, `Lgl` via `as_f64`. Returns scalar `Dbl`. With `na.rm = FALSE`, returns first encountered missing value payload if any missing value occurs; with `na.rm = TRUE`, missing values are filtered. Returns `1.0` for empty vector, `Null`, or when all elements are removed by `na.rm`. Consumes `x.len()` fuel. |
| 55–56 | `var`, `sd` | `f(x, na.rm = FALSE)` (`1` pos + named `na.rm`) | Sample variance and standard deviation. Converts `Int`, `Range`, `Dbl`, `Lgl` via `as_f64`. Returns scalar `Dbl`. With `na.rm = FALSE`, returns first encountered missing value payload. Filters missing values when `na.rm = TRUE`. If remaining non-missing count $n < 2$ (or on empty/`Null`), returns `NA_real_`. Computes $\sum (x_i - \bar{x})^2 / (n - 1)$ (`sd` returns square root). Consumes `x.len()` fuel. |
| 57 | `median` | `median(x, na.rm = FALSE)` (`1` pos + named `na.rm`) | Sample median. Converts `Int`, `Range`, `Dbl`, `Lgl` via `as_f64`. Returns scalar `Dbl`. With `na.rm = FALSE`, returns first encountered missing value payload. Filters missing values when `na.rm = TRUE`. Returns `NA_real_` if non-missing elements are empty (or on empty/`Null`). Sorts non-missing values and returns middle element (odd $n$) or average of two middle elements (even $n$). Consumes `x.len()` fuel. |
| 58 | `cumprod` | `cumprod(x)` (`1`) | Cumulative product. Converts `Int`, `Range`, `Dbl`, `Lgl` via `as_f64` (`Null` returns empty `Dbl`). Returns `Dbl`. Starts running product at `1.0`. Once a missing value (`NA_real_` or `NaN`) is encountered, that exact payload propagates stickily to all subsequent positions. Consumes `x.len()` fuel. |
| 59–60 | `cummin`, `cummax` | `f(x)` (`1`) | Cumulative minimum / maximum. Converts `Int`, `Range`, `Dbl`, `Lgl` via `as_f64` (`Null` returns empty `Dbl`). Returns `Dbl`. Once a missing value (`NA_real_` or `NaN`) is encountered, that exact payload propagates stickily to all subsequent positions. Consumes `x.len()` fuel. |
| 61–62 | `which.min`, `which.max` | `f(x)` (`1`) | Returns the 1-based index of the first minimum or maximum element as a scalar `Int`. Skips missing values (`NA`, `NaN`). Returns empty `Int` (`c()`) if `x` is empty, `Null`, or contains only missing values. Accepts `Int`, `Dbl`, `Lgl`, and `Range` ($O(1)$ range inspection). Rejects `Chr`, `Df`, and `List` with `TypeError`. Consumes `x.len()` fuel. |
| 63 | `match` | `match(x, table, nomatch = NA_integer_)` (`3` pos) | Returns 1-based index of the first match of each `x` in `table` as an `Int` vector, or `nomatch` (default `NA_integer_`) if not found. For `(Chr, Chr)`, matches exact string equality (`NA_character_` matches `NA_character_`; mixing `Chr` with non-`Chr` fails with `TypeError`). For numeric/logical vectors, matches with exact float semantics (`NA_real_` matches only `NA_real_`, `NaN` matches only `NaN`). `Null` table yields all `nomatch`; `Null` `x` yields empty `Int`. Consumes worst-case nested scan fuel $\text{len}(x) \cdot \text{len}(table) + \max(\text{len}(x), \text{len}(table))$. |
| 64–65 | `unique`, `duplicated` | `f(x)` (`1`) | Deduplication and duplicate detection across atomic vectors (`Int`, `Dbl`, `Lgl`, `Chr`, `Range`). Preserves first-seen order.<br>• `unique`: Returns atomic vector of distinct elements (preserves `Range` without materialization; `Null` returns `Null`). For `Dbl`, treats `+0.0` and `-0.0` as identical and distinguishes `NA_real_` from `NaN`.<br>• `duplicated`: Returns `Lgl` vector matching input length (`TRUE` for elements previously seen, `FALSE` for first appearance; `Range` returns all `FALSE`; `Null` returns empty `Lgl`). Consumes `x.len()` fuel. |
| 66 | `sort` | `sort(x, decreasing = FALSE)` (`2` pos) | Sorts atomic vector `x` (`Int`, `Dbl`, `Lgl`, `Chr`, `Range`). `decreasing` must be a length-1 non-NA logical flag (default `FALSE`). For `Range`, returns reversed or ascending `Range` without materialization. For vector types, drops all missing values (`NA_INT`, `NA_real_`, `NaN`, `LGL_NA`, `NA_character_`) and sorts remaining non-missing values in ascending or descending order. `Null` returns `Null`. Rejects `Df` and `List` with `TypeError`. Consumes $n \lceil \log_2(n) \rceil$ fuel. |
| 67 | `order` | `order(x, decreasing = FALSE)` (`2` pos) | Returns 1-based permutation indices as an `Int` vector that orders `x`. `decreasing` must be a length-1 non-NA logical flag (default `FALSE`). Stable sort for tied values. Missing values (`NA_INT`, `NA_real_`, `NaN`, `LGL_NA`, `NA_character_`) are always placed at the end in their original input order (`na.last = TRUE`), regardless of `decreasing`. Empty vector or `Null` returns empty `Int`. Consumes $n \lceil \log_2(n) \rceil$ fuel. |
| 68 | `rank` | `rank(x)` (`1`) | Returns sample ranks of atomic vector `x` as a `Dbl` vector. Tied values receive average ranks (`ties.method = "average"`). Missing values (`NA_INT`, `NA_real_`, `NaN`, `LGL_NA`, `NA_character_`) are placed last and assigned distinct consecutive ranks $(k + 1) \dots n$ in order of appearance (`na.last = TRUE`). Empty vector or `Null` returns empty `Dbl`. Rejects `Df` and `List` with `TypeError`. Consumes $n \lceil \log_2(n) \rceil$ fuel. |
| 69 | `tabulate` | `tabulate(bin, nbins = max(1, bin))` (`2` pos) | Frequency count of positive integers in `bin` into `1..nbins`. `bin` must be integer or numeric (`Int`, `Dbl`, `Range`, `Null`). `nbins` is an optional non-negative scalar integer/numeric; defaults to $\max(1, \max(\text{bin}))$, ignoring values $\le 0$ and missing values. Values outside $1 \dots \text{nbins}$, floats (truncated toward zero), and missing values are ignored. Returns `Int` vector of length `nbins`. Consumes $\text{len}(bin) + nbins$ fuel. |
| 70 | `findInterval` | `findInterval(x, vec)` (`2` pos) | Fast interval lookup for numeric/integer vector `x` against sorted breakpoint vector `vec`. `vec` must be non-decreasing and contain no `NA`/`NaN` (fails otherwise). Returns `Int` vector of indices $i \in 0 \dots \text{len}(vec)$ where $\text{vec}[i] \le x < \text{vec}[i+1]$ (0 if $x < \text{vec}[1]$). Missing values in `x` return `NA_integer_`. Consumes $\text{len}(x) \cdot \text{len}(vec) + \max(\text{len}(x), \text{len}(vec))$ fuel. |

### 7.2 Prelude Intrinsic Implementation Status (All 69 Specified Intrinsics Implemented)

All **69 specified prelude intrinsics** from the language specification and historical oracle inventory (`docs/ADVERSARIAL_ORACLE.md` §4.3)—corresponding to **70 callable function names** and **72 capability entries** dispatched in `R0Vm::call_intrinsic` (`src/vm.rs`)—are **fully implemented** in v0.2.

There are **0 unimplemented intrinsics** remaining in the specified prelude design. All 10 final search, sorting, deduplication, and binning intrinsics (`which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval`) as well as the signature extensions for `approx(rule)` and `diff(lag)` were delivered in G8. Calling any valid intrinsic name dispatches cleanly; calling an unrecognised function name fails deterministically with `Unknown function or intrinsic: '<name>'`.

---

## 8. Error Traps and Deterministic Fuel Metering

- **Standardized error prefixes:**
  - `ErrDimensionMismatch:` — incompatible vector lengths in broadcasting, logical mask indexing/assignment, `ifelse`, `approx`, or `data.frame` column construction.
  - `ErrInvalidCondition:` — non-scalar or `NA` condition in `if`, `while`, `&&`, or `||`.
  - `ErrMixedSubscript:` — mixing negative drop indices with positive or `NA` indices in `x[i]`.
  - `ErrBoundsTrap:` — out-of-bounds, non-scalar, or `NA` index in `x[[i]]`.
  - `ErrIntOverflow:` — integer `+`, `-`, `*`, `%%`, `%/%`, or `sum` result outside `i32::MIN + 1 ..= i32::MAX`.
  - `ErrFuelExhausted:` — instruction/element fuel budget exhausted (`self.fuel <= 0`) or `try_reserve_exact` failure during range materialization.
  - `ParseError:` — float literal with `L` suffix (`1.5L`, `1e3L`).
  - `TypeError:` — invalid type combinations in `c()`, `%in%`, `[` on `df`/`list`, subset assignment, or implicit promotion to `dbl`.
  - `UnknownArgument:`, `DuplicateArgument:`, `TooManyArguments:` — intrinsic argument binding errors.
- **Fuel accounting (`consume_fuel`):**
  - Every `R0Vm::eval` AST node step charges `1` unit (`units.max(1)` via saturating subtraction).
  - Vector operations, `materialize_int`, `promote_dbl`, `subset`, `subset_assign`, `common_concat`, `reduce`, and unary/math/predicate intrinsics charge fuel proportional to vector length *before* allocating or materializing ranges (`[STATE-03]`).
  - Nested scans charge for their worst-case comparison count: `%in%` charges `x.len().saturating_mul(table.len()).max(x.len()).max(table.len())` and `approx` charges `xout.len().saturating_mul(x.len().saturating_sub(1)).max(xout.len())` in addition to the initial length check.

---

## 9. Errata Ledger (v0.1 $\to$ v0.2 Clarifications)

- **B1 (Out-of-range subset assignment):** `x[i] <- v` with positive index `i > length(x)` extends `x` to length `i`, filling intermediate positions with typed `NA`.
- **B2 (Float literal with `L` suffix):** `1.5L` and `1e3L` are rejected by the lexer with `ParseError: float literal cannot have L suffix`.
- **B3 (`NA_real_` vs `NaN` distinction):** `NA_real_` uses R's `1954` payload (`0x7ff8_0000_0000_07a2`); other NaNs remain distinct, print as `NaN`, and are distinguished by `is.nan()` and `%in%`.
- **B4 (`is.nan(x)` intrinsic):** Implemented for `Dbl`, `Int`, and `Range`.
- **B5 (`NA` in character `c()`):** All-`NA` logical inputs promote to `NA_character_` (`None`) when `c()` produces `Chr`; non-`NA` logical inputs mixed with `Chr` remain `TypeError`.
- **B6 (`is.na` on `chr`):** `is.na(x)` returns `TRUE` for `NA_character_` elements and `FALSE` otherwise.
- **B8 (Binary arithmetic missing-value order):** In `Dbl` binary arithmetic, if the first operand is missing (`NA_real_` or `NaN`), its payload is returned; otherwise if the second operand is missing, its payload is returned.
- **B9 (`pmin`/`pmax` missing values):** With `na.rm = FALSE`, when both operands are missing the second operand's payload is returned; when one is missing, that missing operand's payload is returned. With `na.rm = TRUE`, a single missing operand is skipped and two missing operands yield `NA`.
- **B10 (`min`/`max` and `sum`/`mean` missing values):** With `na.rm = FALSE` on `Dbl`, `min` and `max` return `NA_real_` if any `NA_real_` occurs, else the first `NaN`; `sum` and `mean` return the first missing value (`NA_real_` or `NaN`) encountered.
- **`[SPEC-01]`..`[SPEC-04]`, `[STD-01]`..`[STD-04]`, `[STATE-01]`, `[STATE-03]`, `[STATE-04]`:** Remediated and verified in unit and conformance tests (`NA_character_` literal, integer-preserving binary ops and `pmin`/`pmax`, `return()`, `abs`/`cumsum`/`%in%` NaN handling, widened `Range` length/materialization, out-of-range `Dbl` $\to$ `NA_INT` cast, string escapes, pre-allocation fuel checks, and `return` unwinding from value subexpressions).

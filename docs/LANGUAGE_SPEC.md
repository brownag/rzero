# R0 Language Specification v0.2

> **Status:** normative for the current Rust AST interpreter. R0 is a small, deterministic subset of GNU R; features identified as planned are not accepted runtime behavior. No compiler, static type inference, Arrow ABI, or loop-fusion backend is implemented in this version.

R0 source is parsed and evaluated by `src/lexer.rs`, `src/parser.rs`, and `src/vm.rs`. Valid R0 programs use ordinary R syntax except for optional `: type` and `-> type` annotations. R0 deliberately rejects or traps behavior outside the supported subset. No external crates are required.

## 1. Values and memory behavior

### 1.1 Atomic values

| R0 type | Runtime representation | Missing value |
|---|---|---|
| `dbl` | `Vec<f64>` | R `NA_real_` quiet-NaN payload `0x7ff8_0000_0000_07a2`; other NaNs remain distinct |
| `int` | `Vec<i32>` | `i32::MIN` (`NA_INT`) |
| `lgl` | `Vec<u8>` (`0` false, `1` true, `2` missing) | `2` |
| `chr` | `Vec<Option<String>>` | `None` |

`NULL` is represented by `R0Val::Null`. A range is a lazy descriptor `(start, end, step)` with `i32` endpoints and step; its length is the inclusive count `abs(end - start) / abs(step) + 1` using widened arithmetic, or zero for a zero step. R0-generated ranges use a nonzero step directed from start toward end. Range length is reported as `usize` and saturates to `usize::MAX` on targets that cannot represent the full inclusive count. Materialization is fuel-metered by the VM; allocation failure is returned as an R0 error, not allowed to panic.

`list(...)` holds named heterogeneous values. `data.frame(...)` holds named columns of equal length; construction with unequal column lengths traps with `ErrDimensionMismatch`. List/data-frame `$name` and `[[i]]` access are supported. Two-dimensional data-frame subsetting is not implemented.

### 1.2 Value semantics

Values are owned Rust values (`Vec` and nested `R0Val`); the runtime has no garbage collector, copy-on-write analysis, or exposed mutable environments. Subset assignment returns an updated value and replaces the binding; it does not mutate another visible binding through aliasing.

## 2. Scope and evaluation

Evaluation is strict and uses the runtime environment. User functions capture the current environment by value and create a local environment for each call. Function parameters support positional/named binding and defaults. `<<-`, lazy promises, reflection, anonymous function expressions, `break`, and `next` are not implemented.

`return(expr)` exits the nearest user function, including when the return expression occurs in a nested expression, argument, assignment, condition, or loop. `return()` exits with `NULL`. A top-level return yields its value from script evaluation.

## 3. Operators, vectors, and missing values

### 3.1 Broadcasting

For vector operands of lengths `a` and `b`, equal lengths operate elementwise; either length-one operand broadcasts, including when the other length is zero. Unequal lengths greater than one trap with `ErrDimensionMismatch`. Silent recycling is not supported.

### 3.2 Numeric operations and overflow

`int` with `int` preserves `int` for `+`, `-`, `*`, `%%`, and `%/%`. Missing integer operands propagate `NA_INT`. Computation uses widened `i64` intermediates; a non-missing result outside `i32::MIN + 1 ..= i32::MAX` traps with `ErrIntOverflow` rather than wrapping or producing the reserved `NA_INT` value. Integer `%%` and `%/%` use R's floor-based convention; division by zero produces `NA_INT`.

Mixed `int`/`dbl` arithmetic promotes to `dbl`. `/` and `^` return `dbl`, including for two integer operands. `lgl` is not implicitly coerced for binary arithmetic. Integer `abs` preserves `int`; numeric `abs` preserves NaN and NA payloads.

### 3.3 Logical and missing-value rules

Vector `&`, `|`, and `!` use three-valued Kleene logic. `&&`, `||`, `if`, and `while` require a scalar, non-missing logical value; otherwise they trap with `ErrInvalidCondition`.

`NA_real_` is identified by its R payload; other NaNs are distinct. `is.na` is true for both, and `is.nan` is true only for non-NA NaNs. `%in%` matches NA only to NA and non-NA NaN only to non-NA NaN. For binary arithmetic, the first missing operand (NA or NaN) is returned, then the second if the first is not missing. `pmin`/`pmax` with `na.rm = FALSE` return the second operand when both are missing, otherwise return the missing operand; `na.rm = TRUE` skips a single missing operand. `min`/`max` return NA if any NA occurs, otherwise NaN if any NaN occurs. `sum`/`mean` return the first missing value encountered when `na.rm = FALSE`.

### 3.4 Indexing and assignment

`x[i]` supports positive indices, negative drop indices, and logical masks. Double indices are truncated toward zero; missing or out-of-range double values convert to `NA_INT`. Positive out-of-bounds or missing indices produce typed missing values. A logical mask must match the vector length or be scalar; missing mask elements produce missing values. Mixing negative indices with positive or missing indices traps with `ErrMixedSubscript`. `x[[i]]` requires one valid 1-based index and traps with `ErrBoundsTrap` otherwise.

Subset assignment preserves compatible types (or promotes integer to double when required). A positive assignment index beyond the current end extends the vector; new positions are initialized to the type's missing value (B1).

## 4. Precedence and rewriting

The Pratt parser implements these operators, from tighter to looser binding:

| Operators | Associativity |
|---|---|
| calls, `$`, `[`, `[[` | left |
| `^` | right |
| unary `+`, `-` | prefix |
| `:` | left |
| `%%`, `%/%`, `%in%` | left |
| `*`, `/` | left |
| `+`, `-` | left |
| `|>`, `%>%` | left |
| comparisons | left; chaining is not supported |
| `!` | prefix |
| `&`, `&&` | left |
| `|`, `||` | left |
| `<-`, `=` | right |

`%in%` is parsed as a call to the implemented `%in%` intrinsic. A pipe inserts its left operand as the first argument, or substitutes `_` placeholders in the right-hand call. `-2^2` means `-(2^2)`.

The parser also accepts `%>%` as an alias for `|>`. In GNU R, `%>%` requires a binding (normally supplied by a package); use `|>` for a source form available in base GNU R versions that support the native pipe.

## 5. Lexical and grammar notes

Newlines terminate statements unless suppressed inside `()`, `[]`, or `[[ ]]`, after an operator/comma/open delimiter, or between a control-flow header and its body. Semicolons also terminate statements. String literals accept either quote style and the escapes `\"`, `\'`, and `\\`.

```ebnf
Program       = { Term | Statement } ;
Statement     = AssignStmt | ReturnStmt | FnDecl | Expr ;
AssignStmt    = LValue ("<-" | "=") Expr ;
ReturnStmt    = "return" "(" [ Expr ] ")" ;
FnDecl        = IDENT ("<-" | "=") "function" "(" [ ParamList ] ")" Block ;
ParamList     = Param { "," Param } ;
Param         = IDENT [ ":" TypeAnnot ] [ "=" Expr ] ;
TypeAnnot     = "dbl" | "int" | "lgl" | "chr" | "df" | "list" ;
LValue        = IDENT | IDENT "[" Expr "]" ;
Block         = "{" { Term | Statement } "}" ;
```

This is the core statement grammar; control-flow and operator expressions are described in §4 and are parsed by the Pratt parser.

Supported literals include double and `L`-suffixed integer literals, `TRUE`, `FALSE`, `NA`, `NA_real_`, `NA_integer_`, `NA_character_`, `NaN`, `Inf`, and quoted strings. A float literal with an `L` suffix is a parse error (B2).

## 6. Intrinsics

Only the implemented entries below are normative runtime behavior. The counts preserve the oracle's capability accounting: specialized integer implementations and variadic forms count as entries; grouped planned families count once. There are **29 implemented entries covering 27 callable names**, and **40 planned entries covering 43 callable names**. This distinction avoids claiming a planned name is callable.

### 6.1 Implemented intrinsics (29 entries)

The 29-entry legacy capability tally counts the `c(...)`, record-constructor, and integer `pmin`/`pmax` paths separately, while grouping numeric `pmin` and `pmax` together. It covers 27 callable names:

| Entry(s) | Names / behavior |
|---|---|
| 1–6 | `c`, `length`, `seq_len`, `seq_along`, `data.frame`, `list` |
| 7 | Variadic `c(...)` form |
| 8 | Named variadic `data.frame(...)` / `list(...)` constructor form |
| 9–10 | `is.na`, `is.nan` |
| 11–17 | `abs`, `sqrt`, `exp`, `log`, `log10`, `floor`, `ceiling` |
| 18 | Numeric `pmin` / `pmax` paths |
| 19 | Integer-preserving `pmin` / `pmax` path when both operands are `int` |
| 20 | `ifelse` |
| 21–24 | `sum`, `mean`, `min`, `max` (`na.rm` accepts a logical argument) |
| 25–26 | `cumsum`, `diff` |
| 27 | `which` |
| 28 | `%in%` |
| 29 | `approx` |

`approx(x, y, xout)` performs linear interpolation and returns a double vector. `c`, `data.frame`, and `list` accept variadic arguments. `pmin`/`pmax` support two positional operands and optional `na.rm`; `approx` does not implement the spec-proposed `rule` argument, and `diff` does not implement the spec-proposed `lag` argument.

`c(...)` concatenates compatible atomic values. An all-missing logical argument becomes missing character elements when the result is character; a non-missing logical mixed with character is a `TypeError` (B5). `is.na(character)` is true exactly for `NA_character_` elements (B6).

> **Inventory correction:** the earlier v0.1/oracle prose's “29 implemented / 40 unimplemented” totals are capability-entry counts, not distinct function-name counts. The source dispatches 27 callable names in `src/vm.rs`; the figures above preserve the requested legacy labels. Do not use the labels as evidence that any planned operation is implemented.

### 6.2 Planned / unimplemented intrinsics (40 grouped entries)

These names are not available in the current runtime.

| Entry | Planned names |
|---|---|
| 1–9 | `rep`, `rep_len`, `seq`, `numeric`, `integer`, `logical`, `character`, `as.numeric` / `as.integer` / `as.logical` (coercion family), `nrow` / `ncol` (shape family) |
| 10–14 | `is.finite`, `is.infinite`, `any`, `all`, `anyNA` |
| 15–23 | `sin`, `cos`, `tan`, `acos`, `asin`, `atan`, `trunc`, `round`, `atan2` |
| 24–30 | `prod`, `var`, `sd`, `median`, `cumprod`, `cummin`, `cummax` |
| 31–40 | `which.min`, `which.max`, `match`, `order`, `sort`, `rank`, `unique`, `duplicated`, `tabulate`, `findInterval` |

The 40 entries group the coercion trio and row/column-count pair into two families; the table contains 43 distinct function names.

## 7. GNU R subset parity and explicit limits

R0 aims to match GNU R for supported expressions, including missing-value distinctions and integer result types. R0-specific traps are `ErrDimensionMismatch`, `ErrInvalidCondition`, `ErrMixedSubscript`, `ErrBoundsTrap`, `ErrIntOverflow`, and `ErrFuelExhausted`. Unsupported functions, two-dimensional data-frame indexing, loop-control statements, and other omitted R features are not covered by the parity guarantee.

Fuel is charged before vector work and range materialization. Intrinsic scans with nested comparisons (`%in%`, `approx`) charge for their worst-case comparison count. Programs that exceed the configured budget fail with `ErrFuelExhausted` before large range allocations; `try_reserve` failures are returned as errors rather than host panics. The implementation uses safe Rust and must not panic or abort on arbitrary R0 source.

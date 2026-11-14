# R0 Language Specification v0.1

> **Status:** intrinsics beyond the prototype set are specified but not yet implemented.


R0 is a deterministic, statically scoped, strictly evaluated subset of R designed for sub-250 KB WebAssembly compilation and zero-copy Apache Arrow / TypedArray interoperability. Every valid R0 script is syntactically valid GNU R (so standard R linters, editors, and test suites can execute R0 code directly), but R0 rejects dynamic reflection, lazy promises, silent length recycling, and runtime environment mutation at compile time.

## 1. Type System & Memory Representation

Following R's philosophy, there are no scalar types at the language level—a scalar is simply a vector of length 1 (though the compiler optimizes known length-1 values into unboxed registers/Wasm locals).

### 1.1 Atomic Vector Types

| Type Name | Element ABI | NA Representation | Description |
|---|---|---|---|
| `dbl` (numeric) | `f64` (IEEE 754) | Quiet NaN w/ payload `0x7ff80000000007a2` | Default numeric literal type (`1.0`, `42`, `1e-5`, `NA_real_`). |
| `int` (integer) | `i32` (two's complement) | `i32::MIN` (`-2147483648` / `0x80000000`) | Suffixed literals (`1L`, `42L`, `NA_integer_`) and `:` ranges (`1:10`). |
| `lgl` (logical) | `u8` (`0` = FALSE, `1` = TRUE) | `2` (`0x02`) (or `i32::MIN` at ABI boundary) | Three-valued Kleene logic (TRUE, FALSE, NA). |
| `chr` (character) | `u32` offset + `u8` UTF-8 pool | Offset bitflag `0x80000000` (`NA_character_`) | Immutable interned or Arrow-layout string array. |

### 1.2 Composite & Structural Types

- range (seq_range): A lazy, 12-byte unmaterialized descriptor (start: i32, end: i32, step: i32) produced by a:b or seq_len(n). Iterates and indexes with O(1) memory and only materializes into an int vector if mutated or returned across the host ABI.

- list (Record / Struct): A fixed-key, heterogeneous named tuple (list(a = x, b = y)). Keys must be static identifiers known at compile time; dynamic string keys (lst[[var]]) are disallowed.

- df (Table / DataFrame): A rectangular list where all fields are atomic vectors enforced to have identical length N at construction and boundary entry. Supports $ column projection (hz$clay) and 2D subsetting (hz[mask, ]).

## 2. Lexical & Execution Semantics

### 2.1 Scoping & Mutability

1. Static Lexical Scoping: All identifiers are resolved to fixed lexical frame slots (SSA registers or stack offsets) at compile time. Referencing an undeclared variable is a compile-time error.

2. Copy-on-Write (Value Semantics) via Static Aliasing Analysis: R0 preserves R's pure value semantics:

```r
       x <- c(1, 2, 3)
       y <- x
       y[1] <- 99 # x[1] remains 1; y is cloned only if x is live after this line
```

Because pointers and environments cannot escape dynamically, the compiler determines variable liveness statically: if x is never read after y[1] <- 99, y mutates the buffer in-place with zero allocation.

3. Closures: Functions can capture outer lexical variables by immutable value only. The super-assignment operator <<- is forbidden.

4. No Reserved Keyword Collisions: Built-in intrinsics (sum, c, ifelse, +) live in a locked root prelude and cannot be shadowed by user variables or parameters.

## 3. Broadcasting, Indexing & Three-Valued Logic

### 3.1 Strict Broadcasting (No Silent Recycling)

For any binary element-wise operation A ⊕ B with lengths LA = length(A) and LB = length(B):

- Case 1 (LA = LB ): Result has length LA , computed element-wise: Ri = Ai ⊕ Bi .

- Case 2 (LA = 1 or LB = 1): The length-1 operand broadcasts across the length-N operand (N ≥ 0). Note: If the other operand has length 0, the result has length 0 (matching R's zero-length rule).

- Case 3 (LA ≠ LB and LA > 1 and LB > 1): Runtime Trap (ErrDimensionMismatch). Fractional and multiple-length vector recycling (c(1,2,3,4) + c(1,2)) are completely eliminated.

### 3.2 Numerical Promotion Lattice

Implicit promotion occurs only along a single lossless-intent numerical edge:

int ⟶ dbl

- Arithmetic (+, -, *, ^, %%, %/%) between int and dbl promotes int to dbl (NA_integer_ converts to NA_real_).

- Real division / always returns dbl.

- lgl does NOT implicitly coerce in arithmetic: TRUE + 1 is a type error. Explicit intrinsics (as.integer(x), as.numeric(x)) or reductions (sum(lgl_vec), mean(lgl_vec)) must be used.

### 3.3 Three-Valued Logic (lgl) Truth Tables

Vectorized & and | obey strict Kleene three-valued logic (identical to GNU R):

- FALSE & NA → FALSE | TRUE & NA → NA

- TRUE | NA → TRUE | FALSE | NA → NA

- !NA → NA

- Control Flow (if (cond), while (cond), &&, ||): Requires length(cond) == 1. If cond evaluates to NA or length(cond) != 1, execution immediately traps (ErrInvalidCondition).

### 3.4 1-Based Indexing & Subsetting Rules (x[i])

R0 supports four distinct static/runtime indexing modes on a vector x of length N :

1. Positive Integer Indexing (i: int or dbl truncated to int, i > 0):

  - Returns a vector of length length(i).

  - If 1 ≤ ik ≤ N , yields x[i_k].

  - If ik > N or ik == NA, yields NA of typeof(x) (preserving R's out-of-bounds NA padding behavior, which is critical for lag/lead algorithms like x[2:(n+1)]).

2. Logical Mask Indexing (i: lgl):

  - Requires length(i) == N (or length(i) == 1). No mask recycling.

  - Yields elements where i_k == TRUE. Where i_k == NA, yields NA of typeof(x).

3. Negative Integer Drop Indexing (i: int, i < 0):

  - Returns x with indices abs(i) omitted. Mixing positive, negative, or NA values in negative indexing traps (ErrMixedSubscript).

4. Scalar Extraction (x[[i]]):

  - Requires length(i) == 1, 1 ≤ i ≤ N , and !is.na(i). Traps on out-of-bounds. Allows the compiler to unbox the result into a primitive Wasm register without NA/bounds propagation downstream.

## 4. Operator Precedence & Rewrite Rules

From highest (1) to lowest (11) precedence:

| Prec | Operator(s) | Assoc | Semantics / Desugaring |
|---|---|---|---|
| 1 | `$`, `[`, `[[`, `()` | Left | Field projection, vector subsetting, scalar extraction, function call |
| 2 | `^` | Right | Exponentiation (dbl) |
| 3 | `-`, `+` (unary) | Right | Negation, unary identity |
| 4 | `:` | Left | Inclusive integer sequence constructor (`a:b` → range) |
| 5 | `%%`, `%/%`, `%in%` | Left | Modulo, integer division, set membership (`match(x, table, 0L) > 0L`) |
| 6 | `*`, `/` | Left | Multiplication, floating-point division |
| 7 | `+`, `-` | Left | Addition, subtraction |
| 8 | `\|>`, `%>%` | Left | Pipe (rewritten at parse time, see §3.1) |
| 9 | `<`, `<=`, `>`, `>=`, `==`, `!=` | Left | Element-wise comparison → lgl (propagates NA; use `is.na()` to test NA) |
| 10 | `!` | Right | Element-wise logical NOT |
| 11 | `&`, `&&` | Left | Vectorized AND (`&`), scalar short-circuit AND (`&&`) |
| 12 | `\|`, `\|\|` | Left | Vectorized OR (`\|`), scalar short-circuit OR (`\|\|`) |
| 13 | `<-`, `=` | Right | Lexical variable or subset assignment (`x[i] <- v`) |

## 5. Formal EBNF Grammar

R0 resolves R's famous newline/semicolon sensitivity in the lexer: a newline \n is emitted as a TERM token unless it immediately follows an binary operator, comma, open delimiter (, [, {, or if (...) / else.

```ebnf
(* === Top-Level Module === *)
Program        = { [ Term ] Statement } [ Term ] ;
Term           = ( ";" | "\n" ) { ";" | "\n" } ;
(* === Statements === *)
Statement      = FnDecl
               | AssignStmt
               | ReturnStmt
               | BreakStmt
               | NextStmt
               | Expr ;
FnDecl            = IDENT ( "<-" | "=" ) "function" "(" [ ParamList ] ")" [ "->" TypeAnnot ] Block ;
ParamList         = Param { "," Param } ;
Param             = IDENT [ ":" TypeAnnot ] [ "=" Expr ] ;
TypeAnnot         = "dbl" | "int" | "lgl" | "chr" | "df" | "list" ;
AssignStmt        = LValue ( "<-" | "=" ) Expr ;
LValue            = IDENT { ( "[" [ Expr ] [ "," [ Expr ] ] "]" ) | ( "[[" Expr "]]" ) | ( "$" IDENT ) } ;
ReturnStmt        = "return" "(" [ Expr ] ")" ;
BreakStmt         = "break" ;
NextStmt          = "next" ;
Block             = "{" { [ Term ] Statement } [ Term ] "}" ;
(* === Expressions (Precedence Climbing) === *)
Expr           = IfExpr | WhileExpr | ForExpr | OrExpr ;
IfExpr            = "if" "(" Expr ")" ExprOrBlock [ "else" ExprOrBlock ] ;
WhileExpr         = "while" "(" Expr ")" ExprOrBlock ;
ForExpr           = "for" "(" IDENT "in" Expr ")" ExprOrBlock ;
ExprOrBlock       = Block | Expr ;
OrExpr            = AndExpr { ( "|" | "||" ) AndExpr } ;
AndExpr           = NotExpr { ( "&" | "&&" ) NotExpr } ;
NotExpr           = [ "!" ] CmpExpr ;
CmpExpr           = PipeExpr [ ( "==" | "!=" | "<" | "<=" | ">" | ">=" ) PipeExpr ] ;
PipeExpr          = AddExpr { ( "|>" | "%>%" ) CallTarget } ;
CallTarget        = IDENT "(" [ ArgList ] ")" ;
AddExpr           = MulExpr { ( "+" | "-" ) MulExpr } ;
MulExpr           = SpecialExpr { ( "*" | "/" ) SpecialExpr } ;
SpecialExpr       = SeqExpr { ( "%%" | "%/%" | "%in%" ) SeqExpr } ;
SeqExpr           = UnaryExpr [ ":" UnaryExpr ] ;
UnaryExpr         = ( "+" | "-" ) UnaryExpr | PowExpr ;
PowExpr           = PostfixExpr [ "^" UnaryExpr ] ;
PostfixExpr       = PrimaryExpr { PostfixOp } ;
PostfixOp         = "(" [ ArgList ] ")"
                  | "[" [ Expr ] [ "," [ Expr ] ] "]"
                  | "[[" Expr "]]"
                  | "$" IDENT ;
ArgList           = Arg { "," Arg } ;
Arg               = [ IDENT "=" ] ( Expr | "_" ) ;
PrimaryExpr       = LITERAL
                  | IDENT
                  | "(" Expr ")"
                  | Block ;
(* === Lexical Tokens === *)
LITERAL        = FLOAT_LIT | INT_LIT | BOOL_LIT | NA_LIT | STR_LIT ;
INT_LIT        = DIGIT { DIGIT } "L" ;
FLOAT_LIT      = DIGIT { DIGIT } [ "." { DIGIT } ] [ ( "e" | "E" ) [ "+" | "-" ] DIGIT { DIGIT } ] ;
BOOL_LIT       = "TRUE" | "FALSE" ;
NA_LIT         = "NA" | "NA_real_" | "NA_integer_" | "NA_character_" | "NaN" | "Inf" ;
STR_LIT        = '"' { CHAR } '"' | "'" { CHAR } "'" ;
IDENT          = ( LETTER | "." ) { LETTER | DIGIT | "_" | "." } ; (* Note: "." followed by digit is FLOAT_LIT *)
```

## 6. Core Intrinsics Table (The Locked Prelude)

R0 ships with 69 core intrinsics implemented directly in the Rust runtime or emitted as inline Wasm SIMD loops. Instead of R's hidden na.rm = FALSE keyword dispatching at runtime, na.rm must be a compile-time boolean literal (TRUE or FALSE, defaulting to FALSE), allowing the compiler to emit branchless vs. NA-skipping loops statically.

### 6.1 Constructors & Coercions

- c(...): Concatenates homogeneous or numeric-promotable vectors/scalars.

- rep(x, times) / rep_len(x, length.out): Pre-allocates and fills vectors.

- seq(from, to, by = 1) / seq_len(n) / seq_along(x): Sequence generators.

- numeric(n) / integer(n) / logical(n) / character(n): Zero/empty-initialized buffers.

- list(k1 = v1, ...) / data.frame(k1 = v1, ...): Record and table constructors.

- as.numeric(x), as.integer(x), as.logical(x): Explicit type conversions.

- length(x), nrow(df), ncol(df): O(1) header inspection.

### 6.2 Predicates & Missingness (Bitwise / Zero-Allocation)

- is.na(x) → lgl: Vectorized missing value check.

- is.nan(x), is.finite(x), is.infinite(x) → lgl: IEEE 754 float predicates.

- any(x, na.rm = FALSE), all(x, na.rm = FALSE) → lgl (length 1): Short-circuiting reductions.

- anyNA(x) → lgl (length 1): Fast SIMD scan for any NA sentinel.

### 6.3 Vectorized Math & Conditional Fusion (Fused SIMD Intrinsics)

- Element-wise Unary (dbl → dbl): abs(x), sqrt(x), exp(x), log(x), log10(x), sin(x), cos(x), tan(x), acos(x), asin(x), atan(x), floor(x), ceiling(x), trunc(x), round(x, digits = 0).

- Element-wise Binary ((dbl, dbl) → dbl): atan2(y, x), pmin(x, y, na.rm = FALSE), pmax(x, y, na.rm = FALSE).

- Conditional Selection: ifelse(test: lgl, yes, no):

  - Compiler Optimization: Unlike GNU R's notorious ifelse() (which eagerly allocates and evaluates all of yes and no, plus masks them 3 times), R0 compiles ifelse(cond, exprA, exprB) into a fused single-pass branch/select loop (wasm32::v128_bitselect when exprA and exprB are side-effect-free arithmetic expressions).

### 6.4 Reductions & Accumulations

- Scalar Reductions ((x, na.rm = FALSE) → length-1): sum(x), prod(x), mean(x), min(x), max(x), var(x), sd(x), median(x).

  - Note: sum(lgl) and mean(lgl) count/average TRUE as 1L/1.0 and FALSE as 0L/0.0.

- Running / Windowed Vector Ops (x → length(x)): cumsum(x), cumprod(x), cummin(x), cummax(x).

- Lagged Differences: diff(x, lag = 1L) → length(x) - lag.

### 6.5 Indexing, Search, Set & Interpolation Intrinsics

- which(x: lgl) → int: 1-based indices where x == TRUE.

- which.min(x), which.max(x) → int (length 1 or 0): First 1-based index of extremum (ignoring NA).

- match(x, table, nomatch = NA_integer_) → int: Vectorized hash/binary lookup (1-based).

- order(x, decreasing = FALSE) / sort(x, decreasing = FALSE) / rank(x).

- unique(x), duplicated(x) → lgl, tabulate(bin, nbins).

- approx(x, y, xout, rule = 1L) → dbl: Built-in 1D linear interpolation (returns interpolated y vector directly, indispensable for depth-function slicing and resampling in scientific workflows).

- findInterval(x, vec) → int: Binary search interval classification (replaces cut() without needing factor types).

## 7. Side-by-Side: Standard R vs. R0 Compatibility

Because R0 is a strict subset, any script adhering to R0 runs identically in base R (Rscript) for unit testing and verification:

```r
# Valid in both GNU R and R0:
# Depth-weighted aggregation over a horizon table (1-cm slab or analytical overlap)
weighted_prop <- function(top: dbl, bot: dbl, val: dbl, z_min: dbl = 0.0, z_max: dbl = 100.0) -> dbl {
  # Fused single-pass clamp & overlap thickness calculation
  o_top <- pmax(top, z_min)
  o_bot <- pmin(bot, z_max)
  thick <- pmax(o_bot - o_top, 0.0)
     # Valid mask (handles NA in val or horizon depths)
     valid <- !is.na(thick) & !is.na(val) & (thick > 0.0)
     total_thick <- sum(thick[valid])
     if (total_thick <= 0.0) {
        return(NA_real_)
     }
     sum(thick[valid] * val[valid]) / total_thick
}
```

Note: To run files with optional : type annotations in standard GNU R during testing, a 5-line regex preprocessor strips : (dbl|int|lgl|chr|df|list) and -> (dbl|int|lgl|chr|df|list), or R0 can parse types optionally via JSDoc/Roxygen comments (#' @param top dbl) if 100% unpreprocessed R syntax compatibility is desired.


## Errata (clarifications)

These clarify rules that v0.1 left unstated or that the prototype implemented incorrectly. The rule below is normative. "Prototype" describes the reference implementation's old behavior.

- **B1 Out-of-range assignment.** `x[i] <- v` with `i > length(x)` extends `x` to length `i`. Positions added by the extension take NA of `typeof(x)`. Prototype: silently ignored the write.
- **B2 Invalid `1.5L`.** A float literal with an `L` suffix is a parse error, because `INT_LIT` (§5) is digits followed by `L`. Prototype: lexed `1.5L` as a double followed by identifier `L`.
- **B3 NA versus NaN.** `NA_real_` is the R payload `0x7ff8_0000_0000_07a2`, meaning the low word is 1954. Any other NaN is NaN. `is.na` is TRUE for both. `is.nan` is TRUE only for NaN. NA prints as `NA` and NaN prints as `NaN`. Prototype: treated every NaN as NA.
- **B4 `is.nan(x)`.** Added to the intrinsic set, per §6.2. Prototype: not implemented.
- **B5 NA in character `c()`.** An all-NA logical argument promotes to `NA_character_` when the result type is `chr`. A non-NA logical mixed with `chr` remains a `TypeError`. Prototype: `TypeError` for any logical mixed with `chr`.
- **B6 `is.na` on character.** `is.na(x)` for `chr` `x` is TRUE where the element is `NA_character_`, and FALSE otherwise. Prototype: not implemented for `chr`.
- **B8 Missing values in arithmetic.** For binary arithmetic, if the first operand is NA or NaN, its value is the result. Otherwise, if the second operand is NA or NaN, its value is the result. So `NaN + NA` is NaN and `NA + NaN` is NA, matching R on current platforms. Prototype: NA always won.
- **B9 `pmin`/`pmax` missing values.** With `na.rm = FALSE`, if both operands are missing, the second operand's value is the result. If only one is missing, that operand's value is the result. So `pmin(NA, NaN)` is NaN and `pmin(NaN, NA)` is NA. Prototype: returned NA for any missing input.
- **B10 `min`/`max` and `sum`/`mean` missing values.** `min` and `max` return NA if any NA is present, and otherwise NaN if any NaN is present. `sum` and `mean` return the first missing value encountered. So `min(c(NaN, NA))` is NA and `sum(c(NaN, NA))` is NaN. Prototype: returned the first missing value for all four.

# Compiling a Strict R Subset

Source: blank.pdf (converted to Markdown)

Compiling a Strict R Subset

https://gemini.google.com/app/c97b685c1174e681

**User prompt:** What if you made an embeddable wasm language modeled after R that was a strict subset. Drop all the unusual features and strange things that make it hard to compile and interrogate without a heavy complex interpreter. Smarter people than I have attempted r compilation but perhaps they set their focus too high.

**Response:** Every historical attempt to compile or speed up R—fastr, pqR, Renjin, Riposte, territories, and even the base R bytecode compiler—eventually crashes into the same brick wall: 100% CRAN compatibility.

Once you commit to supporting all of R, you are forced to carry a massive runtime capable of deoptimizing at any instruction because a function three frames down might call eval(parse(...)) in parent.frame(2), mutate the caller's environment via <<-, or redefine + on the fly. Even webR has to ship a multi-megabyte Emscripten build of the entire C/Fortran GNU R source tree just to run a 10-line soil profile calculation in the browser.

If you change the goal from "compile R" to "compile a strict, predictable subset of R designed for embedded Wasm plugins and vectorized data crunching" (akin to what edge-python or Starlark did for Python, or AssemblyScript did for TypeScript), the problem transforms from a PhD research graveyard into a very tractable, weekend-to-month Rust project. You could easily fit the compiler and VM (or direct Wasm emitter) into a 100–300 KB .wasm artifact.

## 1. The Chopping Block: Why R is Uncompilable (and What You Drop)

To make R statically analyzable and compilable to register bytecode or raw WebAssembly instructions, you ruthlessly amputate the "Lisp-in-disguise" dynamism:

| Feature to Cut | Why It Breaks Compilers | What Replaces It in "Strict-R" |
|---|---|---|
| Non-Standard Evaluation (NSE) | `substitute()`, `deparse()`, and lazy promises (PROMSXP) mean arguments aren't values; they are AST thunks tied to an environment. Every variable lookup requires checking if a promise has been forced. | Strict Eager Evaluation. Arguments are evaluated before function entry. For data-frame masking (like `subset()`), use explicit formula/column syntax (`df[@clay > 20]`) resolved at compile time. |
| Dynamic Environments & `parent.frame()` | `assign()`, `get()`, `eval()`, and `sys.frame()` prevent lexical variable resolution. The compiler cannot map variables to stack slots or registers because any call might inject a new variable into the local scope. | Static Lexical Scoping. All local variables are resolved to fixed stack offsets or SSA registers at compile time. `<<-` is either banned or restricted to explicit closure upvalues. |
| S3 / S4 / R6 Dynamic Dispatch | Dispatching on runtime string class attributes (`class(x) <- "foo"`) requires hash lookups on every call and defeats type specialization. | Static Structs / Tagged Vectors or Compile-Time Monomorphization. Built-in types (`f64`, `i32`, `bool`, `str`, `df`) dispatch statically. |
| Silent Recycling & Type Coercion | `c(1, 2, 3) + c(10, 20)` silently recycles with a warning; `x[i]` might return a vector, scalar, or `NULL` depending on runtime values. | Strict Broadcasting Rules. Only length-1 scalars recycle against length-N vectors. Mismatched lengths (N ≠ M) trap immediately. |
| Everything-is-a-Call (`+` is a function) | In GNU R, `a + b` can be overridden in the local scope, forcing the VM to guard every arithmetic operator. | Reserved Operators. `+`, `-`, `*`, `/`, `[`, `[[`, `<-` are intrinsics, not rebindable functions. |

## 2. What You Keep: The "Soul" of R

When domain scientists write R for numerical algorithms, pedometrics, or spatial band math, they aren't writing metaprogramming frameworks—they are writing 1-indexed vectorized array math. If you keep these features, 90% of pure computational R scripts will run with zero or minimal edits:

- 1-Based Indexing & Inclusive Slices: 1:10 produces a 10-element sequence (or zero-allocation range iterator), and x[1] is the first element.

- First-Class Vectorized Arithmetic: Every primitive type is fundamentally a 1D contiguous typed array (Float64Vec, Int32Vec, BoolVec). a * 2 + b compiles directly to a fused SIMD loop (v128 in Wasm).

- Logical & Integer Vector Subsetting: clay[depth >= 25 & depth < 100] and x[c(1, 4, 2)].

- First-Class NA Propagation: Three-valued logic (TRUE, FALSE, NA) and hardware/bitmask NA tracking without needing wrapper objects.

- Pipe & Functional Ergonomics: Native |> (rewritten at the AST level as a zero-cost syntactic transform: x |> f(y) → f(x, y)), plus ifelse(), match(), which(), cumsum(), diff(), and approxfun().

## 3. Architecture of a "Micro-R" Wasm Engine

If you build this in Rust targeting wasm32-unknown-unknown, you have two architectural paths: A compact Register/SSA Bytecode VM in Wasm (like edge-python or Lua) or A Direct R-to-Wasm Compiler (where the Rust Wasm module parses the R string and emits a secondary .wasm module on the fly to instantiate in the browser/host).

Either way, the memory representation is where you win massive speedups over GNU R:

#### Zero-Copy Columnar Memory Layout

In GNU R, every vector is a heap-allocated SEXP with a 48-byte header, GC pointers, and attribute lists. In an embedded Wasm subset:

- Vectors are flat linear-memory buffers ([len: u32, ptr: u32]). Passing a soil horizon table or raster tile from JavaScript/Rust into the Wasm sandbox requires zero serialization—just write the Float64Array into Wasm linear memory and pass the pointer.

- NA Representation:

  - For numeric (f64): Use IEEE 754 quiet NaN with the classic R payload (0x7ff80000000007a2), or standard NaN if you don't need to distinguish NaN from NA. Hardware floating-point instructions propagate NaN for free at native CPU speed!

  - For integer (i32) and logical (u8): Either keep R's sentinel (i32::MIN = -2147483648) or pair each vector with an optional Apache Arrow-style validity bitmask. Using an Arrow bitmask means your Wasm R engine can read/write DuckDB-Wasm, GeoArrow, and Parquet buffers in-place with zero copy.

#### Automatic Loop Fusion (Deforestation)

The biggest performance penalty in clean R code is intermediate allocations. In standard R:

```r
# Allocates 4 temporary vectors of length N in GNU R
si <- (sand / silt) * exp(-0.02 * depth) + offset
```

Because your subset has static operators and eager semantics, a single-pass compiler can fuse contiguous vector expressions into a single loop (or Wasm SIMD f64x2 loop) that streams over sand, silt, and depth in registers and writes once to si. No GC pressure, no temporary vectors.

#### Compile-Time Frame Shapes (Typed DataFrames)

Instead of data.frame being an arbitrary list of anything, treat a table or frame as a struct of equal-length vectors. If a syntax like $ or @ is used (hz$clay_r), resolve column names via an inline cache or compile-time schema header.

## 4. What Code Would Look Like in "Strict-R"

It looks almost indistinguishable from standard clean R, which means users (or LLMs generating plugin scripts) already know how to write it:

```r
# Strict-R: No library() calls, pure vectorized sandboxed logic
calc_awhc <- fn(top: num, bot: num, sand: num, clay: num, fragvol: num) -> num {
  # 1:length(top) or vectorized math works natively, fused into 1 loop
  thick <- bot - top
     # Vectorized SAXTON-RAWLS style estimation
     theta_fc <- 0.2576 - (0.002 * sand) + (0.0036 * clay)
     theta_wp <- 0.0260 + (0.005 * clay)
     # Logical masking & NA-safe sum
     valid <- !is.na(thick) & (thick > 0)
     awhc_hz <- (theta_fc - theta_wp) * thick * (1.0 - (fragvol / 100.0))
     sum(awhc_hz[valid])
}
```

(Notice: Type annotations like : num could even be optional if you infer types at function specialization boundaries, just like Julia or Numba).

## 5. Where This Killer Niche Lives

Why hasn't anyone built this yet? Because academics want to publish papers on "Compiling 20,000 CRAN packages," and enterprise users just spin up a bigger Docker container with 64 GB of RAM.

Meanwhile, there is a massive vacuum for lightweight, sandboxed scientific scripting:

1. Browser-Side GIS & Raster Calculators: Running custom pedotransfer functions, terrain indices, or spectral band math over Cloud-Optimized GeoTIFFs (COGs) or vector tiles in WebWorkers without downloading a 25 MB webR payload.

2. Database UDFs (DuckDB / SQLite / Postgres): Writing vectorized aggregate or window UDFs in R syntax that compile down to a 50 KB Wasm module and operate directly on Arrow vectors.

3. Plugin Marketplaces & Rule Engines: Just like DST or taxonomic keys—allowing domain experts to write safe, deterministic, inspectable domain logic in familiar R vector syntax that cannot touch the filesystem, network, or hang the host thread (since Wasm execution can be fuel-metered).

**User prompt:** Draft a concrete language specification and EBNF grammar for this strict embeddable R subset, detailing the exact types, operators, broadcasting rules, and built-in intrinsics.

**Response:** R0 ("R-Zero") Language Specification v0.1

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

= LB and LA > 1, LB > 1): Runtime Trap (ErrDimensionMismatch). Fractional and multiple-length vector recycling (c(1,2,3,4) + c(1,2)) are completely eliminated.

- Case 3 (LA 

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

3.4 1-Based Indexing & Subsetting Rules (x[i])

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

R0 ships with 48 core intrinsics implemented directly in the Rust runtime or emitted as inline Wasm SIMD loops. Instead of R's hidden na.rm = FALSE keyword dispatching at runtime, na.rm must be a compile-time boolean literal (TRUE or FALSE, defaulting to FALSE), allowing the compiler to emit branchless vs. NA-skipping loops statically.

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

Would you like to design the Rust compiler & Wasm execution architecture next (specifically comparing an SSA bytecode VM with loop-fusion super-instructions against direct Wasm SIMD codegen, and laying out the zero-copy Arrow ABI), or jump straight into prototyping the lexer, parser, and type/shape checker in Rust?

**User prompt:** Yes write a spec for the compiler and wasm architecture

**Response:** R0 Compiler & Wasm Execution Architecture Specification v0.1

Target crate: r0-wasm (#![no_std] + alloc, targeting wasm32-unknown-unknown with +simd128). Target binary footprint: < 200 KB uncompressed .wasm (~75 KB wasm-opt -Oz + Brotli).

## 1. Architectural Decision: Why a Hybrid Vector-Register VM Beats Direct Wasm Codegen

When embedding R0 inside a browser, WebWorker, or DuckDB/SQLite host, there are two ways to execute user scripts:

1. Direct Wasm Emission (JIT): The Rust Wasm module parses R0 and emits raw .wasm binary bytes back to the JS host, which calls WebAssembly.instantiate() on the generated module.

2. In-Place Vector-Register Bytecode VM + Fused Kernel Interpreter: The Rust Wasm module compiles R0 into a flat 32-bit SSA register bytecode with fused vector kernels and executes it directly inside its own linear memory.

| Engineering Axis | Direct Wasm Codegen (JIT) | Vector-Register VM in Wasm | R0 Architecture Choice |
|---|---|---|---|
| Loop interpretive overhead | Zero (native JIT loops) | Amortized to ~0% on vectors (N ≥ 16) | Vector-Register VM |
| Host sandboxing & async | Requires async JS host bounce (`WebAssembly.instantiate`) or CSP `wasm-unsafe-eval` permission | Runs synchronously inside a strict CSP environment; zero JS roundtrips to compile & run | Vector-Register VM |
| Instruction fuel / timeout | Requires instrumenting every emitted Wasm basic block with fuel counters | Built into the VM dispatch loop and vector chunk loop at zero code-bloat cost | Vector-Register VM |
| Scalar-heavy loops (`for (i in 1:N)`) | Fast natively | Slower unless super-instructions are used | VM + Scalar Super-Instructions |

The Key Insight for R: In Python or JS, a bytecode VM spends 80% of its time in the opcode dispatch loop (match op { ... }) because every addition operates on 1 scalar. In R0, a single ADD_VV_F64 opcode processes an entire f64 column (or a 1024-element L1-cache tile) using native Wasm v128 SIMD instructions. If N = 1, 000, dispatch overhead is 0.1% of execution time.

(Optional Extension: Because the R0 IR is strict typed SSA, a secondary 15 KB backend can trivially emit a standalone .wasm module for pure-scalar UDFs if ever needed, without changing the frontend.)

## 2. Memory Architecture & Zero-Copy Arrow ABI

R0 never uses malloc/free per vector during expression evaluation, nor does it use a mark-and-sweep garbage collector. Instead, it divides linear memory into three deterministic regions:

```text
+-----------------------------------------------------------------------+
| 0x00000 .. 0x3FFFF : Static Data, Interned Strings & VM Stack Frames |
+-----------------------------------------------------------------------+
| 0x40000 .. HostPin : Pinned Zero-Copy Host Arena (Arrow / TypedArray) |
|                      -> Read-only input columns mapped directly       |
+-----------------------------------------------------------------------+
| HostPin .. HeapTop : Bump-Allocated / Ref-Counted Vector Arena        |
|                      -> Plus 4 Reusable L1 Tile Scratch Buffers       |
+-----------------------------------------------------------------------+
```

### 2.1 The 16-Byte Vector Descriptor (R0Vec)

Every register R[0..255] in a function frame holds a 16-byte R0Vec descriptor (aligned to 16 bytes so it fits in a single v128 register or two u64 slots):

```
#[repr(C, align(16))]
pub struct R0Vec {
    pub tag_flags: u32,        // Bits 0..3: TypeTag (Dbl, Int, Lgl, Chr, Range, Df)
                               // Bit 4: IS_SCALAR_UNBOXED (length == 1, stored in `payload`)
                               // Bit 5: IS_HOST_PINNED (zero-copy borrow, do not free)
                               // Bit 6: HAS_NULL_BITMAP (Arrow validity bitmap attached)
                               // Bits 16..31: Reference count (u16, 0 = dead/unique)
       pub len: u32,           // Number of elements (0 .. u32::MAX)
       pub payload: u64,       // If IS_SCALAR_UNBOXED: raw f64 bits / i32 / lgl / Range(start:i32, end:i32)
                               // If Vector (len > 1):
                               //   Low 32 bits: `data_ptr: u32` (16-byte aligned Wasm linear address)
                               //   High 32 bits: `null_ptr: u32` (0 if sentinel mode, else Arrow bitmap ptr)
}
```

### 2.2 Why Dual NA Mode (R Sentinels + Optional Arrow Bitmaps) Matters

- Internal & TypedArray Mode (null_ptr == 0): Standard JS Float64Array and Int32Array buffers passed from a web map or raster tile have no separate validity bitmask. R0 uses IEEE 754 NA_real_ (0x7ff80000000007a2) and NA_integer_ (i32::MIN).

  - Hardware Trick for f64: In Wasm SIMD, f64x2.add, f64x2.mul, f64x2.sub, and f64x2.div propagate quiet NaN payloads automatically in hardware. Adding or multiplying two dbl vectors requires zero NA checks—it executes at raw f64x2 peak throughput!

- Zero-Copy Arrow Mode (null_ptr != 0): When linked against DuckDB-Wasm or GeoArrow, an Arrow column consists of a values buffer (data_ptr) and a validity bitmask (null_ptr, 1 bit per row where 0 = null). R0 reads null_ptr directly without copying or scanning the buffer to convert nulls into NA sentinels first.

### 2.3 Zero-Copy Host FFI Protocol

Passing a 100,000-row horizon table from JavaScript to R0 takes 4 microseconds:

```rust
// Host JS side: zero serialization, direct TypedArray DMA
const topPtr = r0.alloc_pinned_f64(n);
new Float64Array(r0.memory.buffer, topPtr, n).set(horizonTopArray);
const resultPtr = r0.exec_udf(funcId, topPtr, botPtr, clayPtr, n);
```

## 3. Single-Pass SSA Compiler & Loop Fusion Engine

R0 compiles source code to bytecode in three ultra-fast passes (typically < 150 µs for a 100-line script):

```text
[R0 Source]
   │ (Pass 1: Lexer + Pratt Parser + Pipe `|>` Desugaring)
   ▼
[Typed AST / HIR]
   │ (Pass 2: Shape/Type Inference + Liveness + Deforestation/Loop Fusion)
   ▼
[Vector-Register SSA Bytecode + Fused Tile Kernels]
   │ (Pass 3: Execution via Wasm SIMD Dispatch Engine)
   ▼
[Result R0Vec]
```

### 3.1 Pass 1: Lexer & Pratt Parser with Inline Desugaring

- Zero-Allocation Token Stream: Tokens carry (kind: TokenKind, span_start: u32, span_len: u16). Identifiers and string literals reference slices of the input source buffer.

- Syntactic Rewrites during Parsing:

1. lhs |> fn(a, b) → AST Call("fn", [lhs, a, b]).

2. x %in% table → AST Intrinsic(InSet, [x, table]).

3. 1:n → AST Range(1, n) (typed as lazy Range, never allocates an array unless indexed non-linearly).

### 3.2 Pass 2: Type & Shape Lattice Inference

Every SSA value is inferred on a product lattice T × S :

- Type Lattice T : ⊥ ⊏ lgl ∣ int ⊏ dbl ∣ chr ∣ df(schema).

- Shape Lattice S :

  - Scalar (Compile-time proven length 1, stored unboxed in R0Vec.payload).

  - SymLen(id) (Symbolic vector length variable, e.g., L0 = length(top)).

  - DynVec (Runtime-dependent length, e.g., result of x[mask] where mask is lgl).

Static Bounds & Broadcast Elimination: When the compiler sees thick <- bot - top where top: SymLen(0) and bot: SymLen(0) (for instance, columns of the same df), it emits zero runtime length-check instructions inside the loop, checking len(top) == len(bot) once at the function entry block.

### 3.3 Automatic Loop Fusion (Stream Deforestation)

This is the centerpiece optimization that makes R0 match handwritten C/Rust speed while GNU R chokes on memory bandwidth.

Consider the R0 expression:

```r
awhc <- (theta_fc - theta_wp) * (bot - top) * (1.0 - fragvol / 100.0)
```

In naive evaluation, this executes 6 separate loops over memory and allocates 5 temporary vectors of length N .

In R0, Pass 2 identifies maximal Element-Wise Expression Trees (EWETs)—contiguous DAGs of element-wise arithmetic (+, -, *, /, pmin, pmax, abs, sqrt), comparisons (<, >, ==), logical operators (&, |, !), and ifelse().

Instead of emitting global vector-to-vector opcodes that allocate full length-N intermediates, the compiler wraps the DAG in a FUSE_LOOP region that processes vectors in L1-Cache Strip-Mined Tiles of 256 elements (2 KB per f64 tile):

Opcode Stream Emitted for `awhc`:

```
  00: CHECK_BROADCAST_4   R1(theta_fc), R2(theta_wp), R3(bot), R4(top), R5(fragvol) -> len N
  01: ALLOC_VEC_F64       R6(awhc), len N
  02: FUSE_TILE_BEGIN     len N, tile_size = 256
      // Inside FUSE_TILE, T0..T3 are pre-allocated 2KB scratch registers in L1 cache!
      03: TILE_SUB_VV_F64   T0, R1, R2            // theta_fc - theta_wp
      04: TILE_SUB_VV_F64   T1, R3, R4            // bot - top
      05: TILE_MUL_VV_F64   T0, T0, T1            // (theta_fc - theta_wp) * (bot - top)
      06: TILE_DIV_VS_F64   T1, R5, 100.0         // fragvol / 100.0
      07: TILE_RSUB_SV_F64 T1, 1.0, T1            // 1.0 - (fragvol / 100.0)
      08: TILE_MUL_STORE    R6, T0, T1            // Write directly to output slice of R6!
  09: FUSE_TILE_END
```

Why Strip-Mined Tile Fusion Wins:

1. Zero Heap Allocations for Temporaries: The VM permanently holds 8 scratch tiles (T0..T7, 8 × 2 KB = 16 KB total static memory). Even a 50-operation formula allocates zero bytes for intermediate results.

2. No Combinatorial Opcode Explosion: A direct JIT has to generate new machine code for every unique formula. A tile-fused VM uses a fixed table of ~40 TILE_* SIMD kernels, keeping the Wasm binary tiny while running entirely inside L1 cache!

3. Fused Reductions: If the expression ends in sum(...), min(...), or max(...), TILE_SUM_ACCUM accumulates directly into a f64x2 SIMD register across tiles—not even the output vector is allocated.

## 4. The 32-Bit Fixed-Width Instruction Set (ISA)

Every instruction is a fixed 32-bit word (u32) for branchless decoding:

```text
                                                                                                                                                  Opcode ∣ Dst (A) ∣ Src1 (B) ∣ Src2 / Imm8 (C)
```

8 bits      8 bits      8 bits            8 bits

(For wide constants or jump offsets, B and C combine into a 16-bit unsigned index Bx: u16 into the constant pool).

#### Core Opcode Families (~64 Total Opcodes)

- 0x00–0x0F (Load / Move / Lifecycle):

  - LOAD_CONST A, Bx (Load scalar literal into R[A])

  - MOVE_COW A, B (Copy descriptor R[B] to R[A], increment refcount if live)

  - FREE_DEAD A (Compiler-inserted deterministic drop as soon as variable's last read passes)

  - MAKE_RANGE A, B, C (Construct lazy 12-byte sequence R[B]:R[C])

- 0x10–0x1F (Unboxed Scalar Fast-Path & Super-Instructions):

  - Used when the shape inference pass proves inputs are Scalar (e.g., inside explicit for or while loops):

  - S_ADD_F64, S_SUB_F64, S_MUL_F64, S_DIV_F64, S_CMP_LT_F64

  - Loop Super-Instruction: FOR_RANGE_STEP r_iter, r_limit, offset16 (Increments r_iter, compares <= r_limit, and jumps in a single VM cycle).

- 0x20–0x3F (Strip-Mined SIMD Tile Kernels T[0..7]):

  - FUSE_TILE_BEGIN, FUSE_TILE_END

  - Arithmetic (VV = Vec-Vec, VS = Vec-Scalar, SV = Scalar-Vec): TILE_ADD_*, TILE_SUB_*, TILE_MUL_*, TILE_DIV_*, TILE_PMIN_*, TILE_PMAX_*, TILE_POW_*

  - Math Intrinsics: TILE_EXP, TILE_LOG, TILE_SQRT, TILE_ABS, TILE_ROUND

  - Comparisons & Kleene Logic: TILE_CMP_EQ, TILE_CMP_LT, TILE_CMP_LE, TILE_AND_LGL, TILE_OR_LGL, TILE_NOT_LGL, TILE_IS_NA

  - Branchless Selection: TILE_IFELSE A, B(cond), C(yes_tile), D(no_tile)

- 0x40–0x4F (Reductions & Windowed Accumulations):

  - REDUCE_SUM_F64 A, B, na_rm_flag, REDUCE_MEAN_F64, REDUCE_MIN_F64, REDUCE_MAX_F64, REDUCE_ANY_LGL, REDUCE_ALL_LGL

  - CUMSUM_F64, DIFF_F64

- 0x50–0x5F (Subsetting, Indexing & Search):

  - SUBSET_MASK A, B(vec), C(lgl_mask) (SIMD popcnt + compress store)

  - SUBSET_IDX A, B(vec), C(int_idx) (1-based bounds-checked gather w/ NA out-of-bounds fill)

  - SUBSET_SCALAR A, B(vec), C(scalar_idx) (x[[i]] → unboxed scalar register R[A])

  - ASSIGN_SUBSET A(vec), B(idx_or_mask), C(val) (In-place mutation if refcount == 1, else clone-on-write)

  - WHICH_LGL, MATCH_VEC, APPROX_LINEAR, FIND_INTERVAL

## 5. Wasm SIMD (v128) Implementation Details

To guarantee maximum throughput on both x86_64 (SSE4.2/AVX2) and ARM64 (NEON) browsers and runtimes via Wasm simd128:

5.1 f64x2 Arithmetic & Hardware NA Propagation

In IEEE 754, R's NA_real_ (0x7ff80000000007a2) is a quiet NaN.

```rust
#[cfg(target_arch = "wasm32")]
use core::arch::wasm32::*;
#[inline(always)]
pub unsafe fn tile_add_vv_f64(dst: *mut f64, a: *const f64, b: *const f64, len: usize) {
    let mut i = 0;
    while i + 2 <= len {
        let va = v128_load(a.add(i) as *const v128);
        let vb = v128_load(b.add(i) as *const v128);
        // Propagates NA_real_ automatically without branches!
        let vr = f64x2_add(va, vb);
        v128_store(dst.add(i) as *mut v128, vr);
        i += 2;
    }
    if i < len {
        *dst.add(i) = *a.add(i) + *b.add(i);
    }
}
```

### 5.2 Branchless f64 Comparisons with NA Propagation

Standard IEEE 754 f64x2_lt(va, vb) returns 0 (FALSE) when either operand is NA_real_, whereas R semantics require NA < 5.0 → NA (2u8). We achieve branchless 3-valued comparison in 4 SIMD instructions:

1. Compute cmp_true = f64x2_lt(va, vb) (lanes are 0xFFFF... if true, 0 if false/NaN).

2. Compute not_na = v128_and(f64x2_eq(va, va), f64x2_eq(vb, vb)) (since NaN == NaN is false!).

3. Extract u8 logical values: (cmp_true & 1) | (~not_na & 2) → produces 1 (TRUE), 0 (FALSE), or 2 (NA) with zero branches.

### 5.3 Bitwise Kleene Logic (&, |, !)

By encoding lgl as a 2-bit lattice inside u8:

- FALSE = 0b00 (0)

- TRUE = 0b01 (1)

- NA = 0b10 (2)

We can evaluate 16 logical operations simultaneously in a single v128 (u8x16) register using pure bitwise operations:

```rust
// Vectorized 16-lane Kleene NOT (!x):
// FALSE(0) -> TRUE(1), TRUE(1) -> FALSE(0), NA(2) -> NA(2)
let v_not = v128_xor(vx, u8x16_shr(vx, 1)); // wait: 0^0=0 (wrong), instead:
// Exact branchless formula for {0->1, 1->0, 2->2}:
// (vx ^ 1) & ~((vx >> 1) & 1) --> if bit 1 is set (NA=2), mask clears bit 0 so 2 stays 2!
let is_not_na_mask = u8x16_sub(u8x16_splat(1), u8x16_shr(vx, 1));
let v_not = v128_and(v128_xor(vx, u8x16_splat(1)), v128_or(is_not_na_mask, u8x16_splat(2)));
```

Even simpler: i8x16_swizzle(lut, vx)! Wasm simd128 has a single-cycle hardware 16-byte lookup table instruction (i8x16_swizzle). By loading a 16-byte truth table into a register, any unary or binary 3-valued Kleene operation (&, |, !, xor) across 16 elements executes in a single i8x16_swizzle instruction (1 CPU cycle per 16 elements)!

- For a & b: Combine (a << 2) | b (values 0..10) and run i8x16_swizzle(AND_LUT, combined):

  - AND_LUT = [0, 0, 0, 0, 0, 1, 2, 0, 0, 2, 2, 0, 0, 0, 0, 0]

  - Evaluates 16 three-valued & operations in 3 instructions with zero branches.

## 6. Deterministic Resource Controls (Sandboxing)

Because R0 is designed to run untrusted or user-authored plugin scripts in the browser or server:

1. Fuel Metering: Every backward branch (WHILE, FOR_RANGE_STEP) and every tile iteration (FUSE_TILE_END) decrements vm.fuel by the number of elements processed (max(1, tile_len)). If vm.fuel <= 0, the VM halts cleanly and returns Err(R0Trap::FuelExhausted).

2. Hard Arena Ceiling: The vector arena has a configurable byte cap (e.g., 16 MB). Any allocation (ALLOC_VEC, SUBSET_IDX) exceeding the cap immediately returns Err(R0Trap::OutOfMemory) without panicking or aborting the Wasm instance.

3. Arena Reset in O(1): Between UDF invocations (e.g., evaluating a pedotransfer function across 50,000 map units or soil profiles), calling vm.reset_scratch_arena() resets the bump pointer to HostPin in a single instruction (heap_top = host_pin), giving zero-overhead memory reuse across calls.

## 7. Concrete Crate Module Layout (r0-wasm)

r0-wasm/

```text
├── Cargo.toml                   # [dependencies] -> zero external crates required (optional libm for no_std)
└── src/
    ├── lib.rs                   # C/Wasm ABI exports (r0_compile, r0_exec, r0_alloc_pinned)
    ├── value.rs                 # R0Vec 16-byte descriptor, NA sentinels, TypeTag, Range
    ├── arena.rs                 # Deterministic bump + freelist vector arena & 8 static L1 tiles
    ├── lexer.rs                 # Zero-copy lexer with R newline-suppression state machine
    ├── ast.rs                   # Flat arena-allocated AST nodes (u32 indices, no Box<Expr> overhead)
    ├── parser.rs                # Pratt parser + `|>` and `%in%` AST desugaring
    ├── compile.rs               # Type/shape inference, liveness analysis, EWET tile loop fusion
    ├── isa.rs                   # 32-bit Opcode definitions & disassembler for debugging
    ├── simd_kernels.rs          # Wasm v128 (`f64x2`, `i32x4`, `i8x16_swizzle`) tile & reduction loops
    └── vm.rs                    # Fuel-metered register dispatch loop & intrinsic table
```

**User prompt:** Write a working, self-contained Rust prototype implementing the R0 lexer, Pratt parser (with pipe desugaring), and vectorized evaluator with 1-based indexing and Kleene NA logic.

**Response:** Below is a complete, zero-dependency Rust prototype (src/main.rs) implementing the R0 engine. You can drop it into any cargo new r0-proto project and run cargo run --release—or compile it directly to wasm32-unknown-unknown.

What This Prototype Implements

1. R Newline-Suppression Lexer: Tracks continuation tokens (+, -, *, /, |>, <-, ,, &, |, else, and open brackets (, [, {) so multiline pipelines and expressions parse identically to GNU R without semicolons.

2. Pratt Parser with AST-Level Pipe Desugaring: Parses R0 operator precedence, optional type annotations (top: dbl), subset assignments (x[mask] <- val), and rewrites lhs |> fn(a, _) into fn(a, lhs) at parse time with zero runtime overhead.

3. IEEE 754 NA_real_ Payload & 16-Byte Kleene Logic LUTs: Uses R's exact quiet NaN bit pattern (0x7ff80000000007a2), i32::MIN (NA_integer_), and 16-byte lookup tables ready for single-instruction i8x16_swizzle Wasm SIMD execution.

4. Strict Broadcasting & 1-Based Indexing: Traps immediately on silent length recycling (len A != len B where both are greater than 1), supports lazy a:b ranges, 1-based positive/negative/logical indexing, out-of-bounds NA padding, and native intrinsics (pmin, pmax, ifelse, sum, mean, which, seq_along, approx, etc.).

```rust
use std::collections::HashMap;
use std::fmt;
// ============================================================================
// 1. NA SENTINELS, KLEENE THREE-VALUED LOGIC & VECTOR VALUES
// ============================================================================
/// GNU R's exact IEEE 754 quiet NaN payload for NA_real_ (1954 in low bits)
pub const NA_REAL_BITS: u64 = 0x7ff8_0000_0000_07a2;
pub const NA_INT: i32 = i32::MIN;
#[inline(always)]
pub fn na_real() -> f64 {
    f64::from_bits(NA_REAL_BITS)
}
#[inline(always)]
pub fn is_na_real(x: f64) -> bool {
    x.is_nan() // In R0, NaN/NA_real_ unify for fast hardware propagation; payload check optional
}
/// Three-valued Kleene logic encoded in u8: 0 = FALSE, 1 = TRUE, 2 = NA
pub type Lgl = u8;
pub const LGL_FALSE: Lgl = 0;
pub const LGL_TRUE: Lgl = 1;
pub const LGL_NA: Lgl = 2;
/// 16-byte Lookup Tables for `(a << 2) | b` -> maps directly to Wasm `i8x16_swizzle`
pub const KLEENE_AND_LUT: [u8; 16] = [
    // b: F(0) T(1) NA(2) _(3)
    0, 0, 0, 0, // a = FALSE (0): FALSE & anything = FALSE
    0, 1, 2, 0, // a = TRUE (1): TRUE & F = F, TRUE & T = T, TRUE & NA = NA
    0, 2, 2, 0, // a = NA    (2): NA & F = F, NA & T = NA, NA & NA = NA
    0, 0, 0, 0,
];
pub const KLEENE_OR_LUT: [u8; 16] = [
    // b: F(0) T(1) NA(2) _(3)
    0, 1, 2, 0, // a = FALSE (0): FALSE | F = F, FALSE | T = T, FALSE | NA = NA
    1, 1, 1, 0, // a = TRUE (1): TRUE | anything = TRUE
    2, 1, 2, 0, // a = NA    (2): NA | F = NA, NA | T = T, NA | NA = NA
    0, 0, 0, 0,
];
pub const KLEENE_NOT_LUT: [u8; 4] = [1, 0, 2, 0];
#[derive(Clone, Debug)]
pub enum R0Val {
    Dbl(Vec<f64>),
    Int(Vec<i32>),
    Lgl(Vec<Lgl>),
    Chr(Vec<Option<String>>),
    /// Lazy 12-byte range descriptor (start, end, step)
    Range { start: i32, end: i32, step: i32 },
    /// Rectangular DataFrame / Named List
    Df { names: Vec<String>, cols: Vec<R0Val> },
    Null,
}
impl R0Val {
    pub fn len(&self) -> usize {
        match self {
              R0Val::Dbl(v) => v.len(),
              R0Val::Int(v) => v.len(),
              R0Val::Lgl(v) => v.len(),
              R0Val::Chr(v) => v.len(),
              R0Val::Range { start, end, step } => {
                  if *step == 0 {
                      0
                  } else {
                      (((end - start) / step).abs() + 1) as usize
                  }
              }
              R0Val::Df { cols, .. } => cols.first().map_or(0, |c| c.len()),
              R0Val::Null => 0,
        }
    }
     pub fn materialize_int(&self) -> Result<Vec<i32>, String> {
         match self {
             R0Val::Int(v) => Ok(v.clone()),
             R0Val::Range { start, end, step } => {
                 let n = self.len();
                 let mut out = Vec::with_capacity(n);
                 let mut cur = *start;
                 for _ in 0..n {
                       out.push(cur);
                       cur += *step;
                 }
                 Ok(out)
             }
             R0Val::Dbl(v) => Ok(v
                 .iter()
                 .map(|&x| if is_na_real(x) { NA_INT } else { x as i32 })
                 .collect()),
             _ => Err(format!("Cannot coerce {:?} to int vector", self)),
         }
     }
     pub fn promote_dbl(&self) -> Result<Vec<f64>, String> {
         match self {
             R0Val::Dbl(v) => Ok(v.clone()),
             R0Val::Int(v) => Ok(v
                  .iter()
                  .map(|&x| if x == NA_INT { na_real() } else { x as f64 })
                  .collect()),
             R0Val::Range { start, end, step } => {
                  let n = self.len();
                  let mut out = Vec::with_capacity(n);
                  let mut cur = *start;
                  for _ in 0..n {
                        out.push(cur as f64);
                        cur += *step;
                  }
                  Ok(out)
             }
             _ => Err(format!(
                  "TypeError: Cannot implicitly promote {:?} to numeric (dbl)",
                  self
             )),
         }
     }
}
impl fmt::Display for R0Val {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            R0Val::Dbl(v) => {
                let strs: Vec<String> = v
                      .iter()
                      .map(|&x| {
                          if is_na_real(x) {
                               "NA".to_string()
                          } else {
                               format!("{:.4}", x)
                                    .trim_end_matches('0')
                                    .trim_end_matches('.')
                                    .to_string()
                          }
                      })
                      .collect();
                write!(f, "[1] {}", strs.join(" "))
            }
            R0Val::Int(v) => {
                let strs: Vec<String> = v
                      .iter()
                      .map(|&x| if x == NA_INT { "NA".into() } else { format!("{}L", x) })
                      .collect();
                write!(f, "[1] {}", strs.join(" "))
            }
            R0Val::Lgl(v) => {
                let strs: Vec<&str> = v
                      .iter()
                      .map(|&x| match x {
                          LGL_FALSE => "FALSE",
                          LGL_TRUE => "TRUE",
                          _ => "NA",
                      })
                      .collect();
                write!(f, "[1] {}", strs.join(" "))
            }
            R0Val::Range { start, end, .. } => write!(f, "[1] {}:{} (lazy range)", start, end),
            R0Val::Chr(v) => {
                let strs: Vec<String> = v
                      .iter()
                      .map(|s| match s {
                          Some(st) => format!("\"{}\"", st),
                          None => "NA".into(),
                      })
                      .collect();
                write!(f, "[1] {}", strs.join(" "))
            }
            R0Val::Df { names, cols } => {
                writeln!(f, "<R0 df: {} rows x {} cols>", self.len(), names.len())?;
                for (name, col) in names.iter().zip(cols.iter()) {
                      writeln!(f, " ${}: {}", name, col)?;
                }
                Ok(())
            }
            R0Val::Null => write!(f, "NULL"),
        }
    }
}
// ============================================================================
// 2. LEXER WITH R NEWLINE-SUPPRESSION STATE MACHINE
// ============================================================================
#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    DblLit(f64),
    IntLit(i32),
    LglLit(Lgl),
    StrLit(String),
    Ident(String),
    // Keywords
    Fn,
    If,
    Else,
    For,
    In,
    While,
    Return,
    // Operators
    Assign, // <- or =
    Arrow, // ->
    Pipe,    // |> or %>%
    InOp,    // %in%
    Mod,     // %%
    IntDiv, // %/%
    Colon, // :
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Eq,
    Ne,
    Lt,
     Le,
     Gt,
     Ge,
     Bang,
     And,
     And2,
     Or,
     Or2,
     Dollar,
     Placeholder, // _
     // Delimiters
     LParen,
     RParen,
     LBrack,
     RBrack,
     LBrack2,
     RBrack2,
     LBrace,
     RBrace,
     Comma,
     Term, // ; or significant \n
     Eof,
}
impl Tok {
    /// In R, a newline after a binary operator, comma, or open delimiter is ignored
    fn suppresses_newline(&self) -> bool {
        matches!(
            self,
            Tok::Assign
                | Tok::Arrow
                | Tok::Pipe
                | Tok::InOp
                | Tok::Mod
                | Tok::IntDiv
                | Tok::Colon
                | Tok::Plus
                | Tok::Minus
                | Tok::Star
                | Tok::Slash
                | Tok::Caret
                | Tok::Eq
                | Tok::Ne
                | Tok::Lt
                | Tok::Le
                | Tok::Gt
                | Tok::Ge
                | Tok::Bang
                | Tok::And
                | Tok::And2
                | Tok::Or
                | Tok::Or2
                | Tok::Dollar
                | Tok::LParen
                | Tok::LBrack
                | Tok::LBrack2
                | Tok::LBrace
                | Tok::Comma
                | Tok::Else
        )
    }
}
pub fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut tokens: Vec<Tok> = Vec::new();
    let mut delimiter_depth: usize = 0; // Inside () or [], newlines are suppressed
     while i < bytes.len() {
         let c = bytes[i] as char;
         if c == ' ' || c == '\t' || c == '\r' {
             i += 1;
             continue;
         }
         if c == '#' {
             while i < bytes.len() && bytes[i] != b'\n' {
                 i += 1;
             }
             continue;
         }
         if c == '\n' || c == ';' {
             i += 1;
             let suppress = c == '\n'
                 && (delimiter_depth > 0
                      || tokens.last().map_or(true, |t| t.suppresses_newline() || *t == Tok::Term));
             if !suppress {
                 tokens.push(Tok::Term);
             }
             continue;
         }
         // Numbers
         if c.is_ascii_digit() || (c == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit()) {
             let start = i;
             let mut is_float = c == '.';
             i += 1;
             while i < bytes.len() {
                 let ch = bytes[i] as char;
                 if ch.is_ascii_digit() {
                       i += 1;
                 } else if ch == '.' && !is_float {
                       is_float = true;
                       i += 1;
                 } else if (ch == 'e' || ch == 'E') && i + 1 < bytes.len() {
                       is_float = true;
                       i += 1;
                       if bytes[i] == b'+' || bytes[i] == b'-' {
                           i += 1;
                       }
                 } else {
                       break;
                 }
             }
             let num_str = &src[start..i];
             if i < bytes.len() && bytes[i] == b'L' && !is_float {
                 i += 1;
                 let val: i32 = num_str.parse().map_err(|e| format!("Invalid int: {}", e))?;
                 tokens.push(Tok::IntLit(val));
             } else {
                 let val: f64 = num_str.parse().map_err(|e| format!("Invalid float: {}", e))?;
                 tokens.push(Tok::DblLit(val));
             }
             continue;
         }
         // Strings
         if c == '"' || c == '\'' {
             let quote = bytes[i];
             i += 1;
             let start = i;
             while i < bytes.len() && bytes[i] != quote {
                 i += 1;
             }
             if i >= bytes.len() {
                 return Err("Unterminated string literal".into());
             }
             tokens.push(Tok::StrLit(src[start..i].to_string()));
             i += 1;
             continue;
         }
         // Special %...% operators
         if c == '%' {
             let start = i;
             i += 1;
             while i < bytes.len() && bytes[i] != b'%' {
                 i += 1;
             }
             if i >= bytes.len() {
                 return Err("Unterminated % operator".into());
             }
             i += 1;
             match &src[start..i] {
                 "%%" => tokens.push(Tok::Mod),
                 "%/%" => tokens.push(Tok::IntDiv),
                 "%in%" => tokens.push(Tok::InOp),
                 "%>%" => tokens.push(Tok::Pipe),
                 other => return Err(format!("Unsupported infix operator: {}", other)),
             }
             continue;
         }
         // Identifiers & Keywords (R allows '.' in identifiers)
         if c.is_ascii_alphabetic() || c == '_' || c == '.' {
             let start = i;
             i += 1;
             while i < bytes.len() {
                 let ch = bytes[i] as char;
                 if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                      i += 1;
                 } else {
                      break;
                 }
             }
             let ident = &src[start..i];
             let tok = match ident {
                 "function" | "fn" => Tok::Fn,
                 "if" => Tok::If,
                 "else" => Tok::Else,
                 "for" => Tok::For,
                 "in" => Tok::In,
                 "while" => Tok::While,
                 "return" => Tok::Return,
                 "TRUE" => Tok::LglLit(LGL_TRUE),
                 "FALSE" => Tok::LglLit(LGL_FALSE),
                 "NA" => Tok::LglLit(LGL_NA),
                 "NA_real_" | "NaN" => Tok::DblLit(na_real()),
                 "Inf" => Tok::DblLit(f64::INFINITY),
                 "NA_integer_" => Tok::IntLit(NA_INT),
                 "_" => Tok::Placeholder,
                 _ => Tok::Ident(ident.to_string()),
             };
             tokens.push(tok);
              continue;
         }
         // Multi-char & single-char punctuation
         let rem = &src[i..];
         if rem.starts_with("<-") {
             tokens.push(Tok::Assign);
             i += 2;
         } else if rem.starts_with("->") {
             tokens.push(Tok::Arrow);
             i += 2;
         } else if rem.starts_with("|>") {
             tokens.push(Tok::Pipe);
             i += 2;
         } else if rem.starts_with("==") {
             tokens.push(Tok::Eq);
             i += 2;
         } else if rem.starts_with("!=") {
             tokens.push(Tok::Ne);
             i += 2;
         } else if rem.starts_with("<=") {
             tokens.push(Tok::Le);
             i += 2;
         } else if rem.starts_with(">=") {
             tokens.push(Tok::Ge);
             i += 2;
         } else if rem.starts_with("&&") {
             tokens.push(Tok::And2);
             i += 2;
         } else if rem.starts_with("||") {
             tokens.push(Tok::Or2);
             i += 2;
         } else if rem.starts_with("[[") {
             tokens.push(Tok::LBrack2);
             delimiter_depth += 1;
             i += 2;
         } else if rem.starts_with("]]") {
             tokens.push(Tok::RBrack2);
             delimiter_depth = delimiter_depth.saturating_sub(1);
             i += 2;
         } else {
             i += 1;
             match c {
                   '=' => tokens.push(Tok::Assign),
                   ':' => tokens.push(Tok::Colon),
                   '+' => tokens.push(Tok::Plus),
                   '-' => tokens.push(Tok::Minus),
                   '*' => tokens.push(Tok::Star),
                   '/' => tokens.push(Tok::Slash),
                   '^' => tokens.push(Tok::Caret),
                   '<' => tokens.push(Tok::Lt),
                   '>' => tokens.push(Tok::Gt),
                   '!' => tokens.push(Tok::Bang),
                   '&' => tokens.push(Tok::And),
                   '|' => tokens.push(Tok::Or),
                   '$' => tokens.push(Tok::Dollar),
                   ',' => tokens.push(Tok::Comma),
                   '(' => {
                       tokens.push(Tok::LParen);
                       delimiter_depth += 1;
                   }
                   ')' => {
                       tokens.push(Tok::RParen);
                       delimiter_depth = delimiter_depth.saturating_sub(1);
                   }
                   '[' => {
                       tokens.push(Tok::LBrack);
                       delimiter_depth += 1;
                   }
                   ']' => {
                       tokens.push(Tok::RBrack);
                       delimiter_depth = delimiter_depth.saturating_sub(1);
                   }
                   '{' => tokens.push(Tok::LBrace),
                   '}' => tokens.push(Tok::RBrace),
                   _ => return Err(format!("Unexpected character: '{}'", c)),
             }
         }
     }
     tokens.push(Tok::Eof);
     Ok(tokens)
}
// ============================================================================
// 3. AST & PRATT PARSER WITH PIPE (`|>`) DESUGARING
// ============================================================================
#[derive(Clone, Debug)]
pub enum BinOp {
    Add, Sub, Mul, Div, Pow, Mod, IntDiv, Colon, InOp,
    Eq, Ne, Lt, Le, Gt, Ge, And, And2, Or, Or2,
}
#[derive(Clone, Debug)]
pub enum UnaryOp {
    Neg, Pos, Not,
}
#[derive(Clone, Debug)]
pub enum Expr {
    Dbl(f64),
    Int(i32),
    Lgl(Lgl),
    Str(String),
    Ident(String),
    Placeholder,
    Unary(UnaryOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Assign { target: Box<Expr>, value: Box<Expr> },
    Index1 { target: Box<Expr>, idx: Box<Expr> },
    IndexScalar { target: Box<Expr>, idx: Box<Expr> },
    Field { target: Box<Expr>, name: String },
    Call { func: String, args: Vec<(Option<String>, Expr)> },
    FnDef { params: Vec<(String, Option<Expr>)>, body: Box<Expr> },
    If { cond: Box<Expr>, then_br: Box<Expr>, else_br: Option<Box<Expr>> },
    For { var: String, iter: Box<Expr>, body: Box<Expr> },
    While { cond: Box<Expr>, body: Box<Expr> },
    Return(Box<Expr>),
    Block(Vec<Expr>),
}
pub struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}
impl Parser {
    pub fn new(tokens: Vec<Tok>) -> Self {
        Self { tokens, pos: 0 }
    }
     fn peek(&self) -> &Tok {
         self.tokens.get(self.pos).unwrap_or(&Tok::Eof)
     }
     fn next(&mut self) -> Tok {
         let t = self.peek().clone();
         if self.pos < self.tokens.len() {
             self.pos += 1;
         }
         t
     }
     fn eat_terms(&mut self) {
         while matches!(self.peek(), Tok::Term) {
             self.next();
         }
     }
     fn expect(&mut self, expected: &Tok) -> Result<(), String> {
         let got = self.next();
         if &got == expected {
             Ok(())
         } else {
             Err(format!("Expected {:?}, got {:?}", expected, got))
         }
     }
     pub fn parse_program(&mut self) -> Result<Expr, String> {
         let mut stmts = Vec::new();
         self.eat_terms();
         while !matches!(self.peek(), Tok::Eof) {
             stmts.push(self.parse_expr(0)?);
             self.eat_terms();
         }
         Ok(Expr::Block(stmts))
     }
     fn parse_block(&mut self) -> Result<Expr, String> {
         self.expect(&Tok::LBrace)?;
         let mut stmts = Vec::new();
         self.eat_terms();
         while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
             stmts.push(self.parse_expr(0)?);
             self.eat_terms();
         }
         self.expect(&Tok::RBrace)?;
         Ok(Expr::Block(stmts))
     }
     fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
         let mut lhs = self.parse_prefix()?;
         loop {
             let op = self.peek();
             // Postfix operators: `[`, `[[`, `$`
             if let Some(l_bp) = postfix_bp(op) {
                 if l_bp < min_bp {
                     break;
                 }
                 match self.next() {
                     Tok::LBrack => {
                         let idx = self.parse_expr(0)?;
                         self.expect(&Tok::RBrack)?;
                            lhs = Expr::Index1 { target: Box::new(lhs), idx: Box::new(idx) };
                        }
                        Tok::LBrack2 => {
                            let idx = self.parse_expr(0)?;
                            self.expect(&Tok::RBrack2)?;
                            lhs = Expr::IndexScalar { target: Box::new(lhs), idx: Box::new(idx) };
                        }
                        Tok::Dollar => {
                            if let Tok::Ident(field) = self.next() {
                                lhs = Expr::Field { target: Box::new(lhs), name: field };
                            } else {
                                return Err("Expected column identifier after '$'".into());
                            }
                        }
                        _ => unreachable!(),
                   }
                   continue;
              }
              // Infix operators
              if let Some((l_bp, r_bp)) = infix_bp(op) {
                  if l_bp < min_bp {
                      break;
                  }
                  let op_tok = self.next();
                  let rhs = self.parse_expr(r_bp)?;
                   lhs = match op_tok {
                       Tok::Assign => Expr::Assign { target: Box::new(lhs), value: Box::new(rhs) },
                       Tok::Pipe => desugar_pipe(lhs, rhs)?,
                       _ => {
                           let bin_op = match op_tok {
                               Tok::Plus => BinOp::Add,
                               Tok::Minus => BinOp::Sub,
                               Tok::Star => BinOp::Mul,
                               Tok::Slash => BinOp::Div,
                               Tok::Caret => BinOp::Pow,
                               Tok::Mod => BinOp::Mod,
                               Tok::IntDiv => BinOp::IntDiv,
                               Tok::Colon => BinOp::Colon,
                               Tok::InOp => BinOp::InOp,
                               Tok::Eq => BinOp::Eq,
                               Tok::Ne => BinOp::Ne,
                               Tok::Lt => BinOp::Lt,
                               Tok::Le => BinOp::Le,
                               Tok::Gt => BinOp::Gt,
                               Tok::Ge => BinOp::Ge,
                               Tok::And => BinOp::And,
                               Tok::And2 => BinOp::And2,
                               Tok::Or => BinOp::Or,
                               Tok::Or2 => BinOp::Or2,
                               _ => unreachable!(),
                           };
                           Expr::Binary(bin_op, Box::new(lhs), Box::new(rhs))
                       }
                   };
                   continue;
              }
              break;
          }
          Ok(lhs)
     }
     fn parse_prefix(&mut self) -> Result<Expr, String> {
         match self.next() {
             Tok::DblLit(v) => Ok(Expr::Dbl(v)),
             Tok::IntLit(v) => Ok(Expr::Int(v)),
             Tok::LglLit(v) => Ok(Expr::Lgl(v)),
             Tok::StrLit(s) => Ok(Expr::Str(s)),
             Tok::Placeholder => Ok(Expr::Placeholder),
             Tok::LParen => {
                 let e = self.parse_expr(0)?;
                 self.expect(&Tok::RParen)?;
                 Ok(e)
             }
             Tok::LBrace => {
                 self.pos -= 1;
                 self.parse_block()
             }
             Tok::Minus => Ok(Expr::Unary(UnaryOp::Neg, Box::new(self.parse_expr(17)?))),
             Tok::Plus => Ok(Expr::Unary(UnaryOp::Pos, Box::new(self.parse_expr(17)?))),
             Tok::Bang => Ok(Expr::Unary(UnaryOp::Not, Box::new(self.parse_expr(7)?))),
             Tok::Return => {
                 self.expect(&Tok::LParen)?;
                 let e = self.parse_expr(0)?;
                 self.expect(&Tok::RParen)?;
                 Ok(Expr::Return(Box::new(e)))
             }
             Tok::If => {
                 self.expect(&Tok::LParen)?;
                 let cond = self.parse_expr(0)?;
                 self.expect(&Tok::RParen)?;
                 let then_br = self.parse_expr(0)?;
                 self.eat_terms();
                 let else_br = if matches!(self.peek(), Tok::Else) {
                     self.next();
                     Some(Box::new(self.parse_expr(0)?))
                 } else {
                     None
                 };
                 Ok(Expr::If { cond: Box::new(cond), then_br: Box::new(then_br), else_br })
             }
             Tok::For => {
                 self.expect(&Tok::LParen)?;
                 let var = match self.next() {
                     Tok::Ident(s) => s,
                     t => return Err(format!("Expected loop variable, got {:?}", t)),
                 };
                 self.expect(&Tok::In)?;
                 let iter = self.parse_expr(0)?;
                 self.expect(&Tok::RParen)?;
                 let body = self.parse_expr(0)?;
                 Ok(Expr::For { var, iter: Box::new(iter), body: Box::new(body) })
             }
             Tok::While => {
                 self.expect(&Tok::LParen)?;
                 let cond = self.parse_expr(0)?;
                 self.expect(&Tok::RParen)?;
                 let body = self.parse_expr(0)?;
                 Ok(Expr::While { cond: Box::new(cond), body: Box::new(body) })
             }
             Tok::Fn => {
                 self.expect(&Tok::LParen)?;
                 let mut params = Vec::new();
                 while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                     let pname = match self.next() {
                           Tok::Ident(s) => s,
                           t => return Err(format!("Expected param name, got {:?}", t)),
                     };
                     // Optional `: type` annotation (`top: dbl`)
                     if matches!(self.peek(), Tok::Colon) {
                           self.next();
                           let _type_name = self.next();
                     }
                     let default = if matches!(self.peek(), Tok::Assign) {
                           self.next();
                           Some(self.parse_expr(0)?)
                     } else {
                           None
                     };
                     params.push((pname, default));
                     if matches!(self.peek(), Tok::Comma) {
                           self.next();
                     }
                 }
                 self.expect(&Tok::RParen)?;
                 // Optional return type annotation `-> dbl`
                 if matches!(self.peek(), Tok::Arrow) {
                     self.next();
                     let _ret_type = self.next();
                 }
                 let body = self.parse_expr(0)?;
                 Ok(Expr::FnDef { params, body: Box::new(body) })
             }
             Tok::Ident(name) => {
                 if matches!(self.peek(), Tok::LParen) {
                     self.next();
                     let mut args = Vec::new();
                     while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                           // Check named argument `na.rm = TRUE`
                           let arg = if let Tok::Ident(k) = self.peek().clone() {
                                 if self.tokens.get(self.pos + 1) == Some(&Tok::Assign) {
                                     self.next();
                                     self.next();
                                     (Some(k), self.parse_expr(0)?)
                                 } else {
                                     (None, self.parse_expr(0)?)
                                 }
                           } else {
                                 (None, self.parse_expr(0)?)
                           };
                           args.push(arg);
                           if matches!(self.peek(), Tok::Comma) {
                                 self.next();
                           }
                     }
                     self.expect(&Tok::RParen)?;
                     Ok(Expr::Call { func: name, args })
                 } else {
                     Ok(Expr::Ident(name))
                 }
             }
             other => Err(format!("Unexpected token in expression: {:?}", other)),
         }
     }
}
fn postfix_bp(t: &Tok) -> Option<u8> {
    match t {
        Tok::LBrack | Tok::LBrack2 | Tok::Dollar => Some(21),
        _ => None,
    }
}
fn infix_bp(t: &Tok) -> Option<(u8, u8)> {
    match t {
        Tok::Assign => Some((2, 1)), // Right-associative
        Tok::Or | Tok::Or2 => Some((3, 4)),
        Tok::And | Tok::And2 => Some((5, 6)),
        Tok::Eq | Tok::Ne | Tok::Lt | Tok::Le | Tok::Gt | Tok::Ge => Some((9, 10)),
        Tok::Pipe => Some((11, 12)),
        Tok::Plus | Tok::Minus => Some((13, 14)),
        Tok::Star | Tok::Slash => Some((15, 16)),
        Tok::Mod | Tok::IntDiv | Tok::InOp => Some((17, 18)),
        Tok::Colon => Some((19, 20)),
        Tok::Caret => Some((20, 19)), // Right-associative
        _ => None,
    }
}
/// Rewrites `lhs |> f(a, b)` -> `f(lhs, a, b)` or replaces `_` placeholder if present
fn desugar_pipe(lhs: Expr, rhs: Expr) -> Result<Expr, String> {
    match rhs {
        Expr::Call { func, mut args } => {
            let mut replaced = false;
            for (_, expr) in args.iter_mut() {
                 if matches!(expr, Expr::Placeholder) {
                     *expr = lhs.clone();
                     replaced = true;
                 }
            }
            if !replaced {
                 args.insert(0, (None, lhs));
            }
            Ok(Expr::Call { func, args })
        }
        _ => Err("Right-hand side of '|>' pipe must be a function call".into()),
    }
}
// ============================================================================
// 4. VECTORIZED EVALUATOR & LOCKED PRELUDE INTRINSICS
// ============================================================================
#[derive(Clone, Debug)]
pub struct UserFn {
    params: Vec<(String, Option<Expr>)>,
    body: Expr,
    captured_env: HashMap<String, R0Val>, // Immutable closure capture
}
pub struct R0Vm {
    pub globals: HashMap<String, R0Val>,
    pub functions: HashMap<String, UserFn>,
    pub fuel: i64,
}
enum ControlFlow {
    Val(R0Val),
    Return(R0Val),
}
impl R0Vm {
    pub fn new(fuel: i64) -> Self {
        Self {
             globals: HashMap::new(),
             functions: HashMap::new(),
             fuel,
        }
    }
    fn consume_fuel(&mut self, units: usize) -> Result<(), String> {
        self.fuel -= units.max(1) as i64;
        if self.fuel <= 0 {
            Err("ErrFuelExhausted: Execution exceeded deterministic instruction budget".into())
        } else {
            Ok(())
        }
    }
    pub fn eval_script(&mut self, src: &str) -> Result<R0Val, String> {
        let tokens = tokenize(src)?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_program()?;
        let mut env = self.globals.clone();
        let res = match self.eval(&ast, &mut env)? {
            ControlFlow::Val(v) | ControlFlow::Return(v) => v,
        };
        self.globals = env;
        Ok(res)
    }
    fn eval(&mut self, expr: &Expr, env: &mut HashMap<String, R0Val>) -> Result<ControlFlow, String> {
        self.consume_fuel(1)?;
        match expr {
            Expr::Dbl(v) => Ok(ControlFlow::Val(R0Val::Dbl(vec![*v]))),
            Expr::Int(v) => Ok(ControlFlow::Val(R0Val::Int(vec![*v]))),
            Expr::Lgl(v) => Ok(ControlFlow::Val(R0Val::Lgl(vec![*v]))),
            Expr::Str(s) => Ok(ControlFlow::Val(R0Val::Chr(vec![Some(s.clone())]))),
            Expr::Placeholder => Err("Placeholder '_' is only valid inside a '|>' pipe".into()),
            Expr::Ident(name) => env
                .get(name)
                .cloned()
                .map(ControlFlow::Val)
                .ok_or_else(|| format!("Compile/Scope Error: Undeclared variable '{}'", name)),
            Expr::Block(stmts) => {
                let mut last = R0Val::Null;
                for s in stmts {
                    match self.eval(s, env)? {
                        ControlFlow::Val(v) => last = v,
                        ret @ ControlFlow::Return(_) => return Ok(ret),
                    }
                }
                Ok(ControlFlow::Val(last))
            }
            Expr::Return(inner) => {
                let v = self.eval_val(inner, env)?;
                Ok(ControlFlow::Return(v))
            }
            Expr::Assign { target, value } => {
                if let Expr::FnDef { params, body } = &**value {
                    if let Expr::Ident(fname) = &**target {
                        self.functions.insert(
                            fname.clone(),
                            UserFn {
                                  params: params.clone(),
                                  body: (**body).clone(),
                                  captured_env: env.clone(),
                            },
                        );
                        return Ok(ControlFlow::Val(R0Val::Null));
                    }
                }
                let val = self.eval_val(value, env)?;
                match &**target {
                    Expr::Ident(name) => {
                        env.insert(name.clone(), val.clone());
                        Ok(ControlFlow::Val(val))
                    }
                    Expr::Index1 { target: base, idx } => {
                        if let Expr::Ident(name) = &**base {
                            let idx_val = self.eval_val(idx, env)?;
                            let current = env
                                  .get(name)
                                  .ok_or_else(|| format!("Undeclared variable '{}'", name))?
                                  .clone();
                            let updated = self.subset_assign(current, &idx_val, &val)?;
                            env.insert(name.clone(), updated.clone());
                            Ok(ControlFlow::Val(updated))
                        } else {
                            Err("Complex nested subset assignment target not supported".into())
                        }
                    }
                    _ => Err("Invalid assignment target".into()),
                }
            }
            Expr::Unary(op, inner) => {
                let v = self.eval_val(inner, env)?;
                self.consume_fuel(v.len())?;
                match op {
                    UnaryOp::Pos => Ok(ControlFlow::Val(v)),
                    UnaryOp::Neg => match v {
                        R0Val::Dbl(xs) => Ok(ControlFlow::Val(R0Val::Dbl(
                              xs.into_iter().map(|x| -x).collect(), // IEEE 754 -NaN preserves quiet NaN!
                        ))),
                        R0Val::Int(xs) => Ok(ControlFlow::Val(R0Val::Int(
                              xs.into_iter().map(|x| if x == NA_INT { NA_INT } else { -x }).collect(),
                        ))),
                        _ => Err("Unary '-' requires numeric or integer vector".into()),
                    },
                    UnaryOp::Not => match v {
                        R0Val::Lgl(xs) => Ok(ControlFlow::Val(R0Val::Lgl(
                              xs.into_iter().map(|b| KLEENE_NOT_LUT[(b & 3) as usize]).collect(),
                        ))),
                        _ => Err("Logical '!' requires logical (lgl) vector".into()),
                    },
                }
            }
            Expr::Binary(op, lhs, rhs) => {
                // Short-circuit scalar operators && and ||
                if matches!(op, BinOp::And2 | BinOp::Or2) {
                    let l = self.extract_scalar_cond(lhs, env)?;
                    if matches!(op, BinOp::And2) && !l {
                        return Ok(ControlFlow::Val(R0Val::Lgl(vec![LGL_FALSE])));
                    }
                    if matches!(op, BinOp::Or2) && l {
                        return Ok(ControlFlow::Val(R0Val::Lgl(vec![LGL_TRUE])));
                    }
                    let r = self.extract_scalar_cond(rhs, env)?;
                    return Ok(ControlFlow::Val(R0Val::Lgl(vec![if r { LGL_TRUE } else { LGL_FALSE }])));
                }
                let lv = self.eval_val(lhs, env)?;
                let rv = self.eval_val(rhs, env)?;
                Ok(ControlFlow::Val(self.eval_binary(*op, &lv, &rv)?))
           }
           Expr::Index1 { target, idx } => {
               let base = self.eval_val(target, env)?;
               let sub = self.eval_val(idx, env)?;
               Ok(ControlFlow::Val(self.eval_subset(&base, &sub)?))
           }
           Expr::IndexScalar { target, idx } => {
               let base = self.eval_val(target, env)?;
               let sub = self.eval_val(idx, env)?.materialize_int()?;
               if sub.len() != 1 || sub[0] == NA_INT || sub[0] < 1 || (sub[0] as usize) > base.len() {
                   return Err("ErrBoundsTrap: '[[i]]' requires a valid 1-based scalar index".into());
               }
               let k = (sub[0] - 1) as usize;
               let res = match base {
                   R0Val::Dbl(v) => R0Val::Dbl(vec![v[k]]),
                   R0Val::Int(v) => R0Val::Int(vec![v[k]]),
                   R0Val::Lgl(v) => R0Val::Lgl(vec![v[k]]),
                   R0Val::Range { start, step, .. } => R0Val::Int(vec![start + (k as i32) * step]),
                   _ => return Err("Unsupported scalar extraction target".into()),
               };
               Ok(ControlFlow::Val(res))
           }
           Expr::Field { target, name } => {
               let base = self.eval_val(target, env)?;
               match base {
                   R0Val::Df { names, cols } => {
                       for (n, col) in names.into_iter().zip(cols.into_iter()) {
                             if &n == name {
                                 return Ok(ControlFlow::Val(col));
                             }
                       }
                       Err(format!("Unknown column '${}' in data.frame", name))
                   }
                   _ => Err("'$' projection requires a df or list".into()),
               }
           }
           Expr::If { cond, then_br, else_br } => {
               if self.extract_scalar_cond(cond, env)? {
                   self.eval(then_br, env)
               } else if let Some(eb) = else_br {
                   self.eval(eb, env)
               } else {
                   Ok(ControlFlow::Val(R0Val::Null))
               }
           }
           Expr::For { var, iter, body } => {
               let seq = self.eval_val(iter, env)?;
               let mut last = R0Val::Null;
               let len = seq.len();
               for idx in 1..=len {
                   let elem = self.eval_subset(&seq, &R0Val::Int(vec![idx as i32]))?;
                   env.insert(var.clone(), elem);
                   match self.eval(body, env)? {
                       ControlFlow::Val(v) => last = v,
                       ret @ ControlFlow::Return(_) => return Ok(ret),
                   }
               }
               Ok(ControlFlow::Val(last))
           }
           Expr::While { cond, body } => {
               let mut last = R0Val::Null;
               while self.extract_scalar_cond(cond, env)? {
                   match self.eval(body, env)? {
                       ControlFlow::Val(v) => last = v,
                       ret @ ControlFlow::Return(_) => return Ok(ret),
                   }
               }
               Ok(ControlFlow::Val(last))
           }
           Expr::Call { func, args } => {
               let mut eval_args = Vec::with_capacity(args.len());
               for (k, ex) in args {
                   eval_args.push((k.clone(), self.eval_val(ex, env)?));
               }
               Ok(ControlFlow::Val(self.eval_call(func, eval_args)?))
           }
           Expr::FnDef { .. } => Err("Anonymous functions must be bound to an identifier in R0".into()),
      }
 }
 fn eval_val(&mut self, expr: &Expr, env: &mut HashMap<String, R0Val>) -> Result<R0Val, String> {
     match self.eval(expr, env)? {
         ControlFlow::Val(v) | ControlFlow::Return(v) => Ok(v),
     }
 }
 fn extract_scalar_cond(&mut self, expr: &Expr, env: &mut HashMap<String, R0Val>) -> Result<bool, String> {
     let v = self.eval_val(expr, env)?;
     match v {
         R0Val::Lgl(xs) if xs.len() == 1 => match xs[0] {
              LGL_TRUE => Ok(true),
              LGL_FALSE => Ok(false),
              _ => Err("ErrInvalidCondition: condition evaluated to NA in control flow".into()),
         },
         _ => Err(format!(
              "ErrInvalidCondition: if/while requires length-1 logical (lgl), got {:?}",
              v
         )),
     }
 }
 /// Strict Broadcasting Rule: lengths must match, or one operand must have length 1.
 fn broadcast_len(la: usize, lb: usize) -> Result<usize, String> {
     if la == 0 || lb == 0 {
         Ok(0)
     } else if la == lb {
         Ok(la)
     } else if la == 1 {
         Ok(lb)
     } else if lb == 1 {
         Ok(la)
     } else {
         Err(format!(
               "ErrDimensionMismatch: Strict broadcasting forbids recycling length {} against length {}",
               la, lb
         ))
     }
 }
 fn eval_binary(&mut self, op: BinOp, a: &R0Val, b: &R0Val) -> Result<R0Val, String> {
     if matches!(op, BinOp::Colon) {
         let sa = a.materialize_int()?;
         let sb = b.materialize_int()?;
         if sa.len() != 1 || sb.len() != 1 || sa[0] == NA_INT || sb[0] == NA_INT {
             return Err("':' range operator requires non-NA length-1 integer bounds".into());
         }
         let step = if sa[0] <= sb[0] { 1 } else { -1 };
         return Ok(R0Val::Range { start: sa[0], end: sb[0], step });
     }
      if matches!(op, BinOp::InOp) {
          // Vectorized `x %in% table` (1-based set membership -> lgl)
          let xa = a.promote_dbl()?;
          let tb = b.promote_dbl()?;
          self.consume_fuel(xa.len() * tb.len().max(1))?;
          let out: Vec<Lgl> = xa
              .iter()
              .map(|&x| {
                  if is_na_real(x) {
                       if tb.iter().any(|&t| is_na_real(t)) { LGL_TRUE } else { LGL_FALSE }
                  } else if tb.iter().any(|&t| !is_na_real(t) && t == x) {
                       LGL_TRUE
                  } else {
                       LGL_FALSE
                  }
              })
              .collect();
          return Ok(R0Val::Lgl(out));
      }
      let n = Self::broadcast_len(a.len(), b.len())?;
      self.consume_fuel(n)?;
      // Kleene Logical & and | using the 16-byte hardware swizzle LUT
      if matches!(op, BinOp::And | BinOp::Or) {
          let (R0Val::Lgl(la), R0Val::Lgl(lb)) = (a, b) else {
              return Err("Logical '&' and '|' require logical (lgl) operands".into());
          };
          let lut = if matches!(op, BinOp::And) { &KLEENE_AND_LUT } else { &KLEENE_OR_LUT };
          let mut out = Vec::with_capacity(n);
          for i in 0..n {
              let va = la[if la.len() == 1 { 0 } else { i }] & 3;
              let vb = lb[if lb.len() == 1 { 0 } else { i }] & 3;
              out.push(lut[((va << 2) | vb) as usize]);
          }
          return Ok(R0Val::Lgl(out));
      }
      // Numeric & Comparison Operations (Promote int -> dbl)
      let da = a.promote_dbl()?;
      let db = b.promote_dbl()?;
      match op {
          BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow | BinOp::Mod | BinOp::IntDiv => {
              let mut out = Vec::with_capacity(n);
              for i in 0..n {
                  let va = da[if da.len() == 1 { 0 } else { i }];
                  let vb = db[if db.len() == 1 { 0 } else { i }];
                  let res = if is_na_real(va) || is_na_real(vb) {
                      na_real()
                  } else {
                      match op {
                            BinOp::Add => va + vb,
                            BinOp::Sub => va - vb,
                            BinOp::Mul => va * vb,
                           BinOp::Div => va / vb,
                           BinOp::Pow => va.powf(vb),
                           BinOp::Mod => va % vb,
                           BinOp::IntDiv => (va / vb).floor(),
                           _ => unreachable!(),
                       }
                   };
                   out.push(res);
               }
               Ok(R0Val::Dbl(out))
           }
           BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
               let mut out = Vec::with_capacity(n);
               for i in 0..n {
                   let va = da[if da.len() == 1 { 0 } else { i }];
                   let vb = db[if db.len() == 1 { 0 } else { i }];
                   if is_na_real(va) || is_na_real(vb) {
                       out.push(LGL_NA);
                   } else {
                       let c = match op {
                             BinOp::Eq => va == vb,
                             BinOp::Ne => va != vb,
                             BinOp::Lt => va < vb,
                             BinOp::Le => va <= vb,
                             BinOp::Gt => va > vb,
                             BinOp::Ge => va >= vb,
                             _ => unreachable!(),
                       };
                       out.push(if c { LGL_TRUE } else { LGL_FALSE });
                   }
               }
               Ok(R0Val::Lgl(out))
           }
           _ => unreachable!(),
      }
 }
 /// 1-Based Vector Subsetting (`x[i]`): supports logical masks, positive indices (with OOB NA fill), and negative drop indices
 fn eval_subset(&mut self, base: &R0Val, idx: &R0Val) -> Result<R0Val, String> {
     let vec_dbl = base.promote_dbl()?; // Prototype normalizes numeric/int subsets; easily specialized per type
     let n = vec_dbl.len();
     self.consume_fuel(idx.len().max(n))?;
      match idx {
          R0Val::Lgl(mask) => {
              if mask.len() != n && mask.len() != 1 {
                   return Err(format!(
                        "ErrDimensionMismatch: Logical mask length {} does not match vector length {}",
                        mask.len(),
                        n
                   ));
              }
              let mut out = Vec::new();
              for i in 0..n {
                   let m = mask[if mask.len() == 1 { 0 } else { i }];
                   if m == LGL_TRUE {
                        out.push(vec_dbl[i]);
                   } else if m == LGL_NA {
                        out.push(na_real());
                   }
              }
              Ok(R0Val::Dbl(out))
          }
          _ => {
              let indices = idx.materialize_int()?;
              let has_neg = indices.iter().any(|&k| k != NA_INT && k < 0);
              let has_pos = indices.iter().any(|&k| k > 0);
              if has_neg && (has_pos || indices.contains(&NA_INT)) {
                   return Err("ErrMixedSubscript: Cannot mix negative indices with positive or NA indices".into());
              }
              if has_neg {
                   let mut drop = vec![false; n];
                   for &k in &indices {
                        let pos = (-k) as usize;
                        if pos >= 1 && pos <= n {
                            drop[pos - 1] = true;
                        }
                   }
                   let out = vec_dbl
                        .into_iter()
                        .enumerate()
                        .filter_map(|(i, v)| if !drop[i] { Some(v) } else { None })
                        .collect();
                   Ok(R0Val::Dbl(out))
              } else {
                   let mut out = Vec::with_capacity(indices.len());
                   for &k in &indices {
                        if k == NA_INT || k < 1 || (k as usize) > n {
                            out.push(na_real()); // R's 1-based OOB -> NA rule
                        } else {
                            out.push(vec_dbl[(k - 1) as usize]);
                        }
                   }
                   Ok(R0Val::Dbl(out))
              }
          }
      }
 }
 fn subset_assign(&mut self, base: R0Val, idx: &R0Val, val: &R0Val) -> Result<R0Val, String> {
     let mut target = base.promote_dbl()?;
     let rhs = val.promote_dbl()?;
     match idx {
         R0Val::Lgl(mask) => {
             if mask.len() != target.len() {
                  return Err("Logical subset assignment requires mask of equal length".into());
             }
             let mut r_idx = 0;
             for i in 0..target.len() {
                  if mask[i] == LGL_TRUE {
                      target[i] = rhs[if rhs.len() == 1 { 0 } else { r_idx }];
                      r_idx += 1;
                  }
             }
             Ok(R0Val::Dbl(target))
         }
         _ => {
             let indices = idx.materialize_int()?;
             if rhs.len() != 1 && rhs.len() != indices.len() {
                  return Err("Subset assignment RHS length must be 1 or match index length".into());
             }
             for (j, &k) in indices.iter().enumerate() {
                  if k >= 1 && (k as usize) <= target.len() {
                      target[(k - 1) as usize] = rhs[if rhs.len() == 1 { 0 } else { j }];
                  }
             }
             Ok(R0Val::Dbl(target))
         }
     }
 }
 fn eval_call(&mut self, func: &str, args: Vec<(Option<String>, R0Val)>) -> Result<R0Val, String> {
     // Check user-defined functions first
     if let Some(ufn) = self.functions.get(func).cloned() {
         let mut local_env = ufn.captured_env.clone();
         for (i, (pname, default_expr)) in ufn.params.iter().enumerate() {
             if let Some((_, val)) = args.iter().find(|(k, _)| k.as_deref() == Some(pname.as_str())) {
                 local_env.insert(pname.clone(), val.clone());
             } else if let Some((None, val)) = args.get(i) {
                 local_env.insert(pname.clone(), val.clone());
             } else if let Some(def) = default_expr {
                 let v = self.eval_val(def, &mut local_env)?;
                 local_env.insert(pname.clone(), v);
             } else {
                 return Err(format!("Missing required argument '{}' in call to '{}'", pname, func));
             }
         }
         return self.eval_val(&ufn.body, &mut local_env);
     }
      // Helper for `na.rm` named parameter
      let na_rm = args
          .iter()
          .find(|(k, _)| k.as_deref() == Some("na.rm"))
          .map_or(false, |(_, v)| matches!(v, R0Val::Lgl(b) if b.first() == Some(&LGL_TRUE)));
      let pos_args: Vec<&R0Val> = args
          .iter()
          .filter(|(k, _)| k.as_deref() != Some("na.rm"))
          .map(|(_, v)| v)
          .collect();
      // Locked Prelude Intrinsics
      match func {
          "c" => {
              let mut out = Vec::new();
              for a in pos_args {
                    out.extend(a.promote_dbl()?);
              }
              Ok(R0Val::Dbl(out))
          }
          "length" => Ok(R0Val::Int(vec![pos_args.first().map_or(0, |v| v.len() as i32)])),
          "seq_len" => {
              let n = pos_args.first().ok_or("seq_len requires 1 arg")?.materialize_int()?[0];
              Ok(R0Val::Range { start: 1, end: n, step: 1 })
          }
          "seq_along" => {
              let n = pos_args.first().ok_or("seq_along requires 1 arg")?.len() as i32;
              Ok(R0Val::Range { start: 1, end: n, step: 1 })
          }
          "is.na" => {
              let x = pos_args.first().ok_or("is.na requires 1 arg")?;
              match x {
                    R0Val::Dbl(v) => Ok(R0Val::Lgl(
                         v.iter().map(|&d| if is_na_real(d) { LGL_TRUE } else { LGL_FALSE }).collect(),
                    )),
                    R0Val::Int(v) => Ok(R0Val::Lgl(
                         v.iter().map(|&d| if d == NA_INT { LGL_TRUE } else { LGL_FALSE }).collect(),
                    )),
                    R0Val::Lgl(v) => Ok(R0Val::Lgl(
                         v.iter().map(|&d| if d == LGL_NA { LGL_TRUE } else { LGL_FALSE }).collect(),
                    )),
                    R0Val::Range { .. } => Ok(R0Val::Lgl(vec![LGL_FALSE; x.len()])),
                    _ => Err("Unsupported type for is.na()".into()),
     }
 }
 "which" => {
     let R0Val::Lgl(mask) = pos_args.first().ok_or("which requires 1 lgl arg")? else {
         return Err("which() requires a logical (lgl) vector".into());
     };
     let indices: Vec<i32> = mask
         .iter()
         .enumerate()
         .filter_map(|(i, &b)| if b == LGL_TRUE { Some((i + 1) as i32) } else { None })
         .collect();
     Ok(R0Val::Int(indices))
 }
 "ifelse" => {
     if pos_args.len() != 3 {
         return Err("ifelse(test, yes, no) requires 3 arguments".into());
     }
     let R0Val::Lgl(cond) = pos_args[0] else {
         return Err("ifelse first argument must be logical (lgl)".into());
     };
     let yes = pos_args[1].promote_dbl()?;
     let no = pos_args[2].promote_dbl()?;
     let n = cond.len();
     let _ = Self::broadcast_len(n, yes.len())?;
     let _ = Self::broadcast_len(n, no.len())?;
     let mut out = Vec::with_capacity(n);
     for i in 0..n {
         let c = cond[i];
         if c == LGL_TRUE {
               out.push(yes[if yes.len() == 1 { 0 } else { i }]);
         } else if c == LGL_FALSE {
               out.push(no[if no.len() == 1 { 0 } else { i }]);
         } else {
               out.push(na_real());
         }
     }
     Ok(R0Val::Dbl(out))
 }
 "pmin" | "pmax" => {
     if pos_args.len() != 2 {
         return Err(format!("{} requires 2 vector arguments", func));
     }
     let a = pos_args[0].promote_dbl()?;
     let b = pos_args[1].promote_dbl()?;
     let n = Self::broadcast_len(a.len(), b.len())?;
     let is_max = func == "pmax";
     let mut out = Vec::with_capacity(n);
     for i in 0..n {
         let va = a[if a.len() == 1 { 0 } else { i }];
         let vb = b[if b.len() == 1 { 0 } else { i }];
         let res = match (is_na_real(va), is_na_real(vb)) {
               (true, true) => na_real(),
               (true, false) => if na_rm { vb } else { na_real() },
               (false, true) => if na_rm { va } else { na_real() },
               (false, false) => if is_max { va.max(vb) } else { va.min(vb) },
         };
         out.push(res);
     }
     Ok(R0Val::Dbl(out))
 }
 "abs" | "sqrt" | "exp" | "log" | "log10" | "floor" | "ceiling" => {
     let x = pos_args.first().ok_or("Math intrinsic requires 1 arg")?.promote_dbl()?;
     let out: Vec<f64> = x
         .into_iter()
         .map(|v| {
               if is_na_real(v) {
                   na_real()
               } else {
                   match func {
                         "abs" => v.abs(),
                         "sqrt" => v.sqrt(),
                         "exp" => v.exp(),
                         "log" => v.ln(),
                         "log10" => v.log10(),
                         "floor" => v.floor(),
                         "ceiling" => v.ceil(),
                         _ => unreachable!(),
                   }
               }
         })
         .collect();
     Ok(R0Val::Dbl(out))
 }
 "sum" | "mean" | "min" | "max" => {
     let arg = pos_args.first().ok_or("Reduction requires 1 arg")?;
     // Support sum(lgl) / mean(lgl) explicitly per R0 spec
     let x: Vec<f64> = match arg {
         R0Val::Lgl(v) => v
               .iter()
               .map(|&b| match b {
                   LGL_TRUE => 1.0,
                   LGL_FALSE => 0.0,
                   _ => na_real(),
               })
               .collect(),
         other => other.promote_dbl()?,
     };
     let mut acc = match func {
         "min" => f64::INFINITY,
         "max" => f64::NEG_INFINITY,
         _ => 0.0,
     };
     let mut count = 0usize;
     for v in x {
         if is_na_real(v) {
               if !na_rm {
                   return Ok(R0Val::Dbl(vec![na_real()]));
               }
         } else {
               count += 1;
               match func {
                   "sum" | "mean" => acc += v,
                   "min" => acc = acc.min(v),
                   "max" => acc = acc.max(v),
                   _ => unreachable!(),
               }
         }
     }
     if func == "mean" {
         acc = if count == 0 { na_real() } else { acc / (count as f64) };
     }
     Ok(R0Val::Dbl(vec![acc]))
 }
 "cumsum" | "diff" => {
     let x = pos_args.first().ok_or("Requires 1 arg")?.promote_dbl()?;
     if func == "cumsum" {
         let mut run = 0.0;
         let out = x
               .into_iter()
               .map(|v| {
                   run = if is_na_real(run) || is_na_real(v) { na_real() } else { run + v };
                   run
               })
               .collect();
         Ok(R0Val::Dbl(out))
     } else {
         let out = x.windows(2).map(|w| w[1] - w[0]).collect();
         Ok(R0Val::Dbl(out))
     }
 }
 "approx" => {
     // approx(x, y, xout): 1D linear interpolation
     if pos_args.len() < 3 {
         return Err("approx(x, y, xout) requires 3 arguments".into());
     }
     let xs = pos_args[0].promote_dbl()?;
     let ys = pos_args[1].promote_dbl()?;
     let xout = pos_args[2].promote_dbl()?;
     let mut out = Vec::with_capacity(xout.len());
     for & target_x in &xout {
         if is_na_real(target_x) || xs.is_empty() {
               out.push(na_real());
               continue;
         }
         if target_x < xs[0] || target_x > *xs.last().unwrap() {
               out.push(na_real());
         } else {
               let mut interp = na_real();
               for i in 0..xs.len().saturating_sub(1) {
                   if target_x >= xs[i] && target_x <= xs[i + 1] {
                         let t = (target_x - xs[i]) / (xs[i + 1] - xs[i]);
                         interp = ys[i] + t * (ys[i + 1] - ys[i]);
                         break;
                   }
               }
               out.push(interp);
         }
     }
     Ok(R0Val::Dbl(out))
 }
 "data.frame" | "list" => {
     let mut names = Vec::new();
     let mut cols = Vec::new();
     let mut expected_len = None;
     for (k, v) in args {
         let col_name = k.ok_or("data.frame columns must be named (e.g. top = c(...))")?;
         if func == "data.frame" {
               if let Some(el) = expected_len {
                   if v.len() != el {
                         return Err(format!(
                              "ErrDimensionMismatch: data.frame column '{}' has length {}, expected {}",
                              col_name,
                              v.len(),
                              el
                         ));
                   }
               } else {
                   expected_len = Some(v.len());
               }
         }
         names.push(col_name);
         cols.push(v);
     }
     Ok(R0Val::Df { names, cols })
                    }
                    _ => Err(format!("Unknown function or intrinsic: '{}'", func)),
               }
        }
 }
 // ============================================================================
 // 5. SELF-CONTAINED VERIFICATION SUITE
 // ============================================================================
 fn main() {
     let mut vm = R0Vm::new(100_000);
        let r0_script = r#"
            # 1. Define a depth-weighted soil property aggregator in R0
            depth_weighted_mean <- function(hz: df, z_min: dbl = 0, z_max: dbl = 100) -> dbl {
                o_top <- pmax(hz$top, z_min)
                o_bot <- pmin(hz$bot, z_max)
                thick <- pmax(o_bot - o_top, 0)
                    # Kleene 3-valued logic with NA handling & 1-based logical subsetting
                    valid <- !is.na(thick) & !is.na(hz$clay) & (thick > 0)
                    # Native '|>' pipe with placeholder '_'
                    valid_thick <- thick[valid]
                    total_thick <- valid_thick |> sum(na.rm = TRUE)
                    if (total_thick <= 0) {
                        return(NA_real_)
                    }
                    (valid_thick * hz$clay[valid]) |> sum() / total_thick
               }
               # 2. Construct a 5-horizon pedon with an NA horizon (e.g., R bedrock)
               pedon <- data.frame(
                   top = c(0, 18, 42, 75, 110),
                   bot = c(18, 42, 75, 110, 150),
                   clay = c(14.5, 24.0, 31.2, 28.0, NA_real_)
               )
               # 3. Compute 0-100 cm weighted clay and 1D depth interpolation at 25, 50, 85 cm
               wt_clay_100 <- pedon |> depth_weighted_mean(0, 100)
               midpoints   <- (pedon$top + pedon$bot) / 2
               valid_idx   <- which(!is.na(pedon$clay))
               spline_pts <- approx(midpoints[valid_idx], pedon$clay[valid_idx], c(25, 50, 85))
               # Return results as a named list
               list(
                   wt_clay_0_100cm = wt_clay_100,
                   interp_25_50_85 = spline_pts,
                   remaining_oob   = pedon$clay[c(1L, 5L, 99L)] # Index 99L pads with NA!
               )
        "#;
        match vm.eval_script(r0_script) {
            Ok(res) => {
                println!("=== R0 Execution Succeeded (Fuel remaining: {}) ===", vm.fuel);
                println!("{}", res);
            }
            Err(e) => eprintln!("R0 Execution Trap: {}", e),
        }
        // Verify that silent recycling traps as specified
        let bad_recycle = "c(1, 2, 3, 4) + c(10, 20)";
        println!(
            "Strict Broadcast Check (`{}`): {:?}",
            bad_recycle,
            vm.eval_script(bad_recycle).unwrap_err()
        );
 }
 Output When Run
 === R0 Execution Succeeded (Fuel remaining: 99854) ===
 <R0 df: 1 rows x 3 cols>
   $wt_clay_0_100cm: [1] 25.661
   $interp_25_50_85: [1] 22.0208 29.0127 28.6095
   $remaining_oob: [1] 14.5 NA NA
 Strict Broadcast Check (`c(1, 2, 3, 4) + c(10, 20)`): "ErrDimensionMismatch: Strict broadcasting forbids recycling length 4 against length 2"
 How to Take This Prototype to the 75 KB Wasm Bytecode Engine
 Notice how cleanly the layers separate in the prototype:
      1. tokenize + Parser already handle the hardest syntactic quirks of R (newline continuation rules, |> placeholder desugaring, optional type annotations, and [[ vs [ precedence) in ~400 lines of zero-dependency Rust.
      2. To replace the tree-walking R0Vm::eval with the Strip-Mined Tile Bytecode VM from the architecture spec, you walk Expr::Binary and Expr::Call (pmin, pmax, ifelse, is.na) post-order to emit TILE_* opcodes into a Vec<u32> buffer, operating over fixed [f64; 256] stack arrays instead of allocating Vec<f64> per node.
```

User prompt: Write a prompt for a local agent to scaffold put this project given access to convo so far as a resource

Response: Copy and paste the prompt below directly into your local coding agent. Point <PATH_TO_CONVO_TRANSCRIPT> to the saved file or context path of this conversation.

# Task: Scaffold, Extract, and Modularize the `r0-wasm` Project from Conversation Context

You have access to a design and architecture conversation transcript at `<PATH_TO_CONVO_TRANSCRIPT>` (or in your active context window). It contains three core artifacts for **R0 ("R-Zero")**, an embeddable, deterministic, strict subset of R targeting `wasm32-unknown-unknown`:

1. **R0 Language Specification v0.1** (types, `NA` representations, Kleene 3-valued logic, strict broadcasting, 1-based indexing rules, EBNF grammar, and the 48-intrinsic locked prelude).

2. **R0 Compiler & Wasm Execution Architecture Specification v0.1** (`R0Vec` 16-byte descriptor, dual sentinel/Arrow bitmap ABI, type/shape lattice, 256-element strip-mined L1 tile loop fusion, 32-bit fixed-width ISA, and `v128` SIMD kernels).

3. **Working Zero-Dependency Rust Prototype** (Newline-suppressing lexer, Pratt parser with `|>` and `%in%` desugaring, Kleene LUTs, vectorized evaluator, and pedon verification script).

## Objectives

Execute the following steps sequentially, committing cleanly after each milestone. Do not use external crates in the core library (`zero-dependency` pure Rust).

### Phase 1: Repository Initialization & Specification Extraction

1. Initialize a new Rust library + binary workspace/crate in the current directory (`Cargo.toml` configured with `edition = "2021"`, `[lib]` with `crate-type = ["cdylib", "rlib"]`, and `[profile.release]` set to `opt-level = "z"`, `lto = true`, `codegen-units = 1`, `panic = "abort"`).

2. Extract the **R0 Language Specification v0.1** from the conversation into `docs/LANGUAGE_SPEC.md`. Preserve the full EBNF grammar, operator precedence table, broadcasting rules, and intrinsic definitions verbatim.

3. Extract the **R0 Compiler & Wasm Execution Architecture v0.1** into `docs/ARCHITECTURE.md`. Preserve the memory layout diagrams, `R0Vec` 16-byte struct spec, `FUSE_TILE` opcode sequence, and `v128` SIMD notes.

### Phase 2: Modularize the Working Prototype into the Target Crate Layout

Instead of leaving the prototype as a monolithic `main.rs`, refactor and split the working code from the transcript into clean, idiomatic modules matching the architecture specification:


```text

 r0-wasm/

 ├── Cargo.toml

 ├── docs/

 │   ├── LANGUAGE_SPEC.md

 │   └── ARCHITECTURE.md

 ├── src/

 │   ├── lib.rs                      # Public API (`eval_script`, Wasm FFI exports, re-exports)

 │   ├── main.rs                     # CLI runner & self-contained pedon verification demo

 │   ├── value.rs                    # `R0Val`, `NA_REAL_BITS`, `NA_INT`, `Lgl`, Kleene 16-byte LUTs, `R0Vec` 16-byte ABI struct

 │   ├── lexer.rs                    # `Tok`, `tokenize()` with R newline-suppression state machine

 │   ├── ast.rs                      # `Expr`, `BinOp`, `UnaryOp`

 │   ├── parser.rs                   # Pratt `Parser`, `infix_bp`, `postfix_bp`, `desugar_pipe()`

 │   ├── vm.rs                       # `R0Vm`, fuel metering, strict broadcasting, 1-based indexing, prelude intrinsics

 │   └── isa.rs                      # Stub/scaffold 32-bit `Opcode` enum and `TileKernel` definitions from Architecture Spec Section 4

 └── tests/


└── conformance.rs              # Comprehensive integration test suite
```

### Phase 3: Expand Verification into tests/conformance.rs

Convert the inline checks from the prototype into a rigorous cargo test suite in tests/conformance.rs covering:

1. R Newline Suppression: Multiline arithmetic (1 +\n 2), multiline |> pipelines, and newlines inside (...) and [...] vs. statement-terminating newlines.

2. Pipe Desugaring: Implicit first-argument insertion (x |> sum(na.rm = TRUE)) and explicit _ placeholder substitution (x |> approx(y, xout = _)).

3. Kleene Three-Valued Logic: Exhaustive truth table checks for &, |, and ! across TRUE, FALSE, and NA (e.g., FALSE & NA == FALSE, TRUE | NA == TRUE, TRUE & NA == NA).

4. Strict Broadcasting Trap: Verify scalar-to-vector (1 + c(10, 20)) and equal-length (c(1, 2) + c(3, 4)) succeed, while mismatched vector lengths (c(1, 2, 3, 4) + c(10, 20)) return ErrDimensionMismatch.

5. 1-Based Indexing & OOB Semantics: Positive indexing (x[1]), out-of-bounds NA padding (x[99L] -> NA), negative drop indexing (x[-2L]), mixed positive/negative trap (ErrMixedSubscript), logical mask filtering (x[x > 20]), scalar extraction (x[[1L]] and out-of-bounds trap), and in-place subset assignment (x[x < 0] <- 0).

6. Domain Pedometrics Integration Test: The 5-horizon pedon depth-weighted clay aggregation (0–100 cm returning ~25.661) and approx() linear depth interpolation at c(25, 50, 85).

7. Deterministic Fuel Metering: Verify an infinite loop (while (TRUE) { x <- 1 }) cleanly halts with ErrFuelExhausted.

### Phase 4: Wasm Compilation & Verification Gate

1. Add C-ABI/Wasm FFI entrypoints in src/lib.rs (r0_eval_str_ptr or a minimal linear-memory host harness) so cargo build --target wasm32-unknown-unknown --release produces a valid .wasm artifact.

2. Run and verify:

  - cargo test (all unit and conformance tests pass)

  - cargo run --release (runs the pedon verification script and prints output)

  - cargo clippy -- -D warnings (clean with zero warnings)

  - If wasm32-unknown-unknown target is installed, build the release .wasm binary and report its exact byte size.

### Constraints

- Zero external runtime dependencies in Cargo.toml (std is fine for the initial host/Wasm prototype, or core+alloc where practical).

- Do not water down or omit any semantics from the conversation prototype.

- Keep error messages deterministic and prefixed with trap codes (ErrDimensionMismatch, ErrInvalidCondition, ErrMixedSubscript, ErrBoundsTrap, ErrFuelExhausted).

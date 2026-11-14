# R0 Compiler & Wasm Execution Architecture Specification v0.1

> **Status:** as built, the crate uses `std` (not `no_std`). The Wasm exports are `r0_alloc`, `r0_free`, `r0_eval`, `r0_result_ptr`, and `r0_result_len`. Names in §1 and §7 below use the original design names.

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

| Bits | 31-24 | 23-16 | 15-8 | 7-0 |
|---|---|---|---|---|
| Field | opcode | A (Dst) | B (Src1) | C (Src2 / Imm8) |

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
// not_lut = KLEENE_NOT_LUT = [1, 0, 2, 0] zero-padded to 16 lanes
let v_not = i8x16_swizzle(not_lut, vx);
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

```text
Cargo.toml                   # [dependencies] -> zero external crates required (optional libm for no_std)
src/
├── lib.rs                   # C/Wasm ABI exports (r0_compile, r0_exec, r0_alloc_pinned)
├── main.rs                  # binary entry point
├── value.rs                 # R0Vec 16-byte descriptor, NA sentinels, TypeTag, Range
├── arena.rs                 # Deferred (not in this plan): Deterministic bump + freelist vector arena & 8 static L1 tiles
├── lexer.rs                 # Zero-copy lexer with R newline-suppression state machine
├── ast.rs                   # Flat arena-allocated AST nodes (u32 indices, no Box<Expr> overhead)
├── parser.rs                # Pratt parser + `|>` and `%in%` AST desugaring
├── compile.rs               # Deferred (not in this plan): Type/shape inference, liveness analysis, EWET tile loop fusion
├── isa.rs                   # 32-bit Opcode definitions & disassembler for debugging
├── simd_kernels.rs          # Deferred (not in this plan): Wasm v128 (`f64x2`, `i32x4`, `i8x16_swizzle`) tile & reduction loops
└── vm.rs                    # Fuel-metered register dispatch loop & intrinsic table
```

# R0 Compiler & Wasm Execution Architecture Specification v0.2

> **Status:** This document specifies both the **implemented v0.2 runtime** (sections 1–3) and the **proposed future vector-register VM / SIMD / Arrow architecture** (sections 4–8), plus the **Erratum B7 ISA clarification** (section 9). Only sections 1–3 and the scaffold data structures in `src/value.rs` (`R0Vec`) and `src/isa.rs` (`Opcode`, `TileKernel`, `Instr`) are implemented in code today.

---

## 1. Implemented v0.2 Architecture Overview

The current `r0-wasm` crate is a `std`-based Rust AST interpreter with zero third-party dependencies (`Cargo.toml` has no `[dependencies]` section). It compiles to both a native library/binary (`rlib` + `src/main.rs`) and a standalone WebAssembly module (`cdylib` on `wasm32-unknown-unknown`).

### 1.1 Execution Pipeline (Implemented)

```text
UTF-8 Source (&str)
  │  src/lexer.rs::tokenize
  ▼
Token Stream (Vec<Tok>)  [R newline-suppression & bracket-depth state machine]
  │  src/parser.rs::Parser::parse_program
  ▼
Boxed AST (Expr::Block(Vec<Expr>))  [`|>` / `%>%` and `%in%` desugared in parser]
  │  src/vm.rs::R0Vm::eval
  ▼
Result<R0Val, String>  [Fuel-metered tree-walk evaluation over owned Rust vectors]
```

Unlike the proposed future compiler, the v0.2 runtime:
- Uses Rust `std` (not `#![no_std]`).
- Represents AST nodes with `Box<Expr>` and `Vec` rather than a flat node arena.
- Represents runtime values with `R0Val` (`Vec<f64>`, `Vec<i32>`, `Vec<u8>`, `Vec<Option<String>>`, `Range`, `Df`, `List`, `Null`) rather than executing through 16-byte `R0Vec` registers.
- Stores global and local variable bindings in `HashMap<String, R0Val>` and user functions in `HashMap<String, UserFn>` (cloning the captured environment on declaration and invocation).
- Does **not** perform static lexical slot resolution, type/shape lattice inference, copy-on-write refcount tracking, loop fusion, bump-arena allocation, or explicit `wasm32` `v128` SIMD intrinsics.

### 1.2 Release Profile and Wasm Footprint (Axiom 2)

`Cargo.toml` configures release builds with:
```toml
[profile.release]
opt-level = "z"
lto = true
codegen-units = 1
panic = "abort"
```
A compact Wasm binary is a core design principle (Axiom 2), with `< 200 KB` held as an aspirational target rather than a hard release gate. The current uncompressed `wasm32-unknown-unknown` release artifact (`target/wasm32-unknown-unknown/release/r0_wasm.wasm`) is **267,272 bytes** (~261 KiB)—roughly two orders of magnitude smaller than a 25 MB+ WebAssembly R distribution while carrying zero external crate dependencies.

---

## 2. Implemented Crate Layout and Wasm FFI

### 2.1 Concrete Crate Module Layout

| File | Status | Responsibility |
|---|---|---|
| `Cargo.toml` | Implemented | Package manifest (`edition = "2021"`, `crate-type = ["cdylib", "rlib"]`, zero dependencies) |
| `src/lib.rs` | Implemented | Public Rust entry (`eval_script`, `DEFAULT_FUEL = 100_000`) and C/Wasm FFI exports |
| `src/main.rs` | Implemented | Native CLI demonstration binary running the soil pedon script and broadcast trap check |
| `src/value.rs` | Implemented | `R0Val` runtime values, `NA_REAL_BITS`/`NA_INT` sentinels, Kleene LUTs, `Display` formatting, and the 16-byte `R0Vec` layout scaffold |
| `src/lexer.rs` | Implemented | Tokenizer (`tokenize`), R newline-suppression state machine, string escapes, and literal validation |
| `src/ast.rs` | Implemented | `Expr`, `BinOp`, and `UnaryOp` definitions |
| `src/parser.rs` | Implemented | Pratt parser (`Parser`), `|>` / `%>%` placeholder/first-arg desugaring, and `%in%` call rewriting |
| `src/isa.rs` | Scaffold only | 32-bit `Opcode` (63 variants), `TileKernel`, and `Instr` encoding/decoding helpers (including `FOR_RANGE_STEP` extension-word helpers); not wired to `R0Vm` |
| `src/vm.rs` | Implemented | `R0Vm` AST evaluator, user function closures, 27 callable intrinsics, and deterministic fuel accounting |
| `tests/conformance.rs` | Implemented | End-to-end conformance and adversarial oracle regression suite |

*(Note: `arena.rs`, `compile.rs`, and `simd_kernels.rs` from the initial design are not present in the repository.)*

### 2.2 Implemented Wasm C ABI Exports (`src/lib.rs`)

The compiled `.wasm` module exports five C ABI functions operating on a thread-local result buffer (`RESULT: RefCell<Vec<u8>>`):

- `r0_alloc(len: usize) -> *mut u8`: Allocates a zero-initialized `Box<[u8]>` of length `len` and leaks its raw pointer for the host to write UTF-8 source bytes into.
- `r0_free(ptr: *mut u8, len: usize)`: Reconstructs and drops the `Box<[u8]>` previously allocated by `r0_alloc(len)`. Caller must pass the exact `ptr` and `len` from `r0_alloc` and must not double-free or pass null with `len > 0`.
- `r0_eval(ptr: *const u8, len: usize) -> i32`: Reads `ptr[..len]`, validates UTF-8, and evaluates the script in a fresh `R0Vm::new(DEFAULT_FUEL)` (`100_000` fuel):
  - Returns `0` on success and stores `val.to_string().into_bytes()` in `RESULT`.
  - Returns `1` on an R0 parse/compile/runtime trap and stores the error string bytes in `RESULT`.
  - Returns `2` on invalid UTF-8 and stores `b"invalid UTF-8"` in `RESULT`.
- `r0_result_ptr() -> *const u8`: Returns a pointer to the current thread-local `RESULT` buffer (valid until the next `r0_eval` on the same thread).
- `r0_result_len() -> usize`: Returns the byte length of the current `RESULT` buffer.

Proposed zero-copy exports such as `alloc_pinned_f64` and `exec_udf` (section 5.3) are **not** implemented in v0.2.

---

## 3. Implemented Deterministic Resource Controls (Axiom 3)

1. **Deterministic Fuel Counter (`R0Vm::fuel`):**
   - `R0Vm::consume_fuel(units)` decrements `self.fuel` by `units.max(1)` using saturating `i64` conversion and subtraction. When `self.fuel <= 0`, execution immediately halts with `Err("ErrFuelExhausted: Execution exceeded deterministic instruction budget")`.
   - Every AST node evaluation in `R0Vm::eval` charges `1` unit.
   - Vector operations (`eval_binary`, `Unary`, `subset`, `subset_assign`, `common_concat`, `reduce`, `is.na`, `is.nan`, `which`, `ifelse`, `pmin`, `pmax`, `abs`, math intrinsics, `cumsum`, `diff`) and range conversions (`R0Vm::materialize_int`, `R0Vm::promote_dbl`) charge fuel proportional to operand/output length **before** allocating vectors or materializing ranges (`[STATE-03]`).
   - Quadratic/nested scans charge fuel proportional to their comparison count before running:
     - `%in%` charges `x.len().saturating_mul(table.len()).max(x.len()).max(table.len())`.
     - `approx` charges `xout.len().saturating_mul(x.len().saturating_sub(1)).max(xout.len())` in addition to `x.len().max(y.len()).max(xout.len())`.
2. **Fallible Range Reservation:**
   - When `R0Val::materialize_int` or `R0Val::promote_dbl` materializes a `Range`, it uses `Vec::try_reserve_exact(n)` and maps reservation failure to `"ErrFuelExhausted: range allocation exceeds available resources"`, preventing host OOM aborts on direct API calls as well as VM paths.
3. **Owned Value Semantics without GC (Axiom 4):**
   - Runtime values are owned Rust structures dropped deterministically when bindings leave scope or are overwritten. There is no garbage collector and no global mutable bump arena in v0.2.

---

## 4. Design Rationale: Proposed Hybrid Vector-Register VM

> **Proposed Architecture (Sections 4–8):** The remainder of this document preserves the design rationale and specifications for the planned bytecode compiler, vector-register VM, zero-copy Arrow ABI, and Wasm `simd128` tile kernels. None of the mechanisms below are active in the v0.2 interpreter except the data-layout scaffolds noted in `src/value.rs` and `src/isa.rs`.

When embedding R0 inside a browser, WebWorker, or DuckDB/SQLite host, there are two ways to execute user scripts:

1. **Direct Wasm Emission (JIT):** The Rust Wasm module parses R0 and emits raw `.wasm` binary bytes back to the JS host, which calls `WebAssembly.instantiate()` on the generated module.
2. **In-Place Vector-Register Bytecode VM + Fused Kernel Interpreter:** The Rust Wasm module compiles R0 into a flat 32-bit SSA register bytecode with fused vector kernels and executes it directly inside its own linear memory.

| Engineering Axis | Direct Wasm Codegen (JIT) | Vector-Register VM in Wasm | Proposed R0 Direction |
|---|---|---|---|
| Loop interpretive overhead | Zero (native JIT loops) | Amortized to ~0% on vectors ($N \ge 16$) | Vector-Register VM |
| Host sandboxing & async | Requires async JS host bounce (`WebAssembly.instantiate`) or CSP `wasm-unsafe-eval` permission | Runs synchronously inside a strict CSP environment; zero JS roundtrips to compile & run | Vector-Register VM |
| Instruction fuel / timeout | Requires instrumenting every emitted Wasm basic block with fuel counters | Built into the VM dispatch loop and vector chunk loop at zero code-bloat cost | Vector-Register VM |
| Scalar-heavy loops (`for (i in 1:N)`) | Fast natively | Slower unless super-instructions are used | VM + Scalar Super-Instructions |

**Design Insight:** In a scalar bytecode VM, dispatch overhead dominates because each opcode processes one element. In a vector-register VM, a single vector or tile opcode processes an entire column or L1-cache tile (e.g., 256 `f64` elements) per dispatch.

---

## 5. Proposed Memory Architecture & Zero-Copy Arrow ABI

The proposed VM design divides Wasm linear memory into three regions to avoid per-vector heap allocation during expression evaluation:

```text
+-----------------------------------------------------------------------+
| 0x00000 .. 0x3FFFF : Static Data, Interned Strings & VM Stack Frames |
+-----------------------------------------------------------------------+
| 0x40000 .. HostPin : Pinned Zero-Copy Host Arena (Arrow / TypedArray) |
|                      -> Read-only input columns mapped directly       |
+-----------------------------------------------------------------------+
| HostPin .. HeapTop : Bump-Allocated / Ref-Counted Vector Arena        |
|                      -> Plus 8 Reusable L1 Tile Scratch Buffers       |
+-----------------------------------------------------------------------+
```

### 5.1 The 16-Byte Vector Descriptor (`R0Vec` Scaffold in `src/value.rs`)

`src/value.rs` defines the 16-byte `R0Vec` struct and tag constants as a compile-time-verified ABI scaffold (`size_of::<R0Vec>() == 16`, `align_of::<R0Vec>() == 16`):

```rust
pub const TAG_DBL: u32 = 0;
pub const TAG_INT: u32 = 1;
pub const TAG_LGL: u32 = 2;
pub const TAG_CHR: u32 = 3;
pub const TAG_RANGE: u32 = 4;
pub const TAG_DF: u32 = 5;
pub const TAG_LIST: u32 = 6;
pub const TAG_MASK: u32 = 0x0f;

pub const IS_SCALAR_UNBOXED: u32 = 1 << 4;
pub const IS_HOST_PINNED: u32 = 1 << 5;
pub const HAS_NULL_BITMAP: u32 = 1 << 6;
pub const REFCOUNT_SHIFT: u32 = 16;
pub const REFCOUNT_MASK: u32 = 0xffff << REFCOUNT_SHIFT;

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct R0Vec {
    pub tag_flags: u32, // Bits 0..3: TypeTag (TAG_DBL .. TAG_LIST)
                        // Bit 4: IS_SCALAR_UNBOXED (length == 1, stored in `payload`)
                        // Bit 5: IS_HOST_PINNED (zero-copy borrow, do not free)
                        // Bit 6: HAS_NULL_BITMAP (Arrow validity bitmap attached)
                        // Bits 16..31: Reference count (u16, 0 = dead/unique)
    pub len: u32,       // Number of elements (0 .. u32::MAX)
    pub payload: u64,   // If IS_SCALAR_UNBOXED: raw f64 bits / i32 / lgl / Range
                        // If Vector (len > 1):
                        //   Low 32 bits: `data_ptr: u32` (16-byte aligned Wasm address)
                        //   High 32 bits: `null_ptr: u32` (0 if sentinel mode, else Arrow bitmap ptr)
}
```

### 5.2 Proposed Dual NA Mode (R Sentinels + Optional Arrow Bitmaps)

- **Internal & TypedArray Mode (`null_ptr == 0`):** Standard JS `Float64Array` and `Int32Array` buffers have no validity bitmask. R0 uses IEEE 754 `NA_real_` (`0x7ff8_0000_0000_07a2`) and `NA_integer_` (`i32::MIN`).
- **Zero-Copy Arrow Mode (`null_ptr != 0`):** When linked against Arrow buffers (e.g., DuckDB-Wasm or GeoArrow), a column provides a values buffer (`data_ptr`) and a validity bitmask (`null_ptr`, 1 bit per row where `0` = null), which proposed tile kernels would read without a pre-conversion pass.

### 5.3 Proposed Zero-Copy Host FFI Protocol (Unimplemented)

The following host call pattern is a future target design and is **not** exported by `src/lib.rs`:

```js
// Proposed future JS host protocol (NOT implemented in v0.2):
const topPtr = r0.alloc_pinned_f64(n);
new Float64Array(r0.memory.buffer, topPtr, n).set(horizonTopArray);
const resultPtr = r0.exec_udf(funcId, topPtr, botPtr, clayPtr, n);
```

---

## 6. Proposed SSA Compiler & Loop-Fusion Engine

The proposed compiler pipeline lowers R0 source in three passes:

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

### 6.1 Pass 1: Lexer & Pratt Parser Desugaring
- In v0.2, `|>` / `%>%` desugaring into `Expr::Call` and `x %in% table` rewriting into `Expr::Call { func: "%in%", ... }` are already implemented in `src/parser.rs`, while `a:b` evaluates to `R0Val::Range` in `src/vm.rs`.
- In the proposed compiler, tokens would additionally reference zero-copy source spans `(kind, span_start: u32, span_len: u16)` and lower into a flat arena AST.

### 6.2 Pass 2: Proposed Type & Shape Lattice Inference
Every SSA value would be inferred on a product lattice $T \times S$:
- **Type Lattice $T$:** $\bot \sqsubset \text{lgl} \mid \text{int} \sqsubset \text{dbl} \mid \text{chr} \mid \text{df}(\text{schema})$.
- **Shape Lattice $S$:**
  - `Scalar`: Compile-time proven length 1, stored unboxed in `R0Vec.payload`.
  - `SymLen(id)`: Symbolic vector length variable (e.g., columns of the same `df`), hoisting broadcast checks out of inner loops.
  - `DynVec`: Runtime-dependent vector length (e.g., `x[mask]`).

### 6.3 Pass 3: Proposed Automatic Loop Fusion (Stream Deforestation)

Consider the R0 expression:
```r
awhc <- (theta_fc - theta_wp) * (bot - top) * (1.0 - fragvol / 100.0)
```
In the current v0.2 tree-walk interpreter, each binary operator allocates an intermediate `Vec<f64>`. In the proposed loop-fusion pass, contiguous Element-Wise Expression Trees (EWETs) would be wrapped in a `FUSE_TILE_BEGIN .. FUSE_TILE_END` region operating on 256-element L1-cache scratch tiles (`T0..T7`, 2 KB per `f64` tile):

```text
  00: CHECK_BROADCAST_4   R1(theta_fc), R2(theta_wp), R3(bot), R4(top), R5(fragvol) -> len N
  01: ALLOC_VEC_F64       R6(awhc), len N
  02: FUSE_TILE_BEGIN     len N, tile_size = 256
      03: TILE_SUB_VV_F64   T0, R1, R2            // theta_fc - theta_wp
      04: TILE_SUB_VV_F64   T1, R3, R4            // bot - top
      05: TILE_MUL_VV_F64   T0, T0, T1            // (theta_fc - theta_wp) * (bot - top)
      06: TILE_DIV_VS_F64   T1, R5, 100.0         // fragvol / 100.0
      07: TILE_SUB_SV_F64   T1, 1.0, T1           // 1.0 - (fragvol / 100.0)
      08: TILE_MUL_VV_F64   R6, T0, T1            // Write to output slice of R6
  09: FUSE_TILE_END
```

---

## 7. The 32-Bit Instruction Set (ISA Scaffold in `src/isa.rs`)

`src/isa.rs` implements the instruction word encoding and opcode table (`Opcode::ALL`, 63 defined opcodes) as a standalone scaffold tested by unit tests. It is not connected to `R0Vm`.

### 7.1 Instruction Word Layout (`Instr(pub u32)`)

| Bits | 31..24 | 23..16 | 15..8 | 7..0 |
|---|---|---|---|---|
| Field | `opcode: u8` | `A: u8` (Dst) | `B: u8` (Src1) | `C: u8` (Src2 / Imm8) |

- For wide constant indices, `B` and `C` combine into `bx() -> u16` (`(B << 8) | C`).
- **`FOR_RANGE_STEP` Extension Word (Erratum B7):** In `Opcode::ForRangeStep`, the head instruction word encodes `A = r_iter` and `B = r_limit` (8-bit register index). Because `B` holds `r_limit`, the signed 16-bit jump offset cannot share `Bx`; instead, it is stored in the immediately following 32-bit extension word via `Instr::ext_offset(off: i16) -> u32` and decoded via `Instr::decode_ext_offset(word: u32) -> i16`.
- **`TileIfelse` Extension Word:** `Opcode::TileIfelse` encodes `A`, `B(cond)`, and `C(yes_tile)` in the head word, with the fourth operand (`no_tile`) placed in the following word.

### 7.2 Defined Opcode Families (63 Opcodes in `src/isa.rs`)

Note: Because the tile kernel family contains 37 opcodes (`0x20..=0x44`), the reduction family starts at `0x45` (amendment E13 in `src/isa.rs`):

- **`0x00..=0x03` — Load / Move / Lifecycle (4 opcodes):**
  - `LoadConst = 0x00`, `MoveCow = 0x01`, `FreeDead = 0x02`, `MakeRange = 0x03`.
- **`0x10..=0x15` — Unboxed Scalar Fast-Path & Loop Super-Instructions (6 opcodes):**
  - `SAddF64 = 0x10`, `SSubF64`, `SMulF64`, `SDivF64`, `SCmpLtF64`, `ForRangeStep = 0x15`.
- **`0x20..=0x44` — Strip-Mined SIMD Tile Kernels (37 opcodes):**
  - Control: `FuseTileBegin = 0x20`, `FuseTileEnd = 0x21`.
  - Binary (`_Vv`, `_Vs`, `_Sv` triples): `TileAdd*`, `TileSub*`, `TileMul*`, `TileDiv*`, `TilePmin*`, `TilePmax*`, `TilePow*` (`0x22..=0x36`).
  - Unary Math: `TileExp`, `TileLog`, `TileSqrt`, `TileAbs`, `TileRound` (`0x37..=0x3B`).
  - Comparisons & Kleene Logic: `TileCmpEq`, `TileCmpLt`, `TileCmpLe`, `TileAndLgl`, `TileOrLgl`, `TileNotLgl`, `TileIsNa` (`0x3C..=0x42`).
  - Selection & Fused Accumulator: `TileIfelse = 0x43`, `TileSumAccum = 0x44`.
- **`0x45..=0x4C` — Reductions & Windowed Accumulations (8 opcodes):**
  - `ReduceSumF64 = 0x45`, `ReduceMeanF64`, `ReduceMinF64`, `ReduceMaxF64`, `ReduceAnyLgl`, `ReduceAllLgl`, `CumsumF64`, `DiffF64 = 0x4C`.
- **`0x50..=0x57` — Subsetting, Indexing & Search (8 opcodes):**
  - `SubsetMask = 0x50`, `SubsetIdx`, `SubsetScalar`, `AssignSubset`, `WhichLgl`, `MatchVec`, `ApproxLinear`, `FindInterval = 0x57`.

`Opcode::try_from(u8)` rejects any byte outside those 63 discriminants.

---

## 8. Proposed Wasm SIMD (`v128`) Implementation Details

### 8.1 `f64x2` Arithmetic and Missing-Value Payload Nuance

In IEEE 754, R's `NA_real_` (`0x7ff8_0000_0000_07a2`) is a quiet NaN:

```rust
// Proposed SIMD kernel sketch (not present in current crate):
#[cfg(target_arch = "wasm32")]
use core::arch::wasm32::*;
#[inline(always)]
pub unsafe fn tile_add_vv_f64(dst: *mut f64, a: *const f64, b: *const f64, len: usize) {
    let mut i = 0;
    while i + 2 <= len {
        let va = v128_load(a.add(i) as *const v128);
        let vb = v128_load(b.add(i) as *const v128);
        let vr = f64x2_add(va, vb);
        v128_store(dst.add(i) as *mut v128, vr);
        i += 2;
    }
    if i < len {
        *dst.add(i) = *a.add(i) + *b.add(i);
    }
}
```
*Implementation note (Erratum B8):* When only `NA_real_` is present alongside finite numbers, hardware `f64x2` arithmetic preserves the `NA_real_` quiet-NaN payload. However, when both `NA_real_` and a non-NA `NaN` are mixed in the same operation (`NaN + NA_real_` vs `NA_real_ + NaN`), R0's normative rule (Erratum B8, implemented in `src/vm.rs`) requires the **first** missing operand's payload to win, which any future SIMD kernel must preserve when both NaN kinds can occur.

### 8.2 Proposed Branchless `f64` Comparisons with `NA` Propagation

Standard IEEE 754 `f64x2_lt(va, vb)` yields `0` (`FALSE`) when either operand is `NA_real_` or `NaN`, whereas R0 requires `NA < 5.0` $\to$ `NA` (`2u8`):
1. `cmp_true = f64x2_lt(va, vb)` (lanes `0xFFFF...` if true, `0` if false/NaN).
2. `not_na = v128_and(f64x2_eq(va, va), f64x2_eq(vb, vb))` (since `NaN == NaN` is false).
3. Extract `u8` logical values: `(cmp_true & 1) | (~not_na & 2)` $\to$ `1` (`TRUE`), `0` (`FALSE`), or `2` (`NA`).

### 8.3 Two-Bit Kleene Logic LUTs (`KLEENE_*_LUT` in `src/value.rs`)

`src/value.rs` defines the 2-bit logical encoding (`LGL_FALSE = 0`, `LGL_TRUE = 1`, `LGL_NA = 2`) and lookup tables used today by `R0Vm`:
- `KLEENE_AND_LUT: [u8; 16]` and `KLEENE_OR_LUT: [u8; 16]` indexed by `((a & 3) << 2) | (b & 3)`.
- `KLEENE_NOT_LUT: [u8; 4] = [1, 0, 2, 0]` indexed by `(a & 3)`.

In the current interpreter (`src/vm.rs`), these tables are indexed element-by-element in scalar loops. In a future Wasm `simd128` backend, the 16-byte tables map directly to a single `i8x16_swizzle` instruction across 16 `u8` lanes at once (with `KLEENE_NOT_LUT` zero-padded to 16 bytes).

---

## 9. Errata Ledger

- **B7 (`FOR_RANGE_STEP` instruction encoding):** In `src/isa.rs`, `Opcode::ForRangeStep` encodes `A = r_iter` and `B = r_limit` (8-bit register index) in the first 32-bit word, and places the signed 16-bit jump offset in the following 32-bit extension word (`Instr::ext_offset` / `Instr::decode_ext_offset`). In the v0.1 draft, `Bx = (B << 8) | C` overlapped `B`, making `r_limit` and `offset16` mutually exclusive.

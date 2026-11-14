//! Instruction set architecture definitions.
//!
//! Layout and encoding only (D5). Each instruction is a `u32`:
//! opcode[31:24], A[23:16], B[15:8], C[7:0]. Bx = (B << 8) | C.

/// Opcodes from INITIAL_SPEC §4 (63 named), with amended ranges (E13):
/// 0x00-0x0F, 0x10-0x1F, 0x20-0x44 (TILE), 0x45-0x4C (REDUCE/CUMULATIVE), 0x50-0x57 (SUBSET).
#[repr(u8)]
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Opcode {
    // 0x00-0x0F: load / move / lifecycle
    LoadConst = 0x00,
    MoveCow,
    FreeDead,
    MakeRange,

    // 0x10-0x1F: unboxed scalar fast path
    SAddF64 = 0x10,
    SSubF64,
    SMulF64,
    SDivF64,
    SCmpLtF64,
    ForRangeStep,

    // 0x20-0x44: strip-mined SIMD tile kernels (_VV, _VS, _SV order)
    FuseTileBegin = 0x20,
    FuseTileEnd,
    TileAddVv,
    TileAddVs,
    TileAddSv,
    TileSubVv,
    TileSubVs,
    TileSubSv,
    TileMulVv,
    TileMulVs,
    TileMulSv,
    TileDivVv,
    TileDivVs,
    TileDivSv,
    TilePminVv,
    TilePminVs,
    TilePminSv,
    TilePmaxVv,
    TilePmaxVs,
    TilePmaxSv,
    TilePowVv,
    TilePowVs,
    TilePowSv,
    TileExp,
    TileLog,
    TileSqrt,
    TileAbs,
    TileRound,
    TileCmpEq,
    TileCmpLt,
    TileCmpLe,
    TileAndLgl,
    TileOrLgl,
    TileNotLgl,
    TileIsNa,
    /// Operands: A, B(cond), C(yes_tile); the fourth operand (no_tile) is in the following word.
    TileIfelse,
    TileSumAccum,

    // 0x45-0x4C: reductions and cumulative ops
    ReduceSumF64 = 0x45,
    ReduceMeanF64,
    ReduceMinF64,
    ReduceMaxF64,
    ReduceAnyLgl,
    ReduceAllLgl,
    CumsumF64,
    DiffF64,

    // 0x50-0x57: subsetting, indexing, search
    SubsetMask = 0x50,
    SubsetIdx,
    SubsetScalar,
    AssignSubset,
    WhichLgl,
    MatchVec,
    ApproxLinear,
    FindInterval,
}

impl Opcode {
    /// All 63 opcodes in discriminant order.
    pub const ALL: [Opcode; 63] = [
        Opcode::LoadConst, Opcode::MoveCow, Opcode::FreeDead, Opcode::MakeRange,
        Opcode::SAddF64, Opcode::SSubF64, Opcode::SMulF64, Opcode::SDivF64,
        Opcode::SCmpLtF64, Opcode::ForRangeStep,
        Opcode::FuseTileBegin, Opcode::FuseTileEnd,
        Opcode::TileAddVv, Opcode::TileAddVs, Opcode::TileAddSv,
        Opcode::TileSubVv, Opcode::TileSubVs, Opcode::TileSubSv,
        Opcode::TileMulVv, Opcode::TileMulVs, Opcode::TileMulSv,
        Opcode::TileDivVv, Opcode::TileDivVs, Opcode::TileDivSv,
        Opcode::TilePminVv, Opcode::TilePminVs, Opcode::TilePminSv,
        Opcode::TilePmaxVv, Opcode::TilePmaxVs, Opcode::TilePmaxSv,
        Opcode::TilePowVv, Opcode::TilePowVs, Opcode::TilePowSv,
        Opcode::TileExp, Opcode::TileLog, Opcode::TileSqrt, Opcode::TileAbs, Opcode::TileRound,
        Opcode::TileCmpEq, Opcode::TileCmpLt, Opcode::TileCmpLe,
        Opcode::TileAndLgl, Opcode::TileOrLgl, Opcode::TileNotLgl, Opcode::TileIsNa,
        Opcode::TileIfelse, Opcode::TileSumAccum,
        Opcode::ReduceSumF64, Opcode::ReduceMeanF64, Opcode::ReduceMinF64, Opcode::ReduceMaxF64,
        Opcode::ReduceAnyLgl, Opcode::ReduceAllLgl, Opcode::CumsumF64, Opcode::DiffF64,
        Opcode::SubsetMask, Opcode::SubsetIdx, Opcode::SubsetScalar, Opcode::AssignSubset,
        Opcode::WhichLgl, Opcode::MatchVec, Opcode::ApproxLinear, Opcode::FindInterval,
    ];
}

/// Rejects bytes that are not an opcode discriminant. Returns the bad byte.
impl TryFrom<u8> for Opcode {
    type Error = u8;

    fn try_from(b: u8) -> Result<Self, u8> {
        Opcode::ALL.iter().copied().find(|op| *op as u8 == b).ok_or(b)
    }
}

/// Tile kernel names for 0x20-0x44 (no encoding helpers beyond the names).
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum TileKernel {
    Add,
    Sub,
    Mul,
    Div,
    Pmin,
    Pmax,
    Pow,
    Exp,
    Log,
    Sqrt,
    Abs,
    Round,
    CmpEq,
    CmpLt,
    CmpLe,
    AndLgl,
    OrLgl,
    NotLgl,
    IsNa,
    Ifelse,
}

/// One 32-bit instruction word.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct Instr(pub u32);

impl Instr {
    pub fn new(op: Opcode, a: u8, b: u8, c: u8) -> Self {
        Instr(((op as u32) << 24) | ((a as u32) << 16) | ((b as u32) << 8) | c as u32)
    }

    /// Returns the raw opcode byte's error if it is not defined.
    pub fn opcode(&self) -> Result<Opcode, u8> {
        Opcode::try_from((self.0 >> 24) as u8)
    }

    pub fn a(&self) -> u8 {
        (self.0 >> 16) as u8
    }

    pub fn b(&self) -> u8 {
        (self.0 >> 8) as u8
    }

    pub fn c(&self) -> u8 {
        self.0 as u8
    }

    /// Wide operand: B and C combined as a 16-bit index.
    pub fn bx(&self) -> u16 {
        (self.0 & 0xFFFF) as u16
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn round_trip_every_opcode() {
        for op in Opcode::ALL {
            let i = Instr::new(op, 1, 2, 3);
            assert_eq!(i.opcode(), Ok(op));
            assert_eq!((i.a(), i.b(), i.c(), i.bx()), (1, 2, 3, 0x0203));
        }
    }

    #[test]
    fn undefined_bytes_rejected() {
        assert_eq!(Opcode::try_from(0x0F), Err(0x0F));
        assert_eq!(Opcode::try_from(0x4D), Err(0x4D));
        assert_eq!(Opcode::try_from(0x58), Err(0x58));
        let defined = (0u8..=255).filter(|b| Opcode::try_from(*b).is_ok()).count();
        assert_eq!(defined, 63);
    }

    #[test]
    fn discriminants_in_family_ranges() {
        for op in Opcode::ALL {
            let d = op as u8;
            let ok = matches!(d, 0x00..=0x0F | 0x10..=0x1F | 0x20..=0x44 | 0x45..=0x4C | 0x50..=0x57);
            assert!(ok, "{op:?} = {d:#04x} outside its family range");
        }
        assert_eq!(Opcode::TileSumAccum as u8, 0x44);
        assert_eq!(Opcode::TileAddVv as u8, 0x22);
        assert_eq!(Opcode::ReduceSumF64 as u8, 0x45);
        assert_eq!(Opcode::FindInterval as u8, 0x57);
    }

    #[test]
    fn for_range_step_offset_word_roundtrip() {
        // B7: head word carries A = iter and B = limit; the offset lives in the next word.
        let head = Instr::new(Opcode::ForRangeStep, 3, 7, 0);
        assert_eq!((head.a(), head.b()), (3, 7));
        for off in [-32768i16, -1, 0, 1, 32767] {
            let w = Instr::ext_offset(off);
            assert_eq!(Instr::decode_ext_offset(w), off, "offset {off}");
        }
    }
}

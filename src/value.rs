//! Runtime value representation.

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
/// True only for NA_real_: a NaN whose low 32-bit word is R's 1954 payload (B3).
#[inline(always)]
pub fn is_na_real(x: f64) -> bool {
    x.is_nan() && (x.to_bits() & 0xffff_ffff) == 1954
}
/// Missing for `is.na` and missing-value paths: NA or any other NaN (B3).
#[inline(always)]
pub fn is_missing(x: f64) -> bool {
    x.is_nan()
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

// ============================================================================
// 2. R0Vec 16-BYTE DESCRIPTOR (spec §2.1)
// ============================================================================
/// Bits 0..3: TypeTag
pub const TAG_DBL: u32 = 0;
pub const TAG_INT: u32 = 1;
pub const TAG_LGL: u32 = 2;
pub const TAG_CHR: u32 = 3;
pub const TAG_RANGE: u32 = 4;
pub const TAG_DF: u32 = 5;
pub const TAG_LIST: u32 = 6;
pub const TAG_MASK: u32 = 0x0f;
/// Bit 4: element stored unboxed in `payload` (length == 1)
pub const IS_SCALAR_UNBOXED: u32 = 1 << 4;
/// Bit 5: zero-copy borrow of host memory, do not free
pub const IS_HOST_PINNED: u32 = 1 << 5;
/// Bit 6: Arrow validity bitmap attached
pub const HAS_NULL_BITMAP: u32 = 1 << 6;
/// Bits 16..31: reference count (u16, 0 = dead/unique)
pub const REFCOUNT_SHIFT: u32 = 16;
pub const REFCOUNT_MASK: u32 = 0xffff << REFCOUNT_SHIFT;

#[repr(C, align(16))]
#[derive(Clone, Copy, Debug)]
pub struct R0Vec {
    pub tag_flags: u32,
    pub len: u32,
    pub payload: u64,
}

const _: () = assert!(std::mem::size_of::<R0Vec>() == 16);
const _: () = assert!(std::mem::align_of::<R0Vec>() == 16);

impl R0Vec {
    /// Type tag from bits 0..3 of `tag_flags`.
    pub fn tag(&self) -> u32 {
        self.tag_flags & TAG_MASK
    }
}

// ============================================================================
// 3. RUNTIME VALUES
// ============================================================================
/// Function value payload (params, body, captured env).
pub type UserFnValue = Box<crate::vm::UserFn>;

#[derive(Clone, Debug)]
pub enum R0Val {
    Dbl(Vec<f64>),
    Int(Vec<i32>),
    Lgl(Vec<Lgl>),
    Chr(Vec<Option<String>>),
    /// Lazy 12-byte range descriptor (start, end, step)
    Range {
        start: i32,
        end: i32,
        step: i32,
    },
    /// Rectangular DataFrame (equal-length columns, see `R0Val::df`)
    Df {
        names: Vec<String>,
        cols: Vec<R0Val>,
    },
    /// Named list (no equal-length rule, D12)
    List {
        names: Vec<String>,
        cols: Vec<R0Val>,
    },
    Null,
    /// Function value (no evaluation behaviour yet)
    Fn(UserFnValue),
}

impl R0Val {
    /// Build a data.frame, enforcing equal column lengths.
    pub fn df(names: Vec<String>, cols: Vec<R0Val>) -> Result<R0Val, String> {
        let expected = cols.first().map_or(0, |c| c.len());
        for (name, col) in names.iter().zip(cols.iter()) {
            if col.len() != expected {
                return Err(format!(
                    "ErrDimensionMismatch: data.frame column '{}' has length {}, expected {}",
                    name,
                    col.len(),
                    expected
                ));
            }
        }
        Ok(R0Val::Df { names, cols })
    }

    /// `n` NAs of the same variant as `self`. Variants with no NA form yield `Null`.
    pub fn na_of_type_like(&self, n: usize) -> R0Val {
        match self {
            R0Val::Dbl(_) => R0Val::Dbl(vec![na_real(); n]),
            R0Val::Int(_) | R0Val::Range { .. } => R0Val::Int(vec![NA_INT; n]),
            R0Val::Lgl(_) => R0Val::Lgl(vec![LGL_NA; n]),
            R0Val::Chr(_) => R0Val::Chr(vec![None; n]),
            R0Val::Df { .. } | R0Val::List { .. } | R0Val::Null | R0Val::Fn(_) => R0Val::Null,
        }
    }

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
                    let distance = (*end as i64 - *start as i64).abs();
                    let stride = (*step as i64).abs();
                    usize::try_from(distance / stride + 1).unwrap_or(usize::MAX)
                }
            }
            R0Val::Df { cols, .. } | R0Val::List { cols, .. } => {
                cols.first().map_or(0, |c| c.len())
            }
            R0Val::Null => 0,
            R0Val::Fn(_) => 1,
        }
    }

    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    pub fn materialize_int(&self) -> Result<Vec<i32>, String> {
        match self {
            R0Val::Int(v) => Ok(v.clone()),
            R0Val::Range {
                start,
                end: _,
                step,
            } => {
                let n = self.len();
                let mut out = Vec::new();
                out.try_reserve_exact(n).map_err(|_| {
                    "ErrFuelExhausted: range allocation exceeds available resources".to_string()
                })?;
                out.extend((0..n).map(|i| (*start as i64 + i as i64 * *step as i64) as i32));
                Ok(out)
            }
            R0Val::Dbl(v) => Ok(v
                .iter()
                .map(|&x| {
                    if is_missing(x) || x < (i32::MIN + 1) as f64 || x > i32::MAX as f64 {
                        NA_INT
                    } else {
                        x as i32
                    }
                })
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
            R0Val::Range {
                start,
                end: _,
                step,
            } => {
                let n = self.len();
                let mut out = Vec::new();
                out.try_reserve_exact(n).map_err(|_| {
                    "ErrFuelExhausted: range allocation exceeds available resources".to_string()
                })?;
                out.extend((0..n).map(|i| (*start as i64 + i as i64 * *step as i64) as f64));
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
                    .map(|&x| {
                        if x == NA_INT {
                            "NA".into()
                        } else {
                            format!("{}L", x)
                        }
                    })
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
            R0Val::List { names, cols } => {
                writeln!(f, "<R0 list: {} fields>", names.len())?;
                for (name, col) in names.iter().zip(cols.iter()) {
                    writeln!(f, " ${}: {}", name, col)?;
                }
                Ok(())
            }
            R0Val::Null => write!(f, "NULL"),
            R0Val::Fn(_) => write!(f, "<function>"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn na_is_na_and_not_nan_payload() {
        assert!(is_na_real(na_real()));
        assert!(!is_na_real(f64::NAN));
        assert!(!is_na_real(
            std::hint::black_box(0.0_f64) / std::hint::black_box(0.0_f64)
        ));
    }

    #[test]
    fn is_missing_covers_na_and_nan() {
        assert!(is_missing(na_real()));
        assert!(is_missing(f64::NAN));
        assert!(!is_missing(1.0));
    }

    #[test]
    fn nan_prints_as_nan_and_na_as_na() {
        assert_eq!(format!("{}", R0Val::Dbl(vec![f64::NAN])), "[1] NaN");
        assert_eq!(format!("{}", R0Val::Dbl(vec![na_real()])), "[1] NA");
    }

    #[test]
    fn r0vec_size_and_align_are_16() {
        assert_eq!(std::mem::size_of::<R0Vec>(), 16);
        assert_eq!(std::mem::align_of::<R0Vec>(), 16);
    }

    #[test]
    fn r0vec_tag_masks_bits_0_to_3() {
        let v = R0Vec {
            tag_flags: TAG_LIST | IS_SCALAR_UNBOXED | (7 << REFCOUNT_SHIFT),
            len: 1,
            payload: 0,
        };
        assert_eq!(v.tag(), TAG_LIST);
    }

    #[test]
    fn df_unequal_columns_returns_exact_error() {
        let cols = vec![R0Val::Dbl(vec![1.0, 2.0]), R0Val::Int(vec![1])];
        let err = R0Val::df(vec!["a".into(), "b".into()], cols).unwrap_err();
        assert_eq!(
            err,
            "ErrDimensionMismatch: data.frame column 'b' has length 1, expected 2"
        );
    }

    #[test]
    fn kleene_not_lut_maps_0_1_2() {
        assert_eq!(KLEENE_NOT_LUT[LGL_FALSE as usize], LGL_TRUE);
        assert_eq!(KLEENE_NOT_LUT[LGL_TRUE as usize], LGL_FALSE);
        assert_eq!(KLEENE_NOT_LUT[LGL_NA as usize], LGL_NA);
    }

    #[test]
    fn range_len_matches_materialized_len() {
        let asc = R0Val::Range {
            start: 1,
            end: 5,
            step: 1,
        };
        assert_eq!(asc.len(), asc.materialize_int().unwrap().len());
        let desc = R0Val::Range {
            start: 5,
            end: 1,
            step: -1,
        };
        assert_eq!(desc.len(), desc.materialize_int().unwrap().len());
    }

    #[test]
    fn range_len_uses_widened_arithmetic() {
        let r = R0Val::Range {
            start: -2_000_000_000,
            end: 2_000_000_000,
            step: 1,
        };
        assert_eq!(r.len(), 4_000_000_001);
    }

    #[test]
    fn range_materialization_does_not_increment_past_last_value() {
        let r = R0Val::Range {
            start: i32::MAX,
            end: i32::MAX,
            step: 1,
        };
        assert_eq!(r.materialize_int().unwrap(), vec![i32::MAX]);
        assert_eq!(r.promote_dbl().unwrap(), vec![i32::MAX as f64]);
    }

    #[test]
    fn out_of_range_double_cast_becomes_na_integer() {
        let got = R0Val::Dbl(vec![
            i32::MAX as f64 + 1.0,
            (i32::MIN + 1) as f64 - 1.0,
            1.5,
        ]);
        assert_eq!(got.materialize_int().unwrap(), vec![NA_INT, NA_INT, 1]);
    }
}

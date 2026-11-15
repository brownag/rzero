//! GNU R equivalence harness: canonical text form of R0 values.
//!
//! `canon_r0` must produce exactly the same text as `canon()` in
//! `tests/gnu_r/canon.R` for the same value.

use std::path::PathBuf;
use std::process::Command;
use std::sync::atomic::{AtomicUsize, Ordering};

use r0_wasm::value::{is_na_real, na_real, R0Val, LGL_FALSE, LGL_NA, LGL_TRUE, NA_INT};

const CANON_R: &str = concat!(env!("CARGO_MANIFEST_DIR"), "/tests/gnu_r/canon.R");

/// `%.15g` for a finite, non-zero-or-zero double (Rust has no `%g`).
fn fmt_g15(x: f64) -> String {
    // `{:.14e}` rounds to 15 significant digits; the exponent after rounding
    // decides fixed vs exponential notation, as in C's `%g`.
    let sci = format!("{:.14e}", x);
    let (mant, exp) = sci.split_once('e').expect("exponential form");
    let e: i32 = exp.parse().expect("exponent");
    if !(-4..15).contains(&e) {
        let sign = if e < 0 { '-' } else { '+' };
        format!("{}e{}{:02}", strip_zeros(mant), sign, e.abs())
    } else {
        let prec = (14 - e) as usize;
        strip_zeros(&format!("{:.*}", prec, x))
    }
}

fn strip_zeros(s: &str) -> String {
    if s.contains('.') {
        s.trim_end_matches('0').trim_end_matches('.').to_string()
    } else {
        s.to_string()
    }
}

fn canon_escape(s: &str) -> String {
    let mut out = String::from("\"");
    for c in s.chars() {
        match c {
            '\\' => out.push_str("\\\\"),
            '"' => out.push_str("\\\""),
            '\n' => out.push_str("\\n"),
            c => out.push(c),
        }
    }
    out.push('"');
    out
}

fn canon_dbl(x: f64) -> String {
    if x.is_nan() {
        if is_na_real(x) {
            "NA".into()
        } else {
            "NaN".into()
        }
    } else if x == f64::INFINITY {
        "Inf".into()
    } else if x == f64::NEG_INFINITY {
        "-Inf".into()
    } else {
        fmt_g15(x)
    }
}

/// Canonical lines for a value (see `tests/gnu_r/canon.R`).
fn canon_lines(v: &R0Val) -> Vec<String> {
    fn atomic(tag: &str, elems: Vec<String>) -> Vec<String> {
        let mut line = format!("{}[{}]:", tag, elems.len());
        for e in elems {
            line.push(' ');
            line.push_str(&e);
        }
        vec![line]
    }
    fn container(tag: &str, names: &[String], cols: &[R0Val]) -> Vec<String> {
        let mut lines = vec![format!("{}[{}]", tag, cols.len())];
        for (i, col) in cols.iter().enumerate() {
            lines.push(format!("  {}", canon_escape(&names[i])));
            lines.extend(canon_lines(col).into_iter().map(|l| format!("    {}", l)));
        }
        lines
    }
    match v {
        R0Val::Null => vec!["NULL".into()],
        R0Val::Dbl(xs) => atomic("double", xs.iter().map(|&x| canon_dbl(x)).collect()),
        R0Val::Int(xs) => atomic(
            "integer",
            xs.iter()
                .map(|&x| if x == NA_INT { "NA".into() } else { x.to_string() })
                .collect(),
        ),
        R0Val::Range { start, step, .. } => {
            let elems = (0..v.len())
                .map(|i| (*start as i64 + i as i64 * *step as i64).to_string())
                .collect();
            atomic("integer", elems)
        }
        R0Val::Lgl(xs) => atomic(
            "logical",
            xs.iter()
                .map(|&b| match b {
                    LGL_FALSE => "FALSE".into(),
                    LGL_TRUE => "TRUE".into(),
                    LGL_NA => "NA".into(),
                    _ => "NA".into(),
                })
                .collect(),
        ),
        R0Val::Chr(xs) => atomic(
            "character",
            xs.iter()
                .map(|x| match x {
                    Some(s) => canon_escape(s),
                    None => "NA".into(),
                })
                .collect(),
        ),
        R0Val::List { names, cols } => container("list", names, cols),
        R0Val::Df { names, cols } => container("data.frame", names, cols),
    }
}

/// Canonical text for a value; lines joined with `\n`, no trailing newline.
pub fn canon_r0(v: &R0Val) -> String {
    canon_lines(v).join("\n")
}

fn rscript_available() -> bool {
    Command::new("Rscript")
        .arg("--version")
        .output()
        .map(|o| o.status.success())
        .unwrap_or(false)
}

static TEMP_COUNTER: AtomicUsize = AtomicUsize::new(0);

/// Run `Rscript` on a generated file that prints `canon(<r_expr>)`.
fn canon_via_rscript(r_expr: &str) -> String {
    let n = TEMP_COUNTER.fetch_add(1, Ordering::SeqCst);
    let path: PathBuf = std::env::temp_dir().join(format!(
        "r0_gnu_canon_{}_{}.R",
        std::process::id(),
        n
    ));
    let script = format!(
        "source({:?})\nwriteLines(canon({}))\n",
        CANON_R, r_expr
    );
    std::fs::write(&path, script).expect("write temp R script");
    let out = Command::new("Rscript")
        .arg(&path)
        .output()
        .expect("run Rscript");
    let _ = std::fs::remove_file(&path);
    assert!(
        out.status.success(),
        "Rscript failed: {}",
        String::from_utf8_lossy(&out.stderr)
    );
    String::from_utf8(out.stdout)
        .expect("utf8 stdout")
        .trim_end_matches('\n')
        .to_string()
}

fn s(x: &str) -> Option<String> {
    Some(x.to_string())
}

/// (label, R0 value, R expression building the same value)
fn fixed_cases() -> Vec<(&'static str, R0Val, &'static str)> {
    vec![
        (
            "dbl specials",
            R0Val::Dbl(vec![1.0, na_real(), f64::NAN, f64::INFINITY, f64::NEG_INFINITY, -0.0, 0.1 + 0.2]),
            "c(1, NA_real_, NaN, Inf, -Inf, -0, 0.1+0.2)",
        ),
        (
            "dbl magnitudes",
            R0Val::Dbl(vec![1e-5, 1e15, 123456789012345678.0]),
            "c(1e-5, 1e15, 123456789012345678)",
        ),
        ("dbl empty", R0Val::Dbl(vec![]), "numeric(0)"),
        (
            "int with NA",
            R0Val::Int(vec![1, NA_INT, -7]),
            "c(1L, NA_integer_, -7L)",
        ),
        (
            "lgl with NA",
            R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE, LGL_NA]),
            "c(TRUE, FALSE, NA)",
        ),
        (
            "chr escapes",
            R0Val::Chr(vec![s("a\"b"), s("line\nbreak"), None, s("back\\slash")]),
            "c('a\"b', \"line\\nbreak\", NA_character_, \"back\\\\slash\")",
        ),
        ("null", R0Val::Null, "NULL"),
        (
            "range ascending",
            R0Val::Range { start: 1, end: 5, step: 1 },
            "1:5",
        ),
        (
            "range descending",
            R0Val::Range { start: 5, end: 1, step: -2 },
            "seq(5L, 1L, by = -2L)",
        ),
        (
            "named list nested",
            R0Val::List {
                names: vec!["a".into(), "b".into()],
                cols: vec![
                    R0Val::Dbl(vec![1.0]),
                    R0Val::List {
                        names: vec!["x".into()],
                        cols: vec![R0Val::Lgl(vec![LGL_TRUE])],
                    },
                ],
            },
            "list(a = 1, b = list(x = TRUE))",
        ),
        (
            "data frame",
            R0Val::Df {
                names: vec!["x".into(), "y".into()],
                cols: vec![
                    R0Val::Int(vec![1, 2]),
                    R0Val::Chr(vec![s("p"), None]),
                ],
            },
            "data.frame(x = c(1L, 2L), y = c(\"p\", NA), stringsAsFactors = FALSE)",
        ),
    ]
}

#[test]
fn canon_matches_on_fixed_values() {
    if !rscript_available() {
        eprintln!("gnu_r: Rscript not found, skipping");
        return;
    }
    for (label, val, r_expr) in fixed_cases() {
        let rust = canon_r0(&val);
        let r = canon_via_rscript(r_expr);
        assert_eq!(rust, r, "canon mismatch for case `{}`", label);
    }
}

#[test]
fn canon_fmt_g15_edge_cases() {
    assert_eq!(fmt_g15(0.1 + 0.2), "0.3");
    assert_eq!(fmt_g15(1e-5), "1e-05");
    assert_eq!(fmt_g15(1e15), "1e+15");
    assert_eq!(fmt_g15(123456789012345678.0), "1.23456789012346e+17");
    assert_eq!(fmt_g15(-0.0), "-0");
    assert_eq!(fmt_g15(100000.0), "100000");
    assert_eq!(fmt_g15(0.0001), "0.0001");
}

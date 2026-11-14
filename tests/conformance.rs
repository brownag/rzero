//! Conformance tests: newline handling, the `|>` pipe, Kleene logic, broadcasting and indexing.

use r0_wasm::value::{is_na_real, R0Val, LGL_FALSE, LGL_NA, LGL_TRUE, NA_INT};
use r0_wasm::vm::R0Vm;

fn eval(src: &str) -> R0Val {
    R0Vm::new(100_000)
        .eval_script(src)
        .unwrap_or_else(|e| panic!("expected Ok for {:?}, got Err: {}", src, e))
}

fn eval_err(src: &str) -> String {
    match R0Vm::new(100_000).eval_script(src) {
        Ok(v) => panic!("expected Err for {:?}, got Ok: {:?}", src, v),
        Err(e) => e,
    }
}

fn assert_val(src: &str, want: R0Val) {
    assert_eq!(format!("{:?}", eval(src)), format!("{:?}", want), "src: {:?}", src);
}

#[test]
fn newline_binary_op_continues() {
    assert_val("1 +\n 2", R0Val::Dbl(vec![3.0]));
}

#[test]
fn newline_statement_terminates() {
    assert_val("1\n2", R0Val::Dbl(vec![2.0]));
}

#[test]
fn newline_inside_parens_suppressed() {
    assert_val("sum(c(1,\n 2))", R0Val::Dbl(vec![3.0]));
}

#[test]
fn newline_inside_brackets_suppressed() {
    assert_val("x <- c(10,20,30)\n x[\n2]", R0Val::Dbl(vec![20.0]));
}

#[test]
fn newline_multiline_pipeline() {
    assert_val("c(1,2,3) |>\n sum()", R0Val::Dbl(vec![6.0]));
}

#[test]
fn pipe_implicit_first_arg() {
    assert_val("x <- c(1, NA, 3)\n x |> sum(na.rm = TRUE)", R0Val::Dbl(vec![4.0]));
}

#[test]
fn pipe_placeholder_substitution() {
    assert_val(
        "xs <- c(1,2,3)\n ys <- c(10,20,30)\n xs |> approx(xs, ys, xout = _)",
        R0Val::Dbl(vec![10.0, 20.0, 30.0]),
    );
}

#[test]
fn pipe_rhs_not_call() {
    let err = eval_err("1 |> 2");
    assert!(
        err.contains("Right-hand side of '|>' pipe must be a function call"),
        "unexpected error: {}",
        err
    );
}

const TRI: [(&str, u8); 3] = [("TRUE", LGL_TRUE), ("FALSE", LGL_FALSE), ("NA", LGL_NA)];

fn lgl(v: u8) -> R0Val {
    R0Val::Lgl(vec![v])
}

#[test]
fn kleene_and_full_table() {
    // Standard Kleene AND; rows are the left operand, columns the right.
    let want = [
        (LGL_TRUE, LGL_TRUE, LGL_TRUE),
        (LGL_TRUE, LGL_FALSE, LGL_FALSE),
        (LGL_TRUE, LGL_NA, LGL_NA),
        (LGL_FALSE, LGL_TRUE, LGL_FALSE),
        (LGL_FALSE, LGL_FALSE, LGL_FALSE),
        (LGL_FALSE, LGL_NA, LGL_FALSE),
        (LGL_NA, LGL_TRUE, LGL_NA),
        (LGL_NA, LGL_FALSE, LGL_FALSE),
        (LGL_NA, LGL_NA, LGL_NA),
    ];
    for (a, b, r) in want {
        let (sa, sb) = (name(a), name(b));
        assert_val(&format!("{} & {}", sa, sb), lgl(r));
    }
}

#[test]
fn kleene_or_full_table() {
    // Standard Kleene OR; rows are the left operand, columns the right.
    let want = [
        (LGL_TRUE, LGL_TRUE, LGL_TRUE),
        (LGL_TRUE, LGL_FALSE, LGL_TRUE),
        (LGL_TRUE, LGL_NA, LGL_TRUE),
        (LGL_FALSE, LGL_TRUE, LGL_TRUE),
        (LGL_FALSE, LGL_FALSE, LGL_FALSE),
        (LGL_FALSE, LGL_NA, LGL_NA),
        (LGL_NA, LGL_TRUE, LGL_TRUE),
        (LGL_NA, LGL_FALSE, LGL_NA),
        (LGL_NA, LGL_NA, LGL_NA),
    ];
    for (a, b, r) in want {
        let (sa, sb) = (name(a), name(b));
        assert_val(&format!("{} | {}", sa, sb), lgl(r));
    }
}

fn name(v: u8) -> &'static str {
    TRI.iter().find(|(_, x)| *x == v).map(|(s, _)| *s).unwrap()
}

#[test]
fn kleene_not() {
    assert_val("!TRUE", lgl(LGL_FALSE));
    assert_val("!FALSE", lgl(LGL_TRUE));
    assert_val("!NA", lgl(LGL_NA));
}

#[test]
fn broadcast_scalar_vector() {
    assert_val("1 + c(10, 20)", R0Val::Dbl(vec![11.0, 21.0]));
}

#[test]
fn broadcast_equal_length() {
    assert_val("c(1, 2) + c(3, 4)", R0Val::Dbl(vec![4.0, 6.0]));
}

#[test]
fn broadcast_mismatch_traps() {
    let err = eval_err("c(1, 2, 3, 4) + c(10, 20)");
    assert!(err.starts_with("ErrDimensionMismatch:"), "{}", err);
}

#[test]
fn kleene_if_na_traps() {
    let err = eval_err("if (NA) 1");
    assert!(err.starts_with("ErrInvalidCondition:"), "{}", err);
}

#[test]
fn kleene_if_vector_traps() {
    let err = eval_err("if (c(TRUE, FALSE)) 1");
    assert!(err.starts_with("ErrInvalidCondition:"), "{}", err);
}

#[test]
fn index_positive() {
    assert_val("x <- c(10,20,30)\n x[1]", R0Val::Dbl(vec![10.0]));
}

#[test]
fn index_oob_na_padding() {
    match eval("x <- c(10,20,30)\n x[99L]") {
        R0Val::Dbl(v) => assert!(v.len() == 1 && is_na_real(v[0]), "{:?}", v),
        other => panic!("expected Dbl NA, got {:?}", other),
    }
    assert_val("y <- 1:3\n y[99L]", R0Val::Int(vec![NA_INT]));
}

#[test]
fn index_negative_drop() {
    assert_val("x <- c(10,20,30)\n x[-2L]", R0Val::Dbl(vec![10.0, 30.0]));
}

#[test]
fn index_mixed_subscript_traps() {
    let err = eval_err("x <- c(10,20,30)\n x[c(1L, -2L)]");
    assert!(err.starts_with("ErrMixedSubscript:"), "{}", err);
}

#[test]
fn index_logical_mask() {
    assert_val("x <- c(10,20,30)\n x[x > 20]", R0Val::Dbl(vec![30.0]));
}

#[test]
fn index_scalar() {
    assert_val("x <- c(10,20,30)\n x[[1L]]", R0Val::Dbl(vec![10.0]));
}

#[test]
fn index_scalar_oob_traps() {
    let err = eval_err("x <- c(10,20,30)\n x[[99L]]");
    assert!(err.starts_with("ErrBoundsTrap:"), "{}", err);
}

#[test]
fn assign_subset_mask() {
    assert_val(
        "x <- c(-1,2,-3)\n x[x < 0] <- 0\n x",
        R0Val::Dbl(vec![0.0, 2.0, 0.0]),
    );
}

#[test]
fn index_preserves_int_type() {
    assert_val("y <- 1:3\n y[2L]", R0Val::Int(vec![2]));
}

// Pedon script copied verbatim from src/main.rs (E5 form).
const PEDON_SCRIPT: &str = r#"
        depth_weighted_mean <- function(hz: df, z_min: dbl = 0, z_max: dbl = 100) -> dbl {
            o_top <- pmax(hz$top, z_min)
            o_bot <- pmin(hz$bot, z_max)
            thick <- pmax(o_bot - o_top, 0)
            valid <- !is.na(thick) & !is.na(hz$clay) & (thick > 0)
            valid_thick <- thick[valid]
            total_thick <- valid_thick |> sum(na.rm = TRUE)
            if (total_thick <= 0) {
                return(NA_real_)
            }
            sum(valid_thick * hz$clay[valid]) / total_thick
        }
        pedon <- data.frame(
            top = c(0, 18, 42, 75, 110),
            bot = c(18, 42, 75, 110, 150),
            clay = c(14.5, 24.0, 31.2, 28.0, NA_real_)
        )
        wt_clay_100 <- pedon |> depth_weighted_mean(0, 100)
        midpoints   <- (pedon$top + pedon$bot) / 2
        valid_idx   <- which(!is.na(pedon$clay))
        spline_pts <- approx(midpoints[valid_idx], pedon$clay[valid_idx], c(25, 50, 85))
        list(
            wt_clay_0_100cm = wt_clay_100,
            interp_25_50_85 = spline_pts,
            remaining_oob   = pedon$clay[c(1L, 5L, 99L)]
        )
    "#;

fn pedon_list() -> (Vec<String>, Vec<R0Val>) {
    match R0Vm::new(100_000).eval_script(PEDON_SCRIPT) {
        Ok(R0Val::List { names, cols }) => (names, cols),
        other => panic!("expected Ok(List) from pedon script, got {:?}", other),
    }
}

fn pedon_field<'a>(names: &[String], cols: &'a [R0Val], name: &str) -> &'a R0Val {
    let i = names
        .iter()
        .position(|n| n == name)
        .unwrap_or_else(|| panic!("missing list field {:?}", name));
    &cols[i]
}

fn assert_dbl_close(v: &R0Val, want: &[f64], tol: f64) {
    match v {
        R0Val::Dbl(got) => {
            assert_eq!(got.len(), want.len(), "length mismatch: {:?}", got);
            for (g, w) in got.iter().zip(want) {
                assert!((g - w).abs() < tol, "got {} want {} (tol {})", g, w, tol);
            }
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
}

#[test]
fn pedon_depth_weighted_clay_0_100() {
    let (names, cols) = pedon_list();
    assert_dbl_close(pedon_field(&names, &cols, "wt_clay_0_100cm"), &[25.666], 1e-3);
}

#[test]
fn pedon_approx_25_50_85() {
    let (names, cols) = pedon_list();
    assert_dbl_close(
        pedon_field(&names, &cols, "interp_25_50_85"),
        &[21.7381, 29.0526, 28.7059],
        1e-3,
    );
}

#[test]
fn pedon_remaining_oob() {
    let (names, cols) = pedon_list();
    match pedon_field(&names, &cols, "remaining_oob") {
        R0Val::Dbl(got) if got.len() == 3 => {
            assert!((got[0] - 14.5).abs() < 1e-3, "got {}", got[0]);
            assert!(is_na_real(got[1]), "expected NA at [2]");
            assert!(is_na_real(got[2]), "expected NA at [3]");
        }
        other => panic!("expected Dbl of length 3, got {:?}", other),
    }
}

#[test]
fn pedon_result_is_list() {
    match R0Vm::new(100_000).eval_script(PEDON_SCRIPT) {
        Ok(R0Val::List { names, .. }) => {
            let got: Vec<&str> = names.iter().map(String::as_str).collect();
            assert_eq!(got, ["wt_clay_0_100cm", "interp_25_50_85", "remaining_oob"]);
        }
        other => panic!("expected Ok(List), got {:?}", other),
    }
}

#[test]
fn fuel_infinite_loop_halts() {
    match R0Vm::new(10_000).eval_script("while (TRUE) { x <- 1 }") {
        Ok(v) => panic!("expected Err, got Ok: {:?}", v),
        Err(e) => assert!(e.starts_with("ErrFuelExhausted:"), "unexpected error: {}", e),
    }
}

#[test]
fn fuel_finite_loop_ok() {
    let src = "i <- 0\n while (i < 5) { i <- i + 1 }\n i";
    match R0Vm::new(10_000).eval_script(src) {
        Ok(v) => assert_eq!(format!("{:?}", v), format!("{:?}", R0Val::Dbl(vec![5.0]))),
        Err(e) => panic!("expected Ok for {:?}, got Err: {}", src, e),
    }
}

#[test]
fn fuel_small_budget_vector_op_halts() {
    match R0Vm::new(100).eval_script("x <- 1:10000\n x + 1") {
        Ok(v) => panic!("expected Err, got Ok: {:?}", v),
        Err(e) => assert!(e.starts_with("ErrFuelExhausted:"), "unexpected error: {}", e),
    }
}

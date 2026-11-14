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

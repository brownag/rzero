//! Conformance tests: newline handling and the `|>` pipe.

use r0_wasm::value::R0Val;
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

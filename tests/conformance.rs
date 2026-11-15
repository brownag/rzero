//! Conformance tests: newline handling, the `|>` pipe, Kleene logic, broadcasting and indexing.

use r0_wasm::lexer::tokenize;
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
    assert_eq!(
        format!("{:?}", eval(src)),
        format!("{:?}", want),
        "src: {:?}",
        src
    );
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
    assert_val(
        "x <- c(1, NA, 3)\n x |> sum(na.rm = TRUE)",
        R0Val::Dbl(vec![4.0]),
    );
}

#[test]
fn pipe_placeholder_substitution() {
    assert_val(
        "xs <- c(1,2,3)\n ys <- c(10,20,30)\n (xs |> approx(xs, ys, xout = _))$y",
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
    assert_dbl_close(
        pedon_field(&names, &cols, "wt_clay_0_100cm"),
        &[25.666],
        1e-3,
    );
}

#[test]
fn pedon_approx_25_50_85() {
    let (names, cols) = pedon_list();
    // approx returns list(x, y); the interpolated values are the y column.
    let spline = match pedon_field(&names, &cols, "interp_25_50_85") {
        R0Val::List { names, cols } => pedon_field(names, cols, "y").clone(),
        other => panic!("expected List from approx, got {:?}", other),
    };
    assert_dbl_close(&spline, &[21.7381, 29.0526, 28.7059], 1e-3);
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
        Err(e) => assert!(
            e.starts_with("ErrFuelExhausted:"),
            "unexpected error: {}",
            e
        ),
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
        Err(e) => assert!(
            e.starts_with("ErrFuelExhausted:"),
            "unexpected error: {}",
            e
        ),
    }
}

// ---- Spec errata B1-B6 (TODO.md task 7) ----

fn run_src(src: &str) -> Result<R0Val, String> {
    R0Vm::new(100_000).eval_script(src)
}

#[test]
fn spec_errata_b1_out_of_range_assignment_extends_with_na() {
    match run_src("x <- c(1,2,3)\n x[5] <- 9\n x").unwrap() {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 5);
            assert!(v[0] == 1.0 && v[1] == 2.0 && v[2] == 3.0 && is_na_real(v[3]) && v[4] == 9.0);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
}

#[test]
fn spec_errata_b2_float_with_l_suffix_is_parse_error() {
    assert!(run_src("1.5L").unwrap_err().starts_with("ParseError:"));
}

#[test]
fn spec_errata_b3_na_and_nan_are_distinct() {
    match run_src("c(NA_real_, NaN)").unwrap() {
        R0Val::Dbl(v) => assert!(is_na_real(v[0]) && !is_na_real(v[1]) && v[1].is_nan()),
        other => panic!("expected Dbl, got {:?}", other),
    }
}

#[test]
fn spec_errata_b4_is_nan_intrinsic() {
    match run_src("is.nan(c(NaN, NA_real_))").unwrap() {
        R0Val::Lgl(v) => assert_eq!(v, vec![LGL_TRUE, LGL_FALSE]),
        other => panic!("expected Lgl, got {:?}", other),
    }
}

#[test]
fn spec_errata_b5_na_in_character_c_is_na_character() {
    match run_src("c(\"a\", NA)").unwrap() {
        R0Val::Chr(v) => assert_eq!(v, vec![Some("a".to_string()), None]),
        other => panic!("expected Chr, got {:?}", other),
    }
}

#[test]
fn spec_errata_b6_is_na_on_character() {
    match run_src("is.na(c(\"a\", NA))").unwrap() {
        R0Val::Lgl(v) => assert_eq!(v, vec![LGL_FALSE, LGL_TRUE]),
        other => panic!("expected Lgl, got {:?}", other),
    }
}

#[test]
fn spec_01_na_character_literal() {
    assert_eq!(format!("{:?}", eval("NA_character_")), "Chr([None])");
}

#[test]
fn spec_02_integer_binary_arithmetic_preserves_type() {
    assert_eq!(format!("{:?}", eval("1L + 2L")), "Int([3])");
    assert_eq!(format!("{:?}", eval("5L - 2L")), "Int([3])");
    assert_eq!(format!("{:?}", eval("2L * 3L")), "Int([6])");
    assert_eq!(format!("{:?}", eval("-5L %% 3L")), "Int([1])");
    assert_eq!(format!("{:?}", eval("-5L %/% 3L")), "Int([-2])");
    assert_eq!(format!("{:?}", eval("1L / 2L")), "Dbl([0.5])");
    assert_eq!(format!("{:?}", eval("2L ^ 3L")), "Dbl([8.0])");
    assert!(eval_err("2147483647L + 1L").starts_with("ErrIntOverflow:"));
}

#[test]
fn spec_03_empty_return_is_null() {
    assert_eq!(
        format!("{:?}", eval("f <- function() { return() }; f()")),
        "Null"
    );
}

#[test]
fn spec_04_pmin_pmax_preserve_integer_type() {
    assert_eq!(format!("{:?}", eval("pmin(1L, 2L)")), "Int([1])");
    assert_eq!(format!("{:?}", eval("pmax(1L, 2L)")), "Int([2])");
}

#[test]
fn state_01_string_escapes() {
    assert!(tokenize(r#""a\"b""#).is_ok());
    assert_eq!(
        format!("{:?}", eval(r#""a\"b""#)),
        "Chr([Some(\"a\\\"b\")])"
    );
    assert_eq!(
        format!("{:?}", eval(r#"'a\\b'"#)),
        "Chr([Some(\"a\\\\b\")])"
    );
}

#[test]
fn state_04_return_unwinds_value_subexpressions() {
    assert_eq!(
        format!("{:?}", eval("f <- function() { x <- return(5); 10 }; f()")),
        "Dbl([5.0])"
    );
}

#[test]
fn seq_len_and_seq_along_zero_and_invalid_bounds() {
    assert_val("seq_len(0L)", R0Val::Int(vec![]));
    assert_val("seq_along(c())", R0Val::Int(vec![]));
    assert!(eval_err("seq_len(-1L)").contains("non-negative length-1 integer"));
    assert!(eval_err("seq_len(NA_integer_)").contains("non-negative length-1 integer"));
    assert!(eval_err("seq_len(c(1L, 2L))").contains("non-negative length-1 integer"));
}

#[test]
fn unary_pos_and_neg_range_and_type_checks() {
    assert_val("-(1:3)", R0Val::Int(vec![-1, -2, -3]));
    assert_val(
        "+(1:3)",
        R0Val::Range {
            start: 1,
            end: 3,
            step: 1,
        },
    );
    assert!(eval_err("+TRUE").contains("Unary '+' requires numeric or integer vector"));
    assert!(eval_err("-\"a\"").contains("Unary '-' requires numeric or integer vector"));
}

#[test]
fn locked_prelude_and_strict_user_function_args() {
    assert!(eval_err("sum <- function(x) 0").contains("Cannot shadow built-in intrinsic"));
    assert_val(
        "f <- function(a, b = 10) a - b; f(b = 3, 10)",
        R0Val::Dbl(vec![7.0]),
    );
    assert!(eval_err("f <- function(a) a; f(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("f <- function(a) a; f(unknown = 1)").starts_with("UnknownArgument:"));
    assert!(eval_err("f <- function(a) a; f(a = 1, a = 2)").starts_with("DuplicateArgument:"));
}

#[test]
fn na_rm_validation() {
    assert!(eval_err("sum(c(1L, NA_integer_), na.rm = c(TRUE, FALSE))")
        .starts_with("ErrInvalidCondition:"));
    assert!(eval_err("sum(c(1, 2), na.rm = NA)").starts_with("ErrInvalidCondition:"));
    assert!(eval_err("sum(c(1, 2), na.rm = 1)").starts_with("TypeError:"));
    assert!(eval_err("sum(c(1, 2), na.rm = c(TRUE)[FALSE])").starts_with("ErrInvalidCondition:"));
}

#[test]
fn conformance_g6_constructors() {
    // Default length 0
    assert_val("numeric()", R0Val::Dbl(vec![]));
    assert_val("integer()", R0Val::Int(vec![]));
    assert_val("logical()", R0Val::Lgl(vec![]));
    assert_val("character()", R0Val::Chr(vec![]));

    // Explicit 0 length
    assert_val("numeric(0)", R0Val::Dbl(vec![]));
    assert_val("integer(0L)", R0Val::Int(vec![]));
    assert_val("logical(0)", R0Val::Lgl(vec![]));
    assert_val("character(0)", R0Val::Chr(vec![]));

    // Positive lengths
    assert_val("numeric(3)", R0Val::Dbl(vec![0.0, 0.0, 0.0]));
    assert_val("integer(3L)", R0Val::Int(vec![0, 0, 0]));
    assert_val(
        "logical(3)",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
    );
    assert_val(
        "character(3)",
        R0Val::Chr(vec![
            Some(String::new()),
            Some(String::new()),
            Some(String::new()),
        ]),
    );

    // Named argument
    assert_val("numeric(length = 2)", R0Val::Dbl(vec![0.0, 0.0]));
    assert_val("integer(length = 2L)", R0Val::Int(vec![0, 0]));

    // Error cases: negative length, NA length, non-scalar, non-numeric
    assert!(eval_err("numeric(-1)").contains("requires a non-negative finite scalar length"));
    assert!(
        eval_err("integer(NA_integer_)").contains("requires a non-negative finite scalar length")
    );
    assert!(eval_err("logical(c(1, 2))").contains("requires a length-1 scalar length"));
    assert!(eval_err("character(\"abc\")").contains("requires a numeric length argument"));
    assert!(eval_err("numeric(length = 1, 2)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g6_shape_data_frame() {
    // Regular DataFrame
    assert_val(
        "df <- data.frame(a = 1:5, b = c(10, 20, 30, 40, 50)); nrow(df)",
        R0Val::Int(vec![5]),
    );
    assert_val(
        "df <- data.frame(a = 1:5, b = c(10, 20, 30, 40, 50)); ncol(df)",
        R0Val::Int(vec![2]),
    );

    // Empty DataFrame
    assert_val("df <- data.frame(); nrow(df)", R0Val::Int(vec![0]));
    assert_val("df <- data.frame(); ncol(df)", R0Val::Int(vec![0]));

    // Atomic vectors, Range, Null, List return Null (matching GNU R behavior)
    assert_val("nrow(1:5)", R0Val::Null);
    assert_val("ncol(1:5)", R0Val::Null);
    assert_val("nrow(c(1.0, 2.0))", R0Val::Null);
    assert_val("ncol(c(1.0, 2.0))", R0Val::Null);
    assert_val("nrow(c(\"a\", \"b\"))", R0Val::Null);
    assert_val("ncol(c())", R0Val::Null);
    assert_val("nrow(c())", R0Val::Null);
    assert_val("nrow(list(a = 1:3))", R0Val::Null);
    assert_val("ncol(list(a = 1:3))", R0Val::Null);

    // Argument errors
    assert!(eval_err("nrow()").contains("nrow: missing argument 1"));
    assert!(eval_err("ncol()").contains("ncol: missing argument 1"));
    assert!(eval_err("nrow(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("ncol(1, 2)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g6_coercions() {
    // as.numeric
    assert_val("as.numeric(c(1.5, 2.5))", R0Val::Dbl(vec![1.5, 2.5]));
    assert_val("as.numeric(1:3)", R0Val::Dbl(vec![1.0, 2.0, 3.0]));
    match eval("as.numeric(c(TRUE, FALSE, NA))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], 1.0);
            assert_eq!(v[1], 0.0);
            assert!(is_na_real(v[2]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("as.numeric(c(1L, 2L, NA_integer_))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], 1.0);
            assert_eq!(v[1], 2.0);
            assert!(is_na_real(v[2]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("as.numeric(c(\"1.5\", \"invalid\", NA_character_))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], 1.5);
            assert!(is_na_real(v[1]));
            assert!(is_na_real(v[2]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    assert_val("as.numeric(c())", R0Val::Dbl(vec![]));

    // as.integer
    assert_val("as.integer(c(1.1, -2.9))", R0Val::Int(vec![1, -2]));
    assert_val(
        "as.integer(c(TRUE, FALSE, NA))",
        R0Val::Int(vec![1, 0, NA_INT]),
    );
    assert_val(
        "as.integer(c(\"123\", \"invalid\", NA_character_))",
        R0Val::Int(vec![123, NA_INT, NA_INT]),
    );
    assert_val("as.integer(1:3)", R0Val::Int(vec![1, 2, 3]));
    assert_val("as.integer(c())", R0Val::Int(vec![]));

    // as.logical
    assert_val(
        "as.logical(c(0L, 1L, -5L, NA_integer_))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE, LGL_TRUE, LGL_NA]),
    );
    assert_val(
        "as.logical(c(0.0, 2.5, NA_real_))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE, LGL_NA]),
    );
    assert_val(
        "as.logical(c(\"TRUE\", \"T\", \"true\", \"FALSE\", \"F\", \"false\", \"foo\", NA_character_))",
        R0Val::Lgl(vec![
            LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_NA, LGL_NA,
        ]),
    );
    assert_val(
        "as.logical(0:2)",
        R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE, LGL_TRUE]),
    );
    assert_val("as.logical(c())", R0Val::Lgl(vec![]));

    // Non-atomic error cases
    assert!(eval_err("as.numeric(data.frame(a = 1:2))").starts_with("TypeError:"));
    assert!(eval_err("as.integer(data.frame(a = 1:2))").starts_with("TypeError:"));
    assert!(eval_err("as.logical(data.frame(a = 1:2))").starts_with("TypeError:"));
    assert!(eval_err("as.numeric(list(a = 1:2))").starts_with("TypeError:"));
    assert!(eval_err("as.integer(list(a = 1:2))").starts_with("TypeError:"));
    assert!(eval_err("as.logical(list(a = 1:2))").starts_with("TypeError:"));
}

#[test]
fn conformance_g6_rep_len() {
    // Cyclic repetition and truncation
    assert_val("rep_len(1:3, 5)", R0Val::Int(vec![1, 2, 3, 1, 2]));
    assert_val(
        "rep_len(c(10.0, 20.0), 3)",
        R0Val::Dbl(vec![10.0, 20.0, 10.0]),
    );
    assert_val(
        "rep_len(c(\"a\", \"b\"), 3)",
        R0Val::Chr(vec![Some("a".into()), Some("b".into()), Some("a".into())]),
    );
    assert_val(
        "rep_len(c(TRUE, FALSE), 3)",
        R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE, LGL_TRUE]),
    );
    assert_val("rep_len(1:3, 0)", R0Val::Int(vec![]));
    assert_val("rep_len(c(), 0)", R0Val::Null);

    // NA handling
    assert_val(
        "rep_len(c(1L, NA_integer_), 3)",
        R0Val::Int(vec![1, NA_INT, 1]),
    );

    // Error cases: empty vector with positive length, negative length, NA length
    assert!(eval_err("rep_len(c(), 5)")
        .contains("cannot replicate empty vector with positive length.out"));
    assert!(eval_err("rep_len(1:3, -1)").contains("invalid 'length.out' argument"));
    assert!(eval_err("rep_len(1:3, NA_integer_)").contains("invalid 'length.out' argument"));
    assert!(eval_err("rep_len(1:3)").contains("rep_len: missing argument 2"));
    assert!(eval_err("rep_len(1:3, 2, 3)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g6_rep() {
    // times
    assert_val("rep(1:3, times = 2)", R0Val::Int(vec![1, 2, 3, 1, 2, 3]));
    assert_val("rep(1:3, 2)", R0Val::Int(vec![1, 2, 3, 1, 2, 3]));
    assert_val("rep(1:3, 0)", R0Val::Int(vec![]));

    // each
    assert_val("rep(1:3, each = 2)", R0Val::Int(vec![1, 1, 2, 2, 3, 3]));

    // both times and each
    assert_val(
        "rep(1:2, times = 2, each = 2)",
        R0Val::Int(vec![1, 1, 2, 2, 1, 1, 2, 2]),
    );

    // length.out overrides times
    assert_val("rep(1:3, length.out = 5)", R0Val::Int(vec![1, 2, 3, 1, 2]));
    assert_val(
        "rep(1:3, times = 10, length.out = 4)",
        R0Val::Int(vec![1, 2, 3, 1]),
    );
    assert_val(
        "rep(1:2, each = 3, length.out = 4)",
        R0Val::Int(vec![1, 1, 1, 2]),
    );

    // Empty vector handling
    assert_val("rep(c(), times = 3)", R0Val::Null);
    assert_val("rep(c(), each = 3)", R0Val::Null);

    // NA handling
    assert_val(
        "rep(c(NA_character_, \"x\"), each = 2)",
        R0Val::Chr(vec![None, None, Some("x".into()), Some("x".into())]),
    );

    // Error cases
    assert!(eval_err("rep(c(), length.out = 3)")
        .contains("cannot replicate empty vector with positive length.out"));
    assert!(eval_err("rep(1:3, times = -1)").contains("invalid 'times' argument"));
    assert!(eval_err("rep(1:3, each = -1)").contains("invalid 'each' argument"));
    assert!(eval_err("rep(1:3, length.out = -1)").contains("invalid 'length.out' argument"));
    assert!(eval_err("rep(1:3, times = NA_integer_)").contains("invalid 'times' argument"));
    assert!(eval_err("rep()").contains("rep: missing argument 1"));
}

#[test]
fn conformance_g6_seq() {
    // seq(n)
    assert_val(
        "seq(4)",
        R0Val::Range {
            start: 1,
            end: 4,
            step: 1,
        },
    );
    assert_val(
        "seq(4L)",
        R0Val::Range {
            start: 1,
            end: 4,
            step: 1,
        },
    );
    assert_val(
        "seq(0)",
        R0Val::Range {
            start: 1,
            end: 0,
            step: -1,
        },
    );
    assert_val("seq(c())", R0Val::Int(vec![]));

    // seq(from, to)
    assert_val(
        "seq(2, 5)",
        R0Val::Range {
            start: 2,
            end: 5,
            step: 1,
        },
    );
    assert_val(
        "seq(5, 2)",
        R0Val::Range {
            start: 5,
            end: 2,
            step: -1,
        },
    );
    assert_val(
        "seq(3, 3)",
        R0Val::Range {
            start: 3,
            end: 3,
            step: 1,
        },
    );

    // seq(from, to, by)
    assert_val("seq(1, 5, by = 2)", R0Val::Dbl(vec![1.0, 3.0, 5.0]));
    assert_val("seq(1L, 5L, by = 2L)", R0Val::Int(vec![1, 3, 5]));
    assert_val("seq(5, 1, by = -2)", R0Val::Dbl(vec![5.0, 3.0, 1.0]));

    // seq(from, to, length.out)
    assert_val(
        "seq(1, 10, length.out = 4)",
        R0Val::Dbl(vec![1.0, 4.0, 7.0, 10.0]),
    );
    assert_val(
        "seq(from = 1, length.out = 3)",
        R0Val::Dbl(vec![1.0, 2.0, 3.0]),
    );

    // seq(along.with)
    assert_val(
        "seq(along.with = c(\"a\", \"b\", \"c\"))",
        R0Val::Range {
            start: 1,
            end: 3,
            step: 1,
        },
    );
    assert_val("seq(along.with = c())", R0Val::Int(vec![]));

    // Error cases
    assert!(eval_err("seq(NA_integer_)").contains("seq requires finite non-NA argument"));
    assert!(eval_err("seq(1, 5, by = -1)").contains("wrong sign in 'by' argument"));
    assert!(eval_err("seq(5, 1, by = 1)").contains("wrong sign in 'by' argument"));
    assert!(eval_err("seq(1, 5, by = 0)").contains("invalid '(to - from)/by'"));
    assert!(eval_err("seq(1, 5, length.out = -1)").contains("invalid 'length.out' argument"));
}

#[test]
fn conformance_g6_fuel_exhaustion() {
    let check_fuel_err = |src: &str, budget: i64| {
        let err = R0Vm::new(budget).eval_script(src).unwrap_err();
        assert!(
            err.starts_with("ErrFuelExhausted:"),
            "src: {src}, got: {err}"
        );
    };

    // 1-4. Constructors
    check_fuel_err("numeric(100)", 10);
    check_fuel_err("integer(100L)", 10);
    check_fuel_err("logical(100)", 10);
    check_fuel_err("character(100)", 10);

    // 5-7. Coercions
    check_fuel_err("as.numeric(1:1000)", 10);
    check_fuel_err("as.integer(1:1000)", 10);
    check_fuel_err("as.logical(1:1000)", 10);

    // 8-10. Repetition & sequence
    check_fuel_err("rep_len(1:3, 1000)", 10);
    check_fuel_err("rep(1:3, times = 1000)", 10);
    check_fuel_err("rep(1:3, length.out = 1000)", 10);
    check_fuel_err("seq(1000)", 10);
    check_fuel_err("seq(1, 1000)", 10);
    check_fuel_err("seq(1, 10, length.out = 1000)", 10);

    // 11-12. Shape intrinsics
    let mut vm = R0Vm::new(100);
    vm.eval_script("df <- data.frame(a = 1:5)").unwrap();
    vm.fuel = 1;
    let err = vm.eval_script("nrow(df)").unwrap_err();
    assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

    let mut vm = R0Vm::new(100);
    vm.eval_script("df <- data.frame(a = 1:5)").unwrap();
    vm.fuel = 1;
    let err = vm.eval_script("ncol(df)").unwrap_err();
    assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
}

#[test]
fn conformance_g7_predicates() {
    // is.finite
    assert_val(
        "is.finite(c(1.0, 0.0, -0.0, Inf, -Inf, NaN, NA_real_))",
        R0Val::Lgl(vec![
            LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_FALSE,
        ]),
    );
    assert_val(
        "is.finite(c(1L, 0L, -1L, NA_integer_))",
        R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_FALSE]),
    );
    assert_val(
        "is.finite(c(TRUE, FALSE, NA))",
        R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_FALSE]),
    );
    assert_val(
        "is.finite(c(\"a\", \"b\", NA))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
    );
    assert_val(
        "is.finite(1:3)",
        R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_TRUE]),
    );
    assert_val("is.finite(c())", R0Val::Lgl(vec![]));
    assert_val("is.finite(numeric(0))", R0Val::Lgl(vec![]));
    assert!(eval_err("is.finite(list(a = 1))").contains("Unsupported type"));
    assert!(eval_err("is.finite(data.frame(a = 1))").contains("Unsupported type"));
    assert!(eval_err("is.finite()").contains("is.finite: missing argument 1"));
    assert!(eval_err("is.finite(1, 2)").starts_with("TooManyArguments:"));

    // is.infinite
    assert_val(
        "is.infinite(c(1.0, 0.0, -0.0, Inf, -Inf, NaN, NA_real_))",
        R0Val::Lgl(vec![
            LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE, LGL_FALSE, LGL_FALSE,
        ]),
    );
    assert_val(
        "is.infinite(c(1L, 0L, -1L, NA_integer_))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_FALSE]),
    );
    assert_val(
        "is.infinite(c(TRUE, FALSE, NA))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
    );
    assert_val(
        "is.infinite(c(\"a\", \"b\", NA))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
    );
    assert_val(
        "is.infinite(1:3)",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
    );
    assert_val("is.infinite(c())", R0Val::Lgl(vec![]));
    assert_val("is.infinite(numeric(0))", R0Val::Lgl(vec![]));
    assert!(eval_err("is.infinite(list(a = 1))").contains("Unsupported type"));
    assert!(eval_err("is.infinite(data.frame(a = 1))").contains("Unsupported type"));
    assert!(eval_err("is.infinite()").contains("is.infinite: missing argument 1"));
    assert!(eval_err("is.infinite(1, 2)").starts_with("TooManyArguments:"));

    // anyNA
    assert_val("anyNA(c(1.0, 2.0))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("anyNA(c(1.0, NA_real_))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("anyNA(c(1.0, NaN))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("anyNA(c(1L, 2L))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("anyNA(c(1L, NA_integer_))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("anyNA(c(TRUE, FALSE))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("anyNA(c(TRUE, NA))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("anyNA(c(\"a\", \"b\"))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("anyNA(c(\"a\", NA))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("anyNA(1:10)", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("anyNA(logical(0))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("anyNA(c())", R0Val::Lgl(vec![LGL_FALSE]));
    assert!(eval_err("anyNA(list(a = 1))").contains("Unsupported type"));
    assert!(eval_err("anyNA(data.frame(a = 1))").contains("Unsupported type"));
    assert!(eval_err("anyNA()").contains("anyNA: missing argument 1"));
    assert!(eval_err("anyNA(1, 2)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g7_logical_reductions() {
    // any Kleene 3-valued truth table
    assert_val("any(c(TRUE, TRUE))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("any(c(TRUE, FALSE))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("any(c(TRUE, NA))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("any(c(FALSE, FALSE))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("any(c(FALSE, NA))", R0Val::Lgl(vec![LGL_NA]));
    assert_val("any(c(NA, NA))", R0Val::Lgl(vec![LGL_NA]));
    assert_val("any(logical(0))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("any(c())", R0Val::Lgl(vec![LGL_FALSE]));

    // any with na.rm
    assert_val(
        "any(c(FALSE, NA), na.rm = TRUE)",
        R0Val::Lgl(vec![LGL_FALSE]),
    );
    assert_val("any(c(TRUE, NA), na.rm = TRUE)", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("any(c(NA, NA), na.rm = TRUE)", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("any(c(FALSE, NA), na.rm = FALSE)", R0Val::Lgl(vec![LGL_NA]));

    // all Kleene 3-valued truth table
    assert_val("all(c(TRUE, TRUE))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(c(TRUE, FALSE))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("all(c(TRUE, NA))", R0Val::Lgl(vec![LGL_NA]));
    assert_val("all(c(FALSE, FALSE))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("all(c(FALSE, NA))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("all(c(NA, NA))", R0Val::Lgl(vec![LGL_NA]));
    assert_val("all(logical(0))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(c())", R0Val::Lgl(vec![LGL_TRUE]));

    // all with na.rm
    assert_val(
        "all(c(FALSE, NA), na.rm = TRUE)",
        R0Val::Lgl(vec![LGL_FALSE]),
    );
    assert_val("all(c(TRUE, NA), na.rm = TRUE)", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(c(NA, NA), na.rm = TRUE)", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(c(TRUE, NA), na.rm = FALSE)", R0Val::Lgl(vec![LGL_NA]));

    // Coercion from Int, Range, Dbl
    assert_val("any(c(0, 0))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("any(c(0, 1))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("any(c(0, NA_real_))", R0Val::Lgl(vec![LGL_NA]));
    assert_val("any(c(1, NA_real_))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(c(1, 2))", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(c(1, 0))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("all(c(1, NA_real_))", R0Val::Lgl(vec![LGL_NA]));
    assert_val("all(c(0, NA_real_))", R0Val::Lgl(vec![LGL_FALSE]));
    assert_val("any(1:3)", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(1:3)", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("any(0:2)", R0Val::Lgl(vec![LGL_TRUE]));
    assert_val("all(0:2)", R0Val::Lgl(vec![LGL_FALSE]));

    // Error cases
    assert!(eval_err("any(\"TRUE\")").starts_with("TypeError:"));
    assert!(eval_err("all(\"FALSE\")").starts_with("TypeError:"));
    assert!(eval_err("any(list(a = 1, b = 2))").starts_with("TypeError:"));
    assert!(eval_err("all(data.frame(a = 1))").starts_with("TypeError:"));
    assert!(eval_err("any(c(TRUE, FALSE), na.rm = NA)").starts_with("ErrInvalidCondition:"));
    assert!(eval_err("all(c(TRUE, FALSE), na.rm = \"TRUE\")").starts_with("TypeError:"));
    assert!(eval_err("any()").contains("any: missing argument 1"));
    assert!(eval_err("all()").contains("all: missing argument 1"));
}

#[test]
fn conformance_g7_trig() {
    let pi = std::f64::consts::PI;

    // Values at 0 / 1
    assert_val("sin(0)", R0Val::Dbl(vec![0.0]));
    assert_val("cos(0)", R0Val::Dbl(vec![1.0]));
    assert_val("tan(0)", R0Val::Dbl(vec![0.0]));
    assert_val("asin(0)", R0Val::Dbl(vec![0.0]));
    assert_val("acos(1)", R0Val::Dbl(vec![0.0]));
    assert_val("atan(0)", R0Val::Dbl(vec![0.0]));

    // Non-zero values with float precision
    match eval("sin(c(0.0, 1.5707963267948966))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 2);
            assert!((v[0] - 0.0).abs() < 1e-12);
            assert!((v[1] - 1.0).abs() < 1e-12);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("cos(c(0.0, 3.141592653589793))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 2);
            assert!((v[0] - 1.0).abs() < 1e-12);
            assert!((v[1] - (-1.0)).abs() < 1e-12);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan(c(0.0, 1.0))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 2);
            assert!((v[0] - 0.0).abs() < 1e-12);
            assert!((v[1] - (pi / 4.0)).abs() < 1e-12);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // Domain boundaries producing NaN
    match eval("asin(c(2.0, -2.0))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 2);
            assert!(v[0].is_nan() && !is_na_real(v[0]));
            assert!(v[1].is_nan() && !is_na_real(v[1]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("acos(c(2.0, -2.0))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 2);
            assert!(v[0].is_nan() && !is_na_real(v[0]));
            assert!(v[1].is_nan() && !is_na_real(v[1]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // Int / Range promotion
    assert_val("cos(0L)", R0Val::Dbl(vec![1.0]));
    assert_val("sin(0L:0L)", R0Val::Dbl(vec![0.0]));

    // NA_real_ and NaN preservation across all 6 functions
    for f in ["sin", "cos", "tan", "asin", "acos", "atan"] {
        let code = format!("{f}(c(0.0, NA_real_, NaN))");
        match eval(&code) {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 3, "failed on {f}");
                assert!(is_na_real(v[1]), "NA not preserved in {f}");
                assert!(
                    !is_na_real(v[2]) && v[2].is_nan(),
                    "NaN not preserved in {f}"
                );
            }
            other => panic!("{f}: expected Dbl, got {:?}", other),
        }
    }

    // Empty input
    assert_val("sin(c())", R0Val::Dbl(vec![]));
    assert_val("cos(numeric(0))", R0Val::Dbl(vec![]));

    // Error cases
    assert!(eval_err("sin(\"a\")").starts_with("TypeError:"));
    assert!(eval_err("cos(TRUE)").starts_with("TypeError:"));
    assert!(eval_err("tan(list(a = 1))").starts_with("TypeError:"));
    assert!(eval_err("sin()").contains("sin: missing argument 1"));
    assert!(eval_err("sin(1, 2)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g7_rounding() {
    // trunc values
    assert_val(
        "trunc(c(1.2, 1.9, -1.2, -1.9, 0.0, -0.0))",
        R0Val::Dbl(vec![1.0, 1.0, -1.0, -1.0, 0.0, -0.0]),
    );
    assert_val(
        "trunc(c(Inf, -Inf))",
        R0Val::Dbl(vec![f64::INFINITY, f64::NEG_INFINITY]),
    );
    assert_val("trunc(1:3)", R0Val::Dbl(vec![1.0, 2.0, 3.0]));
    assert_val("trunc(c(1L, -2L))", R0Val::Dbl(vec![1.0, -2.0]));

    // trunc NA / NaN preservation
    match eval("trunc(c(1.5, NA_real_, NaN, NA))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 4);
            assert_eq!(v[0], 1.0);
            assert!(is_na_real(v[1]));
            assert!(!is_na_real(v[2]) && v[2].is_nan());
            assert!(is_na_real(v[3]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    assert_val("trunc(c())", R0Val::Dbl(vec![]));
    assert_val("trunc(numeric(0))", R0Val::Dbl(vec![]));
    assert!(eval_err("trunc(\"abc\")").starts_with("TypeError:"));
    assert!(eval_err("trunc()").contains("trunc: missing argument 1"));
    assert!(eval_err("trunc(1, 2)").starts_with("TooManyArguments:"));

    // round IEEE-754 ties-to-even (banker's rounding) with default digits = 0
    assert_val(
        "round(c(0.5, 1.5, 2.5, 3.5, 4.5))",
        R0Val::Dbl(vec![0.0, 2.0, 2.0, 4.0, 4.0]),
    );
    assert_val(
        "round(c(-0.5, -1.5, -2.5, -3.5, -4.5))",
        R0Val::Dbl(vec![-0.0, -2.0, -2.0, -4.0, -4.0]),
    );

    // Positive digits
    assert_val(
        "round(c(1.2345, 1.2355), 3)",
        R0Val::Dbl(vec![1.234, 1.236]),
    );
    assert_val("round(c(1.25, 1.35), 1)", R0Val::Dbl(vec![1.2, 1.4]));

    // Negative digits
    assert_val(
        "round(c(1234, 1250, 1350), -2)",
        R0Val::Dbl(vec![1200.0, 1200.0, 1400.0]),
    );
    assert_val("round(c(15, 25), -1)", R0Val::Dbl(vec![20.0, 20.0]));

    // Scalar types & named arguments
    assert_val("round(1.5, digits = 0L)", R0Val::Dbl(vec![2.0]));
    assert_val("round(1.5, digits = 0.0)", R0Val::Dbl(vec![2.0]));
    assert_val("round(1.234, digits = 2:2)", R0Val::Dbl(vec![1.23]));
    assert_val("round(digits = 2, x = 1.234)", R0Val::Dbl(vec![1.23]));

    // Preserves NA and NaN
    match eval("round(c(1.5, NA_real_, NaN))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert_eq!(v[0], 2.0);
            assert!(is_na_real(v[1]));
            assert!(!is_na_real(v[2]) && v[2].is_nan());
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("round(c(1L, NA_integer_))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 2);
            assert_eq!(v[0], 1.0);
            assert!(is_na_real(v[1]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // Infinities & empty vectors
    assert_val(
        "round(c(Inf, -Inf))",
        R0Val::Dbl(vec![f64::INFINITY, f64::NEG_INFINITY]),
    );
    assert_val("round(c())", R0Val::Dbl(vec![]));
    assert_val("round(numeric(0))", R0Val::Dbl(vec![]));

    // Digits validation errors
    assert!(eval_err("round(1.5, digits = NA)").contains("round:"));
    assert!(eval_err("round(1.5, digits = NA_integer_)").contains("round:"));
    assert!(eval_err("round(1.5, digits = NA_real_)").contains("round:"));
    assert!(eval_err("round(1.5, digits = NaN)").contains("round:"));
    assert!(eval_err("round(1.5, digits = 1:2)").contains("round:"));
    assert!(eval_err("round(1.5, digits = \"2\")").starts_with("TypeError:"));
    assert!(eval_err("round()").contains("round: missing argument 1"));
}

#[test]
fn conformance_g7_atan2() {
    let pi = std::f64::consts::PI;

    // Quadrants
    match eval("atan2(c(0, 1, 0, -1, 1, -1), c(1, 0, -1, 0, 1, -1))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 6);
            assert!((v[0] - 0.0).abs() < 1e-12);
            assert!((v[1] - (pi / 2.0)).abs() < 1e-12);
            assert!((v[2] - pi).abs() < 1e-12);
            assert!((v[3] - (-pi / 2.0)).abs() < 1e-12);
            assert!((v[4] - (pi / 4.0)).abs() < 1e-12);
            assert!((v[5] - (-3.0 * pi / 4.0)).abs() < 1e-12);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // Named arguments
    match eval("atan2(y = 1, x = 0)") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 1);
            assert!((v[0] - (pi / 2.0)).abs() < 1e-12);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan2(x = 0, y = 1)") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 1);
            assert!((v[0] - (pi / 2.0)).abs() < 1e-12);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // R recycling: length 4 with length 1, length 4 with length 2
    match eval("atan2(c(0, 1, 0, -1), 1)") {
        R0Val::Dbl(v) => assert_eq!(v.len(), 4),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan2(c(0, 1, 0, -1), c(1, 0))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 4);
            assert!((v[0] - 0.0).abs() < 1e-12);
            assert!((v[1] - (pi / 2.0)).abs() < 1e-12);
            assert!((v[2] - 0.0).abs() < 1e-12);
            assert!((v[3] - (-pi / 2.0)).abs() < 1e-12);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // Non-multiple recycling errors
    assert!(eval_err("atan2(c(1, 2), c(1, 2, 3))").starts_with("ErrDimensionMismatch:"));
    assert!(eval_err("atan2(1:3, 1:2)").starts_with("ErrDimensionMismatch:"));

    // Empty inputs
    assert_val("atan2(numeric(0), numeric(0))", R0Val::Dbl(vec![]));
    assert!(eval_err("atan2(c(), 1)").starts_with("TypeError:"));
    assert!(eval_err("atan2(1, c())").starts_with("TypeError:"));
    assert!(eval_err("atan2(c(), c())").starts_with("TypeError:"));

    // Missing argument
    assert!(eval_err("atan2(1)").contains("atan2: missing argument 2"));

    // NA / NaN preservation & B8 priority (first operand wins)
    match eval("atan2(NA_real_, 1)") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan2(1, NA_real_)") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan2(NaN, 1)") {
        R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan2(1, NaN)") {
        R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan2(NA_real_, NaN)") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("atan2(NaN, NA_real_)") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
}

#[test]
fn conformance_g7_statistical_reductions() {
    // prod
    assert_val("prod(c(2, 3, 4))", R0Val::Dbl(vec![24.0]));
    assert_val("prod(numeric(0))", R0Val::Dbl(vec![1.0]));
    assert_val("prod(c())", R0Val::Dbl(vec![1.0]));
    assert_val("prod(1L:4L)", R0Val::Dbl(vec![24.0]));
    assert_val("prod(c(TRUE, TRUE, TRUE))", R0Val::Dbl(vec![1.0]));
    assert_val("prod(c(TRUE, FALSE, TRUE))", R0Val::Dbl(vec![0.0]));
    match eval("prod(c(2, NA_real_, 4))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    assert_val(
        "prod(c(2, NA_real_, 4), na.rm = TRUE)",
        R0Val::Dbl(vec![8.0]),
    );
    assert_val(
        "prod(c(NA_real_, NA_real_), na.rm = TRUE)",
        R0Val::Dbl(vec![1.0]),
    );
    match eval("prod(c(2, NaN, 4))") {
        R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
        other => panic!("expected Dbl, got {:?}", other),
    }

    // var
    assert_val("var(c(1, 2, 3))", R0Val::Dbl(vec![1.0]));
    assert_val("var(c(1, 2, 3, 4, 5))", R0Val::Dbl(vec![2.5]));
    assert_val("var(1L:5L)", R0Val::Dbl(vec![2.5]));
    match eval("var(c(5))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("var(numeric(0))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("var(c(1, NA_real_, 3))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    assert_val(
        "var(c(1, NA_real_, 3), na.rm = TRUE)",
        R0Val::Dbl(vec![2.0]),
    );
    match eval("var(c(1, NA_real_), na.rm = TRUE)") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("var(c(1, NaN, 3))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }

    // sd
    assert_val("sd(c(1, 2, 3))", R0Val::Dbl(vec![1.0]));
    match eval("sd(c(1, 3))") {
        R0Val::Dbl(v) => assert!((v[0] - std::f64::consts::SQRT_2).abs() < 1e-12),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("sd(c(5))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("sd(numeric(0))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("sd(c(1, NA_real_, 3))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("sd(c(1, NA_real_, 3), na.rm = TRUE)") {
        R0Val::Dbl(v) => assert!((v[0] - std::f64::consts::SQRT_2).abs() < 1e-12),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("sd(c(1, NA_real_), na.rm = TRUE)") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("sd(c(1, NaN, 3))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }

    // median
    assert_val("median(c(1, 2, 100))", R0Val::Dbl(vec![2.0]));
    assert_val("median(c(100, 1, 2))", R0Val::Dbl(vec![2.0]));
    assert_val("median(c(1, 2, 3, 4))", R0Val::Dbl(vec![2.5]));
    assert_val("median(c(4, 1, 3, 2))", R0Val::Dbl(vec![2.5]));
    assert_val("median(c(42))", R0Val::Dbl(vec![42.0]));
    match eval("median(numeric(0))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("median(c(1, NA_real_, 3))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    assert_val(
        "median(c(1, NA_real_, 3), na.rm = TRUE)",
        R0Val::Dbl(vec![2.0]),
    );
    assert_val(
        "median(c(1, 2, NA_real_, 10), na.rm = TRUE)",
        R0Val::Dbl(vec![2.0]),
    );
    assert_val(
        "median(c(1, 2, 3, NA_real_, 10), na.rm = TRUE)",
        R0Val::Dbl(vec![2.5]),
    );
    match eval("median(c(NA_real_, NA_real_), na.rm = TRUE)") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("median(c(1, NaN, 3))") {
        R0Val::Dbl(v) => assert!(is_na_real(v[0])),
        other => panic!("expected Dbl, got {:?}", other),
    }

    // Error cases
    assert!(eval_err("prod(1, na.rm = \"foo\")").starts_with("TypeError:"));
    assert!(eval_err("var(1, na.rm = NA)").starts_with("ErrInvalidCondition:"));
    assert!(eval_err("sd(1, na.rm = 1)").starts_with("TypeError:"));
    assert!(eval_err("median(1, unknown = 1)").starts_with("UnknownArgument:"));
    assert!(eval_err("prod()").contains("prod: missing argument 1"));
    assert!(eval_err("var()").contains("var: missing argument 1"));
    assert!(eval_err("sd()").contains("sd: missing argument 1"));
    assert!(eval_err("median()").contains("median: missing argument 1"));
}

#[test]
fn conformance_g7_cumulative_reductions() {
    // cumprod
    assert_val(
        "cumprod(c(1, 2, 3, 4))",
        R0Val::Dbl(vec![1.0, 2.0, 6.0, 24.0]),
    );
    assert_val("cumprod(numeric(0))", R0Val::Dbl(vec![]));
    assert_val("cumprod(c())", R0Val::Dbl(vec![]));
    match eval("cumprod(c(2, 3, NA_real_, 4, 5))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 5);
            assert_eq!(v[0], 2.0);
            assert_eq!(v[1], 6.0);
            assert!(is_na_real(v[2]));
            assert!(is_na_real(v[3]));
            assert!(is_na_real(v[4]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("cumprod(c(2, 3, NaN, 4, 5))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 5);
            assert_eq!(v[0], 2.0);
            assert_eq!(v[1], 6.0);
            assert!(!is_na_real(v[2]) && v[2].is_nan());
            assert!(!is_na_real(v[3]) && v[3].is_nan());
            assert!(!is_na_real(v[4]) && v[4].is_nan());
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("cumprod(c(NA_real_, 1, 2))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert!(is_na_real(v[0]) && is_na_real(v[1]) && is_na_real(v[2]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // cummin
    assert_val(
        "cummin(c(3, 1, 4, 1, 5, 9))",
        R0Val::Dbl(vec![3.0, 1.0, 1.0, 1.0, 1.0, 1.0]),
    );
    assert_val("cummin(numeric(0))", R0Val::Dbl(vec![]));
    assert_val("cummin(c())", R0Val::Dbl(vec![]));
    match eval("cummin(c(3, 1, NA_real_, 0))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 4);
            assert_eq!(v[0], 3.0);
            assert_eq!(v[1], 1.0);
            assert!(is_na_real(v[2]));
            assert!(is_na_real(v[3]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("cummin(c(3, 1, NaN, 0))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 4);
            assert_eq!(v[0], 3.0);
            assert_eq!(v[1], 1.0);
            assert!(!is_na_real(v[2]) && v[2].is_nan());
            assert!(!is_na_real(v[3]) && v[3].is_nan());
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("cummin(c(NA_real_, 1, 2))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert!(is_na_real(v[0]) && is_na_real(v[1]) && is_na_real(v[2]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // cummax
    assert_val(
        "cummax(c(1, 3, 2, 5, 4))",
        R0Val::Dbl(vec![1.0, 3.0, 3.0, 5.0, 5.0]),
    );
    assert_val("cummax(numeric(0))", R0Val::Dbl(vec![]));
    assert_val("cummax(c())", R0Val::Dbl(vec![]));
    match eval("cummax(c(1, 3, NA_real_, 10))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 4);
            assert_eq!(v[0], 1.0);
            assert_eq!(v[1], 3.0);
            assert!(is_na_real(v[2]));
            assert!(is_na_real(v[3]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("cummax(c(1, 3, NaN, 10))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 4);
            assert_eq!(v[0], 1.0);
            assert_eq!(v[1], 3.0);
            assert!(!is_na_real(v[2]) && v[2].is_nan());
            assert!(!is_na_real(v[3]) && v[3].is_nan());
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("cummax(c(NA_real_, 1, 2))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert!(is_na_real(v[0]) && is_na_real(v[1]) && is_na_real(v[2]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // Argument count & unknown argument errors
    assert!(eval_err("cumprod(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("cummin(1, na.rm = TRUE)").starts_with("UnknownArgument:"));
    assert!(eval_err("cummax(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("cumprod()").contains("cumprod: missing argument 1"));
    assert!(eval_err("cummin()").contains("cummin: missing argument 1"));
    assert!(eval_err("cummax()").contains("cummax: missing argument 1"));
}

#[test]
fn conformance_g7_fuel_exhaustion_and_locking() {
    let check_fuel_err = |src: &str, budget: i64| {
        let err = R0Vm::new(budget).eval_script(src).unwrap_err();
        assert!(
            err.starts_with("ErrFuelExhausted:"),
            "src: {src}, got: {err}"
        );
    };

    // Predicates & logical reductions fuel exhaustion
    check_fuel_err("is.finite(1:2000000000)", 100_000);
    check_fuel_err("is.infinite(1:2000000000)", 100_000);
    check_fuel_err("any(1:2000000000)", 100_000);
    check_fuel_err("all(1:2000000000)", 100_000);
    check_fuel_err("anyNA(1:2000000000)", 100_000);

    // Trig and rounding fuel exhaustion
    check_fuel_err("sin(1:2000000000)", 100_000);
    check_fuel_err("cos(1:2000000000)", 100_000);
    check_fuel_err("tan(1:2000000000)", 100_000);
    check_fuel_err("acos(1:2000000000)", 100_000);
    check_fuel_err("asin(1:2000000000)", 100_000);
    check_fuel_err("atan(1:2000000000)", 100_000);
    check_fuel_err("trunc(1:2000000000)", 100_000);
    check_fuel_err("round(1:2000000000)", 100_000);
    check_fuel_err("atan2(1:2000000000, 1.0)", 100_000);
    check_fuel_err("atan2(1.0, 1:2000000000)", 100_000);

    // Statistical & cumulative reductions fuel exhaustion
    check_fuel_err("prod(1:2000000000)", 100_000);
    check_fuel_err("var(1:2000000000)", 100_000);
    check_fuel_err("sd(1:2000000000)", 100_000);
    check_fuel_err("median(1:2000000000)", 100_000);
    check_fuel_err("cumprod(1:2000000000)", 100_000);
    check_fuel_err("cummin(1:2000000000)", 100_000);
    check_fuel_err("cummax(1:2000000000)", 100_000);

    // Shadowing prevention: all 21 G7 intrinsics cannot be shadowed
    let g7_intrinsics = [
        "is.finite",
        "is.infinite",
        "any",
        "all",
        "anyNA",
        "sin",
        "cos",
        "tan",
        "acos",
        "asin",
        "atan",
        "trunc",
        "round",
        "atan2",
        "prod",
        "var",
        "sd",
        "median",
        "cumprod",
        "cummin",
        "cummax",
    ];
    for func in g7_intrinsics {
        let code = format!("{func} <- function(x) x");
        let err = eval_err(&code);
        assert!(
            err.contains("Cannot shadow built-in intrinsic"),
            "{func} was not locked against shadowing: {err}"
        );
    }
}

#[test]
fn conformance_g8_search() {
    // which.min & which.max on integer, numeric, logical, range
    assert_val("which.min(c(5L, 3L, 1L, 1L, 4L))", R0Val::Int(vec![3]));
    assert_val("which.max(c(5L, 3L, 5L, 1L, 4L))", R0Val::Int(vec![1]));
    assert_val("which.min(c(5.5, 3.2, 1.1, 1.1, 4.4))", R0Val::Int(vec![3]));
    assert_val("which.max(c(5.5, 3.2, 5.5, 1.1, 4.4))", R0Val::Int(vec![1]));
    assert_val("which.min(c(TRUE, FALSE, TRUE))", R0Val::Int(vec![2]));
    assert_val("which.max(c(FALSE, TRUE, FALSE))", R0Val::Int(vec![2]));
    assert_val("which.min(1:10)", R0Val::Int(vec![1]));
    assert_val("which.max(1:10)", R0Val::Int(vec![10]));
    assert_val("which.min(10:1)", R0Val::Int(vec![10]));
    assert_val("which.max(10:1)", R0Val::Int(vec![1]));

    // Inf, -Inf
    assert_val("which.min(c(Inf, -Inf, 0))", R0Val::Int(vec![2]));
    assert_val("which.max(c(Inf, -Inf, 0))", R0Val::Int(vec![1]));

    // NA and NaN handling: skipped when finding min/max
    assert_val("which.min(c(NA, 5, 2, NA, 2))", R0Val::Int(vec![3]));
    assert_val("which.max(c(NA, 1, 5, 5, 2))", R0Val::Int(vec![3]));
    assert_val("which.min(c(NaN, 10, -5))", R0Val::Int(vec![3]));
    assert_val("which.max(c(NaN, 10, -5))", R0Val::Int(vec![2]));

    // All NA / NaN or empty: return integer(0)
    assert_val("which.min(c(NA, NaN, NA_real_))", R0Val::Int(vec![]));
    assert_val("which.max(c(NA, NaN, NA_real_))", R0Val::Int(vec![]));
    assert_val("which.min(numeric(0))", R0Val::Int(vec![]));
    assert_val("which.max(integer(0))", R0Val::Int(vec![]));
    assert_val("which.min(c())", R0Val::Int(vec![]));
    assert_val("which.max(c())", R0Val::Int(vec![]));

    // Errors & argument validation
    assert!(eval_err("which.min(c(\"a\", \"b\"))").starts_with("TypeError:"));
    assert!(eval_err("which.max(c(\"a\", \"b\"))").starts_with("TypeError:"));
    assert!(eval_err("which.min(list(a = 1))").starts_with("TypeError:"));
    assert!(eval_err("which.max(list(a = 1))").starts_with("TypeError:"));
    assert!(eval_err("which.min()").contains("which.min: missing argument 1"));
    assert!(eval_err("which.max()").contains("which.max: missing argument 1"));
    assert!(eval_err("which.min(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("which.max(1, 2)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g8_match() {
    // Basic integer, double, character, range
    assert_val(
        "match(c(1, 2, 3), c(2, 3, 4))",
        R0Val::Int(vec![NA_INT, 1, 2]),
    );
    assert_val(
        "match(c(1L, 2L, 3L), c(2L, 3L, 4L))",
        R0Val::Int(vec![NA_INT, 1, 2]),
    );
    assert_val("match(1:3, 2:4)", R0Val::Int(vec![NA_INT, 1, 2]));
    assert_val(
        "match(c(\"b\", \"x\"), c(\"a\", \"b\", \"c\"))",
        R0Val::Int(vec![2, NA_INT]),
    );
    assert_val(
        "match(c(TRUE, FALSE), c(FALSE, TRUE))",
        R0Val::Int(vec![2, 1]),
    );

    // Multiple matches in table: return first match position
    assert_val("match(c(1, 4), c(1, 2, 1))", R0Val::Int(vec![1, NA_INT]));

    // Custom nomatch argument (integer, double, NA)
    assert_val(
        "match(c(1, 2, 3), c(2, 3, 4), nomatch = 0L)",
        R0Val::Int(vec![0, 1, 2]),
    );
    assert_val(
        "match(c(1, 2, 3), c(2, 3, 4), nomatch = 0)",
        R0Val::Int(vec![0, 1, 2]),
    );
    assert_val(
        "match(c(1, 2, 3), c(2, 3, 4), 99L)",
        R0Val::Int(vec![99, 1, 2]),
    );

    // NA and NaN handling: NA matches NA, NaN matches NaN
    assert_val(
        "match(c(\"a\", NA), c(\"b\", NA))",
        R0Val::Int(vec![NA_INT, 2]),
    );
    assert_val("match(c(NA, NaN), c(1, NA))", R0Val::Int(vec![2, NA_INT]));
    assert_val("match(c(NA, NaN), c(NaN, 1))", R0Val::Int(vec![NA_INT, 1]));

    // Empty vectors & c() / NULL
    assert_val("match(numeric(0), c(1, 2))", R0Val::Int(vec![]));
    assert_val(
        "match(c(1, 2), numeric(0))",
        R0Val::Int(vec![NA_INT, NA_INT]),
    );
    assert_val("match(c(), c(1, 2))", R0Val::Int(vec![]));
    assert_val("match(c(1, 2), c())", R0Val::Int(vec![NA_INT, NA_INT]));
    assert_val("match(c(\"a\"), c())", R0Val::Int(vec![NA_INT]));

    // Error and argument validation
    assert!(eval_err("match(\"a\", 1)").starts_with("TypeError:"));
    assert!(eval_err("match(1, \"a\")").starts_with("TypeError:"));
    assert!(eval_err("match(1)").contains("missing argument 2"));
    assert!(eval_err("match(1, 2, nomatch = \"foo\")").starts_with("TypeError:"));
    assert!(eval_err("match(1, 2, 3, 4)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g8_deduplication() {
    // unique and duplicated on integers
    assert_val("unique(c(1L, 2L, 1L, 3L, 2L))", R0Val::Int(vec![1, 2, 3]));
    assert_val(
        "duplicated(c(1L, 2L, 1L, 3L, 2L))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_FALSE, LGL_TRUE]),
    );

    // Integers with NA
    assert_val(
        "unique(c(1L, NA_integer_, 2L, NA_integer_))",
        R0Val::Int(vec![1, NA_INT, 2]),
    );
    assert_val(
        "duplicated(c(1L, NA_integer_, 2L, NA_integer_))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_TRUE]),
    );

    // Logical
    assert_val(
        "unique(c(TRUE, FALSE, NA, TRUE, FALSE, NA))",
        R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE, LGL_NA]),
    );
    assert_val(
        "duplicated(c(TRUE, FALSE, NA, TRUE, FALSE, NA))",
        R0Val::Lgl(vec![
            LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE, LGL_TRUE,
        ]),
    );

    // Characters
    assert_val(
        "unique(c(\"a\", \"b\", \"a\", NA, \"b\", NA))",
        R0Val::Chr(vec![Some("a".into()), Some("b".into()), None]),
    );
    assert_val(
        "duplicated(c(\"a\", \"b\", \"a\", NA, \"b\", NA))",
        R0Val::Lgl(vec![
            LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_FALSE, LGL_TRUE, LGL_TRUE,
        ]),
    );

    // Doubles: NA vs NaN are distinct, -0.0 vs 0.0 are equal
    match eval("unique(c(1.5, 2.5, 1.5, NaN, NA_real_, NaN, NA_real_))") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 4);
            assert_eq!(v[0], 1.5);
            assert_eq!(v[1], 2.5);
            assert!(v[2].is_nan() && !is_na_real(v[2]));
            assert!(is_na_real(v[3]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    assert_val(
        "duplicated(c(1.5, 2.5, 1.5, NaN, NA_real_, NaN, NA_real_))",
        R0Val::Lgl(vec![
            LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE,
        ]),
    );
    assert_val("unique(c(0.0, -0.0))", R0Val::Dbl(vec![0.0]));
    assert_val(
        "duplicated(c(0.0, -0.0))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE]),
    );

    // Range
    assert_val(
        "unique(1:5)",
        R0Val::Range {
            start: 1,
            end: 5,
            step: 1,
        },
    );
    assert_val("duplicated(1:5)", R0Val::Lgl(vec![LGL_FALSE; 5]));

    // Empty and c()
    assert_val("unique(c())", R0Val::Null);
    assert_val("duplicated(c())", R0Val::Lgl(vec![]));

    // Error and arg validation
    assert!(eval_err("unique(list(a = 1))").contains("Unsupported type"));
    assert!(eval_err("duplicated(list(a = 1))").contains("Unsupported type"));
    assert!(eval_err("unique()").contains("unique: missing argument 1"));
    assert!(eval_err("duplicated()").contains("duplicated: missing argument 1"));
    assert!(eval_err("unique(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("duplicated(1, 2)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g8_sorting_and_ranking() {
    // sort: integers, doubles, logical, character, range
    assert_val("sort(c(3L, 1L, 4L, 2L))", R0Val::Int(vec![1, 2, 3, 4]));
    assert_val(
        "sort(c(3L, 1L, 4L, 2L), decreasing = TRUE)",
        R0Val::Int(vec![4, 3, 2, 1]),
    );
    assert_val(
        "sort(c(3L, 1L, 4L, 2L), TRUE)",
        R0Val::Int(vec![4, 3, 2, 1]),
    );
    assert_val(
        "sort(c(3.5, 1.2, 4.8, 2.1))",
        R0Val::Dbl(vec![1.2, 2.1, 3.5, 4.8]),
    );
    assert_val(
        "sort(c(3.5, 1.2, 4.8, 2.1), decreasing = TRUE)",
        R0Val::Dbl(vec![4.8, 3.5, 2.1, 1.2]),
    );
    assert_val(
        "sort(c(TRUE, FALSE, TRUE, NA, FALSE))",
        R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE]),
    );
    assert_val(
        "sort(c(\"banana\", \"apple\", \"cherry\", NA))",
        R0Val::Chr(vec![
            Some("apple".into()),
            Some("banana".into()),
            Some("cherry".into()),
        ]),
    );
    assert_val(
        "sort(1:5)",
        R0Val::Range {
            start: 1,
            end: 5,
            step: 1,
        },
    );
    assert_val(
        "sort(1:5, decreasing = TRUE)",
        R0Val::Range {
            start: 5,
            end: 1,
            step: -1,
        },
    );
    assert_val(
        "sort(5:1)",
        R0Val::Range {
            start: 1,
            end: 5,
            step: 1,
        },
    );

    // sort drops NAs/NaNs by default
    assert_val(
        "sort(c(3L, NA_integer_, 1L, NA_integer_, 2L))",
        R0Val::Int(vec![1, 2, 3]),
    );
    assert_val(
        "sort(c(3.0, NA_real_, 1.0, NaN, 2.0))",
        R0Val::Dbl(vec![1.0, 2.0, 3.0]),
    );
    assert_val("sort(c(NA, NA))", R0Val::Lgl(vec![]));
    assert_val("sort(c(NA_real_, NaN))", R0Val::Dbl(vec![]));
    assert_val("sort(numeric(0))", R0Val::Dbl(vec![]));
    assert_val("sort(c())", R0Val::Null);

    // order: stable sort, tie-breaking preserves original index order
    assert_val("order(c(30L, 10L, 20L))", R0Val::Int(vec![2, 3, 1]));
    assert_val(
        "order(c(30L, 10L, 20L), decreasing = TRUE)",
        R0Val::Int(vec![1, 3, 2]),
    );
    assert_val("order(c(2, 1, 2, 1))", R0Val::Int(vec![2, 4, 1, 3]));
    assert_val(
        "order(c(2, 1, 2, 1), decreasing = TRUE)",
        R0Val::Int(vec![1, 3, 2, 4]),
    );
    assert_val("order(1:5)", R0Val::Int(vec![1, 2, 3, 4, 5]));
    assert_val(
        "order(1:5, decreasing = TRUE)",
        R0Val::Int(vec![5, 4, 3, 2, 1]),
    );

    // order: na.last = TRUE puts NAs/NaNs last in original order even when decreasing = TRUE
    assert_val("order(c(1, NA, 3, NA, 2))", R0Val::Int(vec![1, 5, 3, 2, 4]));
    assert_val(
        "order(c(1, NA, 3, NA, 2), decreasing = TRUE)",
        R0Val::Int(vec![3, 5, 1, 2, 4]),
    );
    assert_val("order(c(NaN, 1, NA, 2))", R0Val::Int(vec![2, 4, 1, 3]));
    assert_val(
        "order(c(NaN, 1, NA, 2), decreasing = TRUE)",
        R0Val::Int(vec![4, 2, 1, 3]),
    );
    assert_val(
        "order(c(\"b\", \"a\", NA, \"c\"))",
        R0Val::Int(vec![2, 1, 4, 3]),
    );
    assert_val(
        "order(c(\"b\", \"a\", NA, \"c\"), decreasing = TRUE)",
        R0Val::Int(vec![4, 1, 2, 3]),
    );
    assert_val("order(c(NA, NA))", R0Val::Int(vec![1, 2]));
    assert_val("order(numeric(0))", R0Val::Int(vec![]));
    assert_val("order(c())", R0Val::Int(vec![]));

    // rank: average ties method
    assert_val("rank(c(30, 10, 20))", R0Val::Dbl(vec![3.0, 1.0, 2.0]));
    assert_val(
        "rank(c(3, 1, 4, 1, 5))",
        R0Val::Dbl(vec![3.0, 1.5, 4.0, 1.5, 5.0]),
    );
    assert_val(
        "rank(c(10, 20, 20, 20, 30))",
        R0Val::Dbl(vec![1.0, 3.0, 3.0, 3.0, 5.0]),
    );
    assert_val("rank(1:5)", R0Val::Dbl(vec![1.0, 2.0, 3.0, 4.0, 5.0]));
    assert_val(
        "rank(c(\"b\", \"a\", \"b\"))",
        R0Val::Dbl(vec![2.5, 1.0, 2.5]),
    );
    assert_val(
        "rank(c(TRUE, FALSE, TRUE))",
        R0Val::Dbl(vec![2.5, 1.0, 2.5]),
    );

    // rank: NAs receive distinct ranks consecutively at the end
    assert_val(
        "rank(c(10, NA, 20, NA, 10))",
        R0Val::Dbl(vec![1.5, 4.0, 3.0, 5.0, 1.5]),
    );
    assert_val(
        "rank(c(NaN, 1, NA, 2))",
        R0Val::Dbl(vec![3.0, 1.0, 4.0, 2.0]),
    );
    assert_val("rank(c(NA, NA))", R0Val::Dbl(vec![1.0, 2.0]));
    assert_val("rank(numeric(0))", R0Val::Dbl(vec![]));
    assert_val("rank(c())", R0Val::Dbl(vec![]));

    // Errors and arg validation
    assert!(eval_err("sort(1, 2, 3)").starts_with("TooManyArguments:"));
    assert!(eval_err("order(1, 2, 3)").starts_with("TooManyArguments:"));
    assert!(eval_err("rank(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("sort(1, decreasing = \"foo\")").starts_with("TypeError:"));
    assert!(eval_err("sort(1, decreasing = NA)").starts_with("ErrInvalidCondition:"));
    assert!(eval_err("order(1, decreasing = 1)").starts_with("TypeError:"));
    assert!(eval_err("sort(list(a = 1))").starts_with("TypeError:"));
    assert!(eval_err("order(list(a = 1))").starts_with("TypeError:"));
    assert!(eval_err("rank(list(a = 1))").starts_with("TypeError:"));
}

#[test]
fn conformance_g8_binning() {
    // tabulate: default nbins = max(1, bin)
    assert_val("tabulate(c(1, 2, 2, 5))", R0Val::Int(vec![1, 2, 0, 0, 1]));
    assert_val(
        "tabulate(c(1L, 2L, 2L, 5L))",
        R0Val::Int(vec![1, 2, 0, 0, 1]),
    );
    assert_val("tabulate(1:5)", R0Val::Int(vec![1, 1, 1, 1, 1]));
    assert_val("tabulate(c(1.2, 2.8))", R0Val::Int(vec![1, 1]));

    // tabulate: custom nbins
    assert_val(
        "tabulate(c(1, 2, 2, 5), nbins = 3)",
        R0Val::Int(vec![1, 2, 0]),
    );
    assert_val("tabulate(c(1, 2, 2, 5), 3)", R0Val::Int(vec![1, 2, 0]));
    assert_val(
        "tabulate(c(1, 2), nbins = 5)",
        R0Val::Int(vec![1, 1, 0, 0, 0]),
    );
    assert_val("tabulate(c(1, 2), nbins = 0)", R0Val::Int(vec![]));

    // tabulate: values <= 0 and NA are ignored; default nbins is max(1, valid)
    assert_val("tabulate(c(-1, 0, NA, 2, 3))", R0Val::Int(vec![0, 1, 1]));
    assert_val("tabulate(c(-1, 0, NA))", R0Val::Int(vec![0]));
    assert_val("tabulate(c(0, 0))", R0Val::Int(vec![0]));
    assert_val("tabulate(integer(0))", R0Val::Int(vec![0]));
    assert_val("tabulate(c())", R0Val::Int(vec![0]));
    assert_val("tabulate(c(), nbins = 3)", R0Val::Int(vec![0, 0, 0]));

    // tabulate errors
    assert!(eval_err("tabulate(1, nbins = -1)").contains("invalid 'nbins' argument"));
    assert!(eval_err("tabulate(1, nbins = NA)").contains("invalid 'nbins' argument"));
    assert!(eval_err("tabulate(1, nbins = c(1, 2))").contains("invalid 'nbins' argument"));
    assert!(eval_err("tabulate(c(\"a\", \"b\"))").starts_with("TypeError:"));
    assert!(eval_err("tabulate(c(TRUE, FALSE))").starts_with("TypeError:"));
    assert!(eval_err("tabulate(1, 2, 3)").starts_with("TooManyArguments:"));

    // findInterval: standard lookups
    assert_val(
        "findInterval(c(0, 1, 2.5, 3, 5, 10), c(1, 2, 3, 5))",
        R0Val::Int(vec![0, 1, 2, 3, 4, 4]),
    );
    assert_val(
        "findInterval(c(1, 2, 2.5, 5), c(1, 2, 2, 5))",
        R0Val::Int(vec![1, 3, 3, 4]),
    );
    assert_val(
        "findInterval(c(0, 1, 2, 3), c(2))",
        R0Val::Int(vec![0, 0, 1, 1]),
    );
    assert_val(
        "findInterval(1:5, c(2, 4))",
        R0Val::Int(vec![0, 1, 1, 2, 2]),
    );
    assert_val(
        "findInterval(c(TRUE, FALSE), c(0, 1))",
        R0Val::Int(vec![2, 1]),
    );

    // findInterval: empty and NA handling
    assert_val("findInterval(c(1, 2), c())", R0Val::Int(vec![0, 0]));
    assert_val("findInterval(c(), c(1, 2, 3))", R0Val::Int(vec![]));
    assert_val(
        "findInterval(c(NA, 1, 2, NA), c(1, 2, 3))",
        R0Val::Int(vec![NA_INT, 1, 2, NA_INT]),
    );
    assert_val("findInterval(c(NA, 1), c())", R0Val::Int(vec![NA_INT, 0]));

    // findInterval errors: vec must be sorted and non-NA
    let err = eval_err("findInterval(c(1, 2), c(3, 1))");
    assert!(err.contains("'vec' must be sorted non-decreasingly and not contain NAs"));
    let err = eval_err("findInterval(c(1, 2), c(1, NA, 3))");
    assert!(err.contains("'vec' must be sorted non-decreasingly and not contain NAs"));
    assert!(eval_err("findInterval(c(\"a\"), c(1, 2))").starts_with("TypeError:"));
    assert!(eval_err("findInterval(c(1, 2), c(\"a\"))").starts_with("TypeError:"));
    assert!(eval_err("findInterval(1)").contains("missing argument 2"));
    assert!(eval_err("findInterval(1, 2, 3)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g8_extensions() {
    // approx rule = 1 (default: NA outside range)
    match eval("approx(c(1, 3), c(10, 30), c(0, 1, 2, 3, 4))$y") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 5);
            assert!(is_na_real(v[0]));
            assert_eq!(v[1], 10.0);
            assert_eq!(v[2], 20.0);
            assert_eq!(v[3], 30.0);
            assert!(is_na_real(v[4]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // approx rule = 2 (constant extrapolation to boundary values)
    assert_val(
        "approx(c(1, 3), c(10, 30), c(0, 1, 2, 3, 4), rule = 2)$y",
        R0Val::Dbl(vec![10.0, 10.0, 20.0, 30.0, 30.0]),
    );
    assert_val(
        "approx(c(1, 3), c(10, 30), c(0, 1, 2, 3, 4), 2)$y",
        R0Val::Dbl(vec![10.0, 10.0, 20.0, 30.0, 30.0]),
    );

    // approx rule = c(left, right) asymmetric rules
    match eval("approx(c(1, 3), c(10, 30), c(0, 1, 2, 3, 4), rule = c(1, 2))$y") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 5);
            assert!(is_na_real(v[0]));
            assert_eq!(v[1], 10.0);
            assert_eq!(v[2], 20.0);
            assert_eq!(v[3], 30.0);
            assert_eq!(v[4], 30.0);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    match eval("approx(c(1, 3), c(10, 30), c(0, 1, 2, 3, 4), rule = c(2, 1))$y") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 5);
            assert_eq!(v[0], 10.0);
            assert_eq!(v[1], 10.0);
            assert_eq!(v[2], 20.0);
            assert_eq!(v[3], 30.0);
            assert!(is_na_real(v[4]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // approx rule with NA in xout
    match eval("approx(c(1, 3), c(10, 30), c(NA, 0, 2), rule = 2)$y") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 3);
            assert!(is_na_real(v[0]));
            assert_eq!(v[1], 10.0);
            assert_eq!(v[2], 20.0);
        }
        other => panic!("expected Dbl, got {:?}", other),
    }

    // approx errors
    assert!(eval_err("approx(c(1, 2), c(10, 20), 0, rule = 0)").contains("values must be 1 or 2"));
    assert!(eval_err("approx(c(1, 2), c(10, 20), 0, rule = 3)").contains("values must be 1 or 2"));
    assert!(eval_err("approx(c(1, 2), c(10, 20), 0, rule = c(1, 2, 3))").contains("length 1 or 2"));
    assert!(eval_err("approx(c(1, 2), c(10, 20), 0, rule = \"a\")").starts_with("TypeError:"));
    assert!(eval_err("approx(c(1, 2), c(10, 20), 0, 1, 2)").starts_with("TooManyArguments:"));

    // diff: default lag = 1
    assert_val("diff(c(1, 4, 9))", R0Val::Dbl(vec![3.0, 5.0]));
    assert_val("diff(c(1, 4, 9), lag = 1)", R0Val::Dbl(vec![3.0, 5.0]));
    assert_val("diff(c(1, 4, 9), lag = 2)", R0Val::Dbl(vec![8.0]));
    assert_val("diff(c(1, 4, 9), 2)", R0Val::Dbl(vec![8.0]));
    assert_val("diff(c(1, 4, 9), lag = 3)", R0Val::Dbl(vec![]));
    assert_val("diff(c(1, 4, 9), lag = 5)", R0Val::Dbl(vec![]));
    assert_val("diff(c(1, 4))", R0Val::Dbl(vec![3.0]));
    assert_val("diff(c(1))", R0Val::Dbl(vec![]));
    assert_val("diff(numeric(0))", R0Val::Dbl(vec![]));
    assert_val("diff(c())", R0Val::Dbl(vec![]));
    assert_val("diff(1:5, lag = 2)", R0Val::Int(vec![2, 2, 2]));
    assert_val("diff(c(1L, 4L, 9L))", R0Val::Int(vec![3, 5]));

    // diff with NAs
    match eval("diff(c(1, NA_real_, 9), lag = 1)") {
        R0Val::Dbl(v) => {
            assert_eq!(v.len(), 2);
            assert!(is_na_real(v[0]));
            assert!(is_na_real(v[1]));
        }
        other => panic!("expected Dbl, got {:?}", other),
    }
    assert_val("diff(c(1, NA_real_, 9), lag = 2)", R0Val::Dbl(vec![8.0]));

    // diff errors
    assert!(eval_err("diff(c(1, 2), lag = 0)").contains("must be an integer >= 1"));
    assert!(eval_err("diff(c(1, 2), lag = -1)").contains("must be an integer >= 1"));
    assert!(eval_err("diff(c(1, 2), lag = NA)").contains("must be an integer >= 1"));
    assert!(eval_err("diff(c(1, 2), lag = c(1, 2))").contains("must be an integer >= 1"));
    assert!(eval_err("diff(c(1, 2), lag = \"a\")").contains("must be an integer >= 1"));
    assert!(eval_err("diff(c(1, 2), 1, 2)").starts_with("TooManyArguments:"));
}

#[test]
fn conformance_g8_fuel_exhaustion_and_locking() {
    let check_fuel_err = |src: &str, budget: i64| {
        let err = R0Vm::new(budget).eval_script(src).unwrap_err();
        assert!(
            err.starts_with("ErrFuelExhausted:"),
            "src: {src}, got: {err}"
        );
    };

    // Fuel exhaustion for all 10 G8 intrinsics
    check_fuel_err("which.min(1:2000000000)", 100_000);
    check_fuel_err("which.max(1:2000000000)", 100_000);
    check_fuel_err("unique(1:2000000000)", 100_000);
    check_fuel_err("duplicated(1:2000000000)", 100_000);
    check_fuel_err("match(1:100000, 1:100000)", 100_000);
    check_fuel_err("sort(1:2000000000)", 100_000);
    check_fuel_err("order(1:2000000000)", 100_000);
    check_fuel_err("rank(1:2000000000)", 100_000);
    check_fuel_err("tabulate(1:2000000000)", 100_000);
    check_fuel_err("tabulate(1:5, nbins = 2000000000)", 100_000);
    check_fuel_err("findInterval(1:100000, 1:100000)", 100_000);

    // Shadowing prevention: all 10 G8 intrinsics cannot be shadowed
    let g8_intrinsics = [
        "which.min",
        "which.max",
        "match",
        "unique",
        "duplicated",
        "sort",
        "order",
        "rank",
        "tabulate",
        "findInterval",
    ];
    for func in g8_intrinsics {
        let code = format!("{func} <- function(x) x");
        let err = eval_err(&code);
        assert!(
            err.contains("Cannot shadow built-in intrinsic"),
            "{func} was not locked against shadowing: {err}"
        );
    }
}

const G9_DF: &str = "d <- data.frame(a = c(1, 2, 3), b = c(4, 5, 6), c = c(7, 8, 9))\n";

fn g9(s: &str) -> R0Val {
    eval(&format!("{G9_DF}{s}"))
}

fn g9_err(s: &str) -> String {
    eval_err(&format!("{G9_DF}{s}"))
}

#[test]
fn conformance_g9_break_next() {
    assert_val(
        "s <- 0\nfor (i in 1:10) { if (i > 3) break\n s <- s + i }\ns",
        R0Val::Dbl(vec![6.0]),
    );
    assert_val(
        "s <- 0\nfor (i in 1:5) { if (i == 2) next\n s <- s + i }\ns",
        R0Val::Dbl(vec![13.0]),
    );
    assert_val(
        "i <- 0\nwhile (TRUE) { i <- i + 1\n if (i >= 4) break }\ni",
        R0Val::Dbl(vec![4.0]),
    );
    assert_val(
        "i <- 0\ns <- 0\nwhile (i < 5) { i <- i + 1\n if (i == 3) next\n s <- s + i }\ns",
        R0Val::Dbl(vec![12.0]),
    );
    // nested: break only exits the inner loop
    assert_val(
        "n <- 0\nfor (i in 1:3) { for (j in 1:5) { if (j > 2) break\n n <- n + 1 } }\nn",
        R0Val::Dbl(vec![6.0]),
    );
    assert!(eval_err("break").contains("'break' used outside a loop"));
    assert!(eval_err("next").contains("outside a loop"));
    // escaping a function body is an error
    assert!(eval_err("f <- function() break\nfor (i in 1:3) f()").contains("outside a loop"));
}

#[test]
fn conformance_g9_break_in_condition_gap() {
    // Known gap: break inside an if condition errors even within a loop.
    // This asserts the CURRENT behaviour, not the desired one.
    assert!(eval_err("for (i in 1:3) { if (break) 1 }").contains("outside a loop"));
}

#[test]
fn conformance_g9_extract_index_kinds() {
    assert_val(
        "d <- data.frame(a = c(1, 2, 3), b = c(4, 5, 6))\nd[2, 1]",
        R0Val::Dbl(vec![2.0]),
    );
    assert_eq!(g9("d[2:3, 1]").len(), 2);
    assert_eq!(
        format!("{:?}", g9("d[-1, \"a\"]")),
        format!("{:?}", R0Val::Dbl(vec![2.0, 3.0]))
    );
    assert_eq!(
        format!("{:?}", g9("d[c(TRUE, FALSE, TRUE), \"b\"]")),
        format!("{:?}", R0Val::Dbl(vec![4.0, 6.0]))
    );
    // NOTE: src rejects a short row mask (ErrDimensionMismatch) rather than recycling it
    assert!(g9_err("d[c(TRUE, FALSE), 1]").contains("ErrDimensionMismatch"));
    assert_eq!(
        format!("{:?}", g9("d[2, \"c\"]")),
        format!("{:?}", R0Val::Dbl(vec![8.0]))
    );
    // empty slots
    assert_eq!(
        format!("{:?}", g9("d[, 2]")),
        format!("{:?}", R0Val::Dbl(vec![4.0, 5.0, 6.0]))
    );
    assert!(matches!(g9("d[, ]"), R0Val::Df { .. }));
}

#[test]
fn conformance_g9_extract_drop_rule() {
    assert!(matches!(g9("d[, 1]"), R0Val::Dbl(_)));
    assert!(matches!(g9("d[1, 1]"), R0Val::Dbl(_)));
    match g9("d[, 1:2]") {
        R0Val::Df { names, cols } => {
            assert_eq!(names, vec!["a", "b"]);
            assert_eq!(cols.len(), 2);
        }
        o => panic!("{o:?}"),
    }
    match g9("d[2, ]") {
        R0Val::Df { names, cols } => {
            assert_eq!(names.len(), 3);
            assert!(cols.iter().all(|c| c.len() == 1));
        }
        o => panic!("{o:?}"),
    }
    assert!(matches!(g9("d[-1, ]"), R0Val::Df { .. }));
}

#[test]
fn conformance_g9_extract_errors() {
    assert!(g9_err("d[, c(-1, 2)]").contains("ErrMixedSubscript"));
    assert!(g9_err("d[c(TRUE, NA, FALSE), ]").contains("ErrNASubscript"));
    assert!(g9_err("d[, \"zz\"]").contains("Unknown column"));
    assert!(g9_err("d[, 4]").contains("ErrBoundsTrap"));
    assert!(g9_err("d[4, ]").contains("ErrBoundsTrap"));
    assert!(g9_err("d[\"a\", ]").contains("column slot"));
    assert!(g9_err("d[1, 2, 3]").contains("Too many subscripts"));
}

#[test]
fn conformance_g9_df_list_single_index() {
    match g9("d[2]") {
        R0Val::Df { names, .. } => assert_eq!(names, vec!["b"]),
        o => panic!("{o:?}"),
    }
    match g9("d[c(-1, -2)]") {
        R0Val::Df { names, .. } => assert_eq!(names, vec!["c"]),
        o => panic!("{o:?}"),
    }
    match g9("d[c(TRUE, FALSE, TRUE)]") {
        R0Val::Df { names, .. } => assert_eq!(names, vec!["a", "c"]),
        o => panic!("{o:?}"),
    }
    assert!(matches!(g9("d[\"a\"]"), R0Val::Df { .. }));
    let l = "l <- list(x = 1, y = c(2, 3), z = \"q\")\n";
    match eval(&format!("{l}l[2]")) {
        R0Val::List { names, cols } => {
            assert_eq!(names, vec!["y"]);
            assert_eq!(cols[0].len(), 2);
        }
        o => panic!("{o:?}"),
    }
    match eval(&format!("{l}l[c(\"x\", \"z\")]")) {
        R0Val::List { names, .. } => assert_eq!(names, vec!["x", "z"]),
        o => panic!("{o:?}"),
    }
    assert!(matches!(eval(&format!("{l}l[-1]")), R0Val::List { .. }));
    assert!(eval_err(&format!("{l}l[4]")).contains("ErrBoundsTrap"));
    assert!(g9_err("d[3 + 1]").contains("ErrBoundsTrap"));
}

#[test]
fn conformance_g9_assign_2d() {
    let b = |v: Vec<f64>| format!("{:?}", R0Val::Dbl(v));
    assert_eq!(
        format!("{:?}", g9("d[2, 1] <- 9\nd[, 1]")),
        b(vec![1.0, 9.0, 3.0])
    );
    assert_eq!(format!("{:?}", g9("d[, 1] <- 5\nd[, 1]")), b(vec![5.0; 3]));
    assert_eq!(
        format!("{:?}", g9("d[-1, \"a\"] <- c(7, 8)\nd[, 1]")),
        b(vec![1.0, 7.0, 8.0])
    );
    assert_eq!(
        format!("{:?}", g9("d[c(TRUE, FALSE, TRUE), 1] <- 0\nd[, 1]")),
        b(vec![0.0, 2.0, 0.0])
    );
    // column-major recycling
    assert_eq!(
        format!("{:?}", g9("d[1:2, 1:2] <- c(10, 20)\nd[, 2]")),
        b(vec![10.0, 20.0, 6.0])
    );
    assert_eq!(
        format!("{:?}", g9("d[1:2, 1:2] <- c(10, 20, 30, 40)\nd[, 2]")),
        b(vec![30.0, 40.0, 6.0])
    );
    // whole row via empty slot
    assert_eq!(
        format!("{:?}", g9("d[1, ] <- 0\nd[, 3]")),
        b(vec![0.0, 8.0, 9.0])
    );
    match g9("d[2:3, 1:2] <- 0\nd") {
        R0Val::Df { cols, .. } => assert!(cols.iter().all(|c| c.len() == 3)),
        o => panic!("{o:?}"),
    }
}

#[test]
fn conformance_g9_assign_1index() {
    let b = |v: Vec<f64>| format!("{:?}", R0Val::Dbl(v));
    assert_eq!(
        format!("{:?}", g9("d[\"a\"] <- 0\nd[, 1]")),
        b(vec![0.0; 3])
    );
    assert_eq!(
        format!("{:?}", g9("d[c(1, 2)] <- 1:3\nd[, 2]")),
        format!("{:?}", R0Val::Int(vec![1, 2, 3]))
    );
    assert_eq!(format!("{:?}", g9("d[-1] <- 8\nd[, 2]")), b(vec![8.0; 3]));
    match eval("l <- list(a = 1, b = 2)\nl[\"b\"] <- c(1, 2, 3)\nl") {
        R0Val::List { names, cols } => {
            assert_eq!(names.len(), 2);
            assert_eq!(cols[1].len(), 1);
            assert_eq!(format!("{:?}", cols[1]), b(vec![1.0]));
        }
        o => panic!("{o:?}"),
    }
}

#[test]
fn conformance_g9_assign_errors() {
    assert!(g9_err("d[1:2, 1:2] <- c(1, 2, 3)").contains("ErrDimensionMismatch"));
    assert!(g9_err("d[4, 1] <- 1").contains("ErrBoundsTrap"));
    assert!(g9_err("d[1, 4] <- 1").contains("ErrBoundsTrap"));
    assert!(g9_err("d[1, \"zz\"] <- 1").contains("Unknown column"));
    assert!(g9_err("d[\"q\"] <- 1").contains("Unknown column"));
    assert!(g9_err("d[4] <- 1").contains("ErrBoundsTrap"));
    assert!(g9_err("d[1] <- c(1, 2)").contains("ErrDimensionMismatch"));
    assert!(g9_err("d[c(TRUE, NA, FALSE), 1] <- 1").contains("ErrNASubscript"));
    assert!(g9_err("d[c(-1, 2), 1] <- 1").contains("ErrMixedSubscript"));
    assert!(eval_err("l <- list(a = 1)\nl[1, 1] <- 1").contains("data frame"));
    assert!(eval_err("l <- list(a = 1)\nl[2] <- 1").contains("ErrBoundsTrap"));
}

#[test]
fn conformance_g9_fuel_exhaustion_large_frame() {
    let pre = "d <- data.frame(a = 1:200000, b = 1:200000)\n";
    for tail in ["d[, 1]", "d[1:1000, ]", "d[1] <- 0", "d[, 1] <- 0"] {
        let err = R0Vm::new(100_000)
            .eval_script(&format!("{pre}{tail}"))
            .unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{tail}: {err}");
    }
}

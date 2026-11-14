//! Conformance tests: newline handling, the `|>` pipe, Kleene logic, broadcasting and indexing.

use r0_wasm::value::{is_na_real, R0Val, LGL_FALSE, LGL_NA, LGL_TRUE, NA_INT};
use r0_wasm::lexer::tokenize;
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
    assert_eq!(format!("{:?}", eval("f <- function() { return() }; f()")), "Null");
}

#[test]
fn spec_04_pmin_pmax_preserve_integer_type() {
    assert_eq!(format!("{:?}", eval("pmin(1L, 2L)")), "Int([1])");
    assert_eq!(format!("{:?}", eval("pmax(1L, 2L)")), "Int([2])");
}

#[test]
fn state_01_string_escapes() {
    assert!(tokenize(r#""a\"b""#).is_ok());
    assert_eq!(format!("{:?}", eval(r#""a\"b""#)), "Chr([Some(\"a\\\"b\")])");
    assert_eq!(format!("{:?}", eval(r#"'a\\b'"#)), "Chr([Some(\"a\\\\b\")])");
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
    assert_val("+(1:3)", R0Val::Range { start: 1, end: 3, step: 1 });
    assert!(eval_err("+TRUE").contains("Unary '+' requires numeric or integer vector"));
    assert!(eval_err("-\"a\"").contains("Unary '-' requires numeric or integer vector"));
}

#[test]
fn locked_prelude_and_strict_user_function_args() {
    assert!(eval_err("sum <- function(x) 0").contains("Cannot shadow built-in intrinsic"));
    assert_val("f <- function(a, b = 10) a - b; f(b = 3, 10)", R0Val::Dbl(vec![7.0]));
    assert!(eval_err("f <- function(a) a; f(1, 2)").starts_with("TooManyArguments:"));
    assert!(eval_err("f <- function(a) a; f(unknown = 1)").starts_with("UnknownArgument:"));
    assert!(eval_err("f <- function(a) a; f(a = 1, a = 2)").starts_with("DuplicateArgument:"));
}

#[test]
fn na_rm_validation() {
    assert_val("sum(c(1L, NA_integer_), na.rm = c(TRUE, FALSE))", R0Val::Int(vec![1]));
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
    assert_val("logical(3)", R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]));
    assert_val(
        "character(3)",
        R0Val::Chr(vec![Some(String::new()), Some(String::new()), Some(String::new())]),
    );

    // Named argument
    assert_val("numeric(length = 2)", R0Val::Dbl(vec![0.0, 0.0]));
    assert_val("integer(length = 2L)", R0Val::Int(vec![0, 0]));

    // Error cases: negative length, NA length, non-scalar, non-numeric
    assert!(eval_err("numeric(-1)").contains("requires a non-negative finite scalar length"));
    assert!(eval_err("integer(NA_integer_)").contains("requires a non-negative finite scalar length"));
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
    assert_val("as.integer(c(TRUE, FALSE, NA))", R0Val::Int(vec![1, 0, NA_INT]));
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
    assert_val("rep_len(c(10.0, 20.0), 3)", R0Val::Dbl(vec![10.0, 20.0, 10.0]));
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
    assert_val("rep_len(c(1L, NA_integer_), 3)", R0Val::Int(vec![1, NA_INT, 1]));

    // Error cases: empty vector with positive length, negative length, NA length
    assert!(eval_err("rep_len(c(), 5)").contains("cannot replicate empty vector with positive length.out"));
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
    assert_val("rep(1:3, times = 10, length.out = 4)", R0Val::Int(vec![1, 2, 3, 1]));
    assert_val("rep(1:2, each = 3, length.out = 4)", R0Val::Int(vec![1, 1, 1, 2]));

    // Empty vector handling
    assert_val("rep(c(), times = 3)", R0Val::Null);
    assert_val("rep(c(), each = 3)", R0Val::Null);

    // NA handling
    assert_val(
        "rep(c(NA_character_, \"x\"), each = 2)",
        R0Val::Chr(vec![None, None, Some("x".into()), Some("x".into())]),
    );

    // Error cases
    assert!(eval_err("rep(c(), length.out = 3)").contains("cannot replicate empty vector with positive length.out"));
    assert!(eval_err("rep(1:3, times = -1)").contains("invalid 'times' argument"));
    assert!(eval_err("rep(1:3, each = -1)").contains("invalid 'each' argument"));
    assert!(eval_err("rep(1:3, length.out = -1)").contains("invalid 'length.out' argument"));
    assert!(eval_err("rep(1:3, times = NA_integer_)").contains("invalid 'times' argument"));
    assert!(eval_err("rep()").contains("rep: missing argument 1"));
}

#[test]
fn conformance_g6_seq() {
    // seq(n)
    assert_val("seq(4)", R0Val::Range { start: 1, end: 4, step: 1 });
    assert_val("seq(4L)", R0Val::Range { start: 1, end: 4, step: 1 });
    assert_val("seq(0)", R0Val::Range { start: 1, end: 0, step: -1 });
    assert_val("seq(c())", R0Val::Int(vec![]));

    // seq(from, to)
    assert_val("seq(2, 5)", R0Val::Range { start: 2, end: 5, step: 1 });
    assert_val("seq(5, 2)", R0Val::Range { start: 5, end: 2, step: -1 });
    assert_val("seq(3, 3)", R0Val::Range { start: 3, end: 3, step: 1 });

    // seq(from, to, by)
    assert_val("seq(1, 5, by = 2)", R0Val::Dbl(vec![1.0, 3.0, 5.0]));
    assert_val("seq(1L, 5L, by = 2L)", R0Val::Int(vec![1, 3, 5]));
    assert_val("seq(5, 1, by = -2)", R0Val::Dbl(vec![5.0, 3.0, 1.0]));

    // seq(from, to, length.out)
    assert_val("seq(1, 10, length.out = 4)", R0Val::Dbl(vec![1.0, 4.0, 7.0, 10.0]));
    assert_val("seq(from = 1, length.out = 3)", R0Val::Dbl(vec![1.0, 2.0, 3.0]));

    // seq(along.with)
    assert_val(
        "seq(along.with = c(\"a\", \"b\", \"c\"))",
        R0Val::Range { start: 1, end: 3, step: 1 },
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
        assert!(err.starts_with("ErrFuelExhausted:"), "src: {src}, got: {err}");
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

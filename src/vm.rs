//! Virtual machine that evaluates the AST.

use std::collections::HashMap;

use crate::ast::{BinOp, Expr, UnaryOp};
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::value::{
    is_na_real, na_real, R0Val, KLEENE_AND_LUT, KLEENE_NOT_LUT, KLEENE_OR_LUT, LGL_FALSE,
    LGL_NA, LGL_TRUE, NA_INT,
};

#[derive(Clone, Debug)]
pub struct UserFn {
    params: Vec<(String, Option<Expr>)>,
    body: Expr,
    captured_env: HashMap<String, R0Val>, // Immutable closure capture
}

pub struct R0Vm {
    pub globals: HashMap<String, R0Val>,
    pub functions: HashMap<String, UserFn>,
    pub fuel: i64,
}

enum ControlFlow {
    Val(R0Val),
    Return(R0Val),
}

/// 1-based element extraction for `for` loops (not subsetting).
fn elem_at(seq: &R0Val, k: usize) -> Result<R0Val, String> {
    Ok(match seq {
        R0Val::Dbl(v) => R0Val::Dbl(vec![v[k]]),
        R0Val::Int(v) => R0Val::Int(vec![v[k]]),
        R0Val::Lgl(v) => R0Val::Lgl(vec![v[k]]),
        R0Val::Chr(v) => R0Val::Chr(vec![v[k].clone()]),
        R0Val::Range { start, step, .. } => R0Val::Int(vec![start + (k as i32) * step]),
        _ => return Err("'for' requires an atomic vector or range sequence".into()),
    })
}

impl R0Vm {
    pub fn new(fuel: i64) -> Self {
        Self {
            globals: HashMap::new(),
            functions: HashMap::new(),
            fuel,
        }
    }

    fn consume_fuel(&mut self, units: usize) -> Result<(), String> {
        self.fuel -= units.max(1) as i64;
        if self.fuel <= 0 {
            Err("ErrFuelExhausted: Execution exceeded deterministic instruction budget".into())
        } else {
            Ok(())
        }
    }

    pub fn eval_script(&mut self, src: &str) -> Result<R0Val, String> {
        let tokens = tokenize(src)?;
        let mut parser = Parser::new(tokens);
        let ast = parser.parse_program()?;
        let mut env = self.globals.clone();
        let res = match self.eval(&ast, &mut env)? {
            ControlFlow::Val(v) | ControlFlow::Return(v) => v,
        };
        self.globals = env;
        Ok(res)
    }

    fn eval(&mut self, expr: &Expr, env: &mut HashMap<String, R0Val>) -> Result<ControlFlow, String> {
        self.consume_fuel(1)?;
        match expr {
            Expr::Dbl(v) => Ok(ControlFlow::Val(R0Val::Dbl(vec![*v]))),
            Expr::Int(v) => Ok(ControlFlow::Val(R0Val::Int(vec![*v]))),
            Expr::Lgl(v) => Ok(ControlFlow::Val(R0Val::Lgl(vec![*v]))),
            Expr::Str(s) => Ok(ControlFlow::Val(R0Val::Chr(vec![Some(s.clone())]))),
            Expr::Placeholder => Err("Placeholder '_' is only valid inside a '|>' pipe".into()),
            Expr::Ident(name) => env
                .get(name)
                .cloned()
                .map(ControlFlow::Val)
                .ok_or_else(|| format!("Compile/Scope Error: Undeclared variable '{}'", name)),
            Expr::Block(stmts) => {
                let mut last = R0Val::Null;
                for s in stmts {
                    match self.eval(s, env)? {
                        ControlFlow::Val(v) => last = v,
                        ret @ ControlFlow::Return(_) => return Ok(ret),
                    }
                }
                Ok(ControlFlow::Val(last))
            }
            Expr::Return(inner) => {
                let v = self.eval_val(inner, env)?;
                Ok(ControlFlow::Return(v))
            }
            Expr::Assign { target, value } => {
                if let Expr::FnDef { params, body } = &**value {
                    if let Expr::Ident(fname) = &**target {
                        self.functions.insert(
                            fname.clone(),
                            UserFn {
                                params: params.clone(),
                                body: (**body).clone(),
                                captured_env: env.clone(),
                            },
                        );
                        return Ok(ControlFlow::Val(R0Val::Null));
                    }
                }
                let val = self.eval_val(value, env)?;
                match &**target {
                    Expr::Ident(name) => {
                        env.insert(name.clone(), val.clone());
                        Ok(ControlFlow::Val(val))
                    }
                    Expr::Index1 { target: base, idx } => {
                        if let Expr::Ident(name) = &**base {
                            let idx_val = self.eval_val(idx, env)?;
                            let current = env
                                .get(name)
                                .ok_or_else(|| format!("Undeclared variable '{}'", name))?
                                .clone();
                            let updated = self.subset_assign(current, &idx_val, &val)?;
                            env.insert(name.clone(), updated.clone());
                            Ok(ControlFlow::Val(updated))
                        } else {
                            Err("Complex nested subset assignment target not supported".into())
                        }
                    }
                    _ => Err("Invalid assignment target".into()),
                }
            }
            Expr::Unary(op, inner) => {
                let v = self.eval_val(inner, env)?;
                self.consume_fuel(v.len())?;
                match op {
                    UnaryOp::Pos => Ok(ControlFlow::Val(v)),
                    UnaryOp::Neg => match v {
                        R0Val::Dbl(xs) => Ok(ControlFlow::Val(R0Val::Dbl(
                            xs.into_iter().map(|x| -x).collect(), // IEEE 754 -NaN preserves quiet NaN
                        ))),
                        R0Val::Int(xs) => Ok(ControlFlow::Val(R0Val::Int(
                            xs.into_iter().map(|x| if x == NA_INT { NA_INT } else { -x }).collect(),
                        ))),
                        _ => Err("Unary '-' requires numeric or integer vector".into()),
                    },
                    UnaryOp::Not => match v {
                        R0Val::Lgl(xs) => Ok(ControlFlow::Val(R0Val::Lgl(
                            xs.into_iter().map(|b| KLEENE_NOT_LUT[(b & 3) as usize]).collect(),
                        ))),
                        _ => Err("Logical '!' requires logical (lgl) vector".into()),
                    },
                }
            }
            Expr::Binary(op, lhs, rhs) => {
                // Short-circuit scalar operators && and ||
                if matches!(op, BinOp::And2 | BinOp::Or2) {
                    let l = self.extract_scalar_cond(lhs, env)?;
                    if matches!(op, BinOp::And2) && !l {
                        return Ok(ControlFlow::Val(R0Val::Lgl(vec![LGL_FALSE])));
                    }
                    if matches!(op, BinOp::Or2) && l {
                        return Ok(ControlFlow::Val(R0Val::Lgl(vec![LGL_TRUE])));
                    }
                    let r = self.extract_scalar_cond(rhs, env)?;
                    return Ok(ControlFlow::Val(R0Val::Lgl(vec![if r { LGL_TRUE } else { LGL_FALSE }])));
                }
                let lv = self.eval_val(lhs, env)?;
                let rv = self.eval_val(rhs, env)?;
                Ok(ControlFlow::Val(self.eval_binary(*op, &lv, &rv)?))
            }
            Expr::Index1 { target, idx } => {
                let base = self.eval_val(target, env)?;
                let sub = self.eval_val(idx, env)?;
                Ok(ControlFlow::Val(self.subset(&base, &sub)?))
            }
            Expr::IndexScalar { target, idx } => {
                let base = self.eval_val(target, env)?;
                let sub = self.eval_val(idx, env)?;
                Ok(ControlFlow::Val(self.index_scalar(&base, &sub)?))
            }
            Expr::Field { target, name } => {
                let base = self.eval_val(target, env)?;
                Ok(ControlFlow::Val(self.field(&base, name)?))
            }
            Expr::If { cond, then_br, else_br } => {
                if self.extract_scalar_cond(cond, env)? {
                    self.eval(then_br, env)
                } else if let Some(eb) = else_br {
                    self.eval(eb, env)
                } else {
                    Ok(ControlFlow::Val(R0Val::Null))
                }
            }
            Expr::For { var, iter, body } => {
                let seq = self.eval_val(iter, env)?;
                let mut last = R0Val::Null;
                for idx in 0..seq.len() {
                    env.insert(var.clone(), elem_at(&seq, idx)?);
                    match self.eval(body, env)? {
                        ControlFlow::Val(v) => last = v,
                        ret @ ControlFlow::Return(_) => return Ok(ret),
                    }
                }
                Ok(ControlFlow::Val(last))
            }
            Expr::While { cond, body } => {
                let mut last = R0Val::Null;
                while self.extract_scalar_cond(cond, env)? {
                    match self.eval(body, env)? {
                        ControlFlow::Val(v) => last = v,
                        ret @ ControlFlow::Return(_) => return Ok(ret),
                    }
                }
                Ok(ControlFlow::Val(last))
            }
            Expr::Call { func, args } => {
                let mut eval_args = Vec::with_capacity(args.len());
                for (k, ex) in args {
                    eval_args.push((k.clone(), self.eval_val(ex, env)?));
                }
                if let Some(ufn) = self.functions.get(func).cloned() {
                    return Ok(ControlFlow::Val(self.call_user(func, &ufn, eval_args)?));
                }
                Ok(ControlFlow::Val(self.call_intrinsic(func, eval_args)?))
            }
            Expr::FnDef { .. } => Err("Anonymous functions must be bound to an identifier in R0".into()),
        }
    }

    fn eval_val(&mut self, expr: &Expr, env: &mut HashMap<String, R0Val>) -> Result<R0Val, String> {
        match self.eval(expr, env)? {
            ControlFlow::Val(v) | ControlFlow::Return(v) => Ok(v),
        }
    }

    /// Bind args to params (named match, then positional, then default) and run the body.
    fn call_user(
        &mut self,
        func: &str,
        ufn: &UserFn,
        args: Vec<(Option<String>, R0Val)>,
    ) -> Result<R0Val, String> {
        let mut local_env = ufn.captured_env.clone();
        for (i, (pname, default_expr)) in ufn.params.iter().enumerate() {
            if let Some((_, val)) = args.iter().find(|(k, _)| k.as_deref() == Some(pname.as_str())) {
                local_env.insert(pname.clone(), val.clone());
            } else if let Some((None, val)) = args.get(i) {
                local_env.insert(pname.clone(), val.clone());
            } else if let Some(def) = default_expr {
                let v = self.eval_val(def, &mut local_env)?;
                local_env.insert(pname.clone(), v);
            } else {
                return Err(format!("Missing required argument '{}' in call to '{}'", pname, func));
            }
        }
        self.eval_val(&ufn.body, &mut local_env)
    }

    fn extract_scalar_cond(&mut self, expr: &Expr, env: &mut HashMap<String, R0Val>) -> Result<bool, String> {
        let v = self.eval_val(expr, env)?;
        match v {
            R0Val::Lgl(xs) if xs.len() == 1 => match xs[0] {
                LGL_TRUE => Ok(true),
                LGL_FALSE => Ok(false),
                _ => Err("ErrInvalidCondition: condition evaluated to NA in control flow".into()),
            },
            _ => Err(format!(
                "ErrInvalidCondition: if/while requires length-1 logical (lgl), got {:?}",
                v
            )),
        }
    }

    /// Strict Broadcasting Rule: lengths must match, or one operand must have length 1.
    fn broadcast_len(la: usize, lb: usize) -> Result<usize, String> {
        if la == 0 || lb == 0 {
            Ok(0)
        } else if la == lb {
            Ok(la)
        } else if la == 1 {
            Ok(lb)
        } else if lb == 1 {
            Ok(la)
        } else {
            Err(format!(
                "ErrDimensionMismatch: Strict broadcasting forbids recycling length {} against length {}",
                la, lb
            ))
        }
    }

    fn eval_binary(&mut self, op: BinOp, a: &R0Val, b: &R0Val) -> Result<R0Val, String> {
        if matches!(op, BinOp::Colon) {
            let sa = a.materialize_int()?;
            let sb = b.materialize_int()?;
            if sa.len() != 1 || sb.len() != 1 || sa[0] == NA_INT || sb[0] == NA_INT {
                return Err("':' range operator requires non-NA length-1 integer bounds".into());
            }
            let step = if sa[0] <= sb[0] { 1 } else { -1 };
            return Ok(R0Val::Range { start: sa[0], end: sb[0], step });
        }
        if matches!(op, BinOp::InOp) {
            // Parser desugars `%in%` to an intrinsic call (D17); no BinOp path.
            return Err("ErrNotImplemented: BinOp::InOp".into());
        }
        let n = Self::broadcast_len(a.len(), b.len())?;
        self.consume_fuel(n)?;
        // Kleene Logical & and | using the 16-byte LUT
        if matches!(op, BinOp::And | BinOp::Or) {
            let (R0Val::Lgl(la), R0Val::Lgl(lb)) = (a, b) else {
                return Err("Logical '&' and '|' require logical (lgl) operands".into());
            };
            let lut = if matches!(op, BinOp::And) { &KLEENE_AND_LUT } else { &KLEENE_OR_LUT };
            let mut out = Vec::with_capacity(n);
            for i in 0..n {
                let va = la[if la.len() == 1 { 0 } else { i }] & 3;
                let vb = lb[if lb.len() == 1 { 0 } else { i }] & 3;
                out.push(lut[((va << 2) | vb) as usize]);
            }
            return Ok(R0Val::Lgl(out));
        }
        // Numeric & comparison operations (promote int -> dbl)
        let da = a.promote_dbl()?;
        let db = b.promote_dbl()?;
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow | BinOp::Mod | BinOp::IntDiv => {
                let mut out = Vec::with_capacity(n);
                for i in 0..n {
                    let va = da[if da.len() == 1 { 0 } else { i }];
                    let vb = db[if db.len() == 1 { 0 } else { i }];
                    let res = if is_na_real(va) || is_na_real(vb) {
                        na_real()
                    } else {
                        match op {
                            BinOp::Add => va + vb,
                            BinOp::Sub => va - vb,
                            BinOp::Mul => va * vb,
                            BinOp::Div => va / vb,
                            BinOp::Pow => va.powf(vb),
                            BinOp::Mod => va % vb,
                            BinOp::IntDiv => (va / vb).floor(),
                            _ => unreachable!(),
                        }
                    };
                    out.push(res);
                }
                Ok(R0Val::Dbl(out))
            }
            BinOp::Eq | BinOp::Ne | BinOp::Lt | BinOp::Le | BinOp::Gt | BinOp::Ge => {
                let mut out = Vec::with_capacity(n);
                for i in 0..n {
                    let va = da[if da.len() == 1 { 0 } else { i }];
                    let vb = db[if db.len() == 1 { 0 } else { i }];
                    if is_na_real(va) || is_na_real(vb) {
                        out.push(LGL_NA);
                    } else {
                        let c = match op {
                            BinOp::Eq => va == vb,
                            BinOp::Ne => va != vb,
                            BinOp::Lt => va < vb,
                            BinOp::Le => va <= vb,
                            BinOp::Gt => va > vb,
                            BinOp::Ge => va >= vb,
                            _ => unreachable!(),
                        };
                        out.push(if c { LGL_TRUE } else { LGL_FALSE });
                    }
                }
                Ok(R0Val::Lgl(out))
            }
            _ => Err(format!("ErrNotImplemented: BinOp {:?}", op)),
        }
    }

    // ---- Stubs replaced by 1.5.2 (subsetting) and 1.5.3 (intrinsics) ----

    fn subset(&mut self, _base: &R0Val, _idx: &R0Val) -> Result<R0Val, String> {
        Err("ErrNotImplemented: subset".into())
    }

    fn index_scalar(&mut self, _base: &R0Val, _idx: &R0Val) -> Result<R0Val, String> {
        Err("ErrNotImplemented: index_scalar".into())
    }

    fn field(&mut self, _base: &R0Val, _name: &str) -> Result<R0Val, String> {
        Err("ErrNotImplemented: field".into())
    }

    fn subset_assign(&mut self, _base: R0Val, _idx: &R0Val, _val: &R0Val) -> Result<R0Val, String> {
        Err("ErrNotImplemented: subset_assign".into())
    }

    fn call_intrinsic(&mut self, _func: &str, _args: Vec<(Option<String>, R0Val)>) -> Result<R0Val, String> {
        Err("ErrNotImplemented: call_intrinsic".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(src: &str) -> Result<R0Val, String> {
        R0Vm::new(100_000).eval_script(src)
    }

    /// R0Val has no PartialEq (value.rs is out of scope), so compare Debug forms.
    fn same(got: Result<R0Val, String>, want: R0Val) {
        assert_eq!(format!("{:?}", got.unwrap()), format!("{:?}", want));
    }

    #[test]
    fn fuel_exhausts_on_infinite_loop() {
        let mut vm = R0Vm::new(100);
        let err = vm.eval_script("while (TRUE) { x <- 1 }").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
    }

    #[test]
    fn broadcast_scalar_and_dimension_mismatch() {
        let mut vm = R0Vm::new(100_000);
        vm.globals.insert("c1".into(), R0Val::Dbl(vec![1.0, 2.0, 3.0, 4.0]));
        vm.globals.insert("c2".into(), R0Val::Dbl(vec![10.0, 20.0]));
        let err = vm.eval_script("c1 + c2").unwrap_err();
        assert!(err.starts_with("ErrDimensionMismatch:"), "{err}");
        same(vm.eval_script("1 + c2"), R0Val::Dbl(vec![11.0, 21.0]));
        assert_eq!(R0Vm::broadcast_len(0, 3), Ok(0));
    }

    #[test]
    fn kleene_and_and_short_circuit() {
        same(run("FALSE & NA"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("TRUE & NA"), R0Val::Lgl(vec![LGL_NA]));
        same(run("FALSE && NA"), R0Val::Lgl(vec![LGL_FALSE]));
    }

    #[test]
    fn invalid_conditions_trap() {
        let err = run("if (NA) 1").unwrap_err();
        assert!(err.starts_with("ErrInvalidCondition:"), "{err}");
        let mut vm = R0Vm::new(100_000);
        vm.globals.insert("two".into(), R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE]));
        let err = vm.eval_script("if (two) 1").unwrap_err();
        assert!(err.starts_with("ErrInvalidCondition:"), "{err}");
    }

    #[test]
    fn assignment_and_arithmetic() {
        same(run("x <- 2; y <- x * 3 + 1; y"), R0Val::Dbl(vec![7.0]));
    }

    #[test]
    fn user_function_binds_defaults_and_named_args() {
        same(run("f <- function(x, y = 10) x + y; f(1)"), R0Val::Dbl(vec![11.0]));
        same(run("f <- function(x, y = 10) x + y; f(1, y = 5)"), R0Val::Dbl(vec![6.0]));
    }

    #[test]
    fn stubs_report_not_implemented() {
        assert!(run("(1)[1L]").unwrap_err().starts_with("ErrNotImplemented: subset"));
        assert!(run("sum(1)").unwrap_err().starts_with("ErrNotImplemented: call_intrinsic"));
    }
}

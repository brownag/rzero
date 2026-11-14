//! Virtual machine that evaluates the AST.

use std::collections::HashMap;

use crate::ast::{BinOp, Expr, UnaryOp};
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::value::{
    is_missing, is_na_real, na_real, Lgl, R0Val, KLEENE_AND_LUT, KLEENE_NOT_LUT, KLEENE_OR_LUT, LGL_FALSE,
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

enum ConditionFlow {
    Bool(bool),
    Return(R0Val),
}

/// 1-based element extraction for `for` loops (not subsetting).
fn elem_at(seq: &R0Val, k: usize) -> Result<R0Val, String> {
    Ok(match seq {
        R0Val::Dbl(v) => R0Val::Dbl(vec![v[k]]),
        R0Val::Int(v) => R0Val::Int(vec![v[k]]),
        R0Val::Lgl(v) => R0Val::Lgl(vec![v[k]]),
        R0Val::Chr(v) => R0Val::Chr(vec![v[k].clone()]),
        R0Val::Range { start, step, .. } => {
            R0Val::Int(vec![(*start as i64 + k as i64 * *step as i64) as i32])
        }
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
        self.fuel = self
            .fuel
            .saturating_sub(i64::try_from(units.max(1)).unwrap_or(i64::MAX));
        if self.fuel <= 0 {
            Err("ErrFuelExhausted: Execution exceeded deterministic instruction budget".into())
        } else {
            Ok(())
        }
    }

    fn materialize_int(&mut self, value: &R0Val) -> Result<Vec<i32>, String> {
        self.consume_fuel(value.len())?;
        value.materialize_int()
    }

    fn promote_dbl(&mut self, value: &R0Val) -> Result<Vec<f64>, String> {
        self.consume_fuel(value.len())?;
        value.promote_dbl()
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
            Expr::ChrNA => Ok(ControlFlow::Val(R0Val::Chr(vec![None]))),
            Expr::Null => Ok(ControlFlow::Val(R0Val::Null)),
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
                match self.eval(inner, env)? {
                    ControlFlow::Val(v) => Ok(ControlFlow::Return(v)),
                    ret @ ControlFlow::Return(_) => Ok(ret),
                }
            }
            Expr::Assign { target, value } => {
                if let Expr::FnDef { params, body } = &**value {
                    if let Expr::Ident(fname) = &**target {
                        if Self::is_intrinsic(fname) {
                            return Err(format!("Compile/Scope Error: Cannot shadow built-in intrinsic '{}'", fname));
                        }
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
                let val = match self.eval(value, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                match &**target {
                    Expr::Ident(name) => {
                        env.insert(name.clone(), val.clone());
                        Ok(ControlFlow::Val(val))
                    }
                    Expr::Index1 { target: base, idx } => {
                        if let Expr::Ident(name) = &**base {
                            let idx_val = match self.eval(idx, env)? {
                                ControlFlow::Val(v) => v,
                                ret @ ControlFlow::Return(_) => return Ok(ret),
                            };
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
                let v = match self.eval(inner, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                self.consume_fuel(v.len())?;
                match op {
                    UnaryOp::Pos => match v {
                        R0Val::Dbl(_) | R0Val::Int(_) | R0Val::Range { .. } => Ok(ControlFlow::Val(v)),
                        _ => Err("Unary '+' requires numeric or integer vector".into()),
                    },
                    UnaryOp::Neg => match v {
                        R0Val::Dbl(xs) => Ok(ControlFlow::Val(R0Val::Dbl(
                            xs.into_iter().map(|x| -x).collect(), // IEEE 754 -NaN preserves quiet NaN
                        ))),
                        R0Val::Int(_) | R0Val::Range { .. } => {
                            let xs = self.materialize_int(&v)?;
                            Ok(ControlFlow::Val(R0Val::Int(
                                xs.into_iter().map(|x| if x == NA_INT { NA_INT } else { -x }).collect(),
                            )))
                        }
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
                    let l = match self.extract_scalar_cond(lhs, env)? {
                        ConditionFlow::Bool(v) => v,
                        ConditionFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                    };
                    if matches!(op, BinOp::And2) && !l {
                        return Ok(ControlFlow::Val(R0Val::Lgl(vec![LGL_FALSE])));
                    }
                    if matches!(op, BinOp::Or2) && l {
                        return Ok(ControlFlow::Val(R0Val::Lgl(vec![LGL_TRUE])));
                    }
                    let r = match self.extract_scalar_cond(rhs, env)? {
                        ConditionFlow::Bool(v) => v,
                        ConditionFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                    };
                    return Ok(ControlFlow::Val(R0Val::Lgl(vec![if r { LGL_TRUE } else { LGL_FALSE }])));
                }
                let lv = match self.eval(lhs, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                let rv = match self.eval(rhs, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                Ok(ControlFlow::Val(self.eval_binary(*op, &lv, &rv)?))
            }
            Expr::Index1 { target, idx } => {
                let base = match self.eval(target, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                let sub = match self.eval(idx, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                Ok(ControlFlow::Val(self.subset(&base, &sub)?))
            }
            Expr::IndexScalar { target, idx } => {
                let base = match self.eval(target, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                let sub = match self.eval(idx, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                Ok(ControlFlow::Val(self.index_scalar(&base, &sub)?))
            }
            Expr::Field { target, name } => {
                let base = match self.eval(target, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
                Ok(ControlFlow::Val(self.field(&base, name)?))
            }
            Expr::If { cond, then_br, else_br } => {
                if match self.extract_scalar_cond(cond, env)? {
                    ConditionFlow::Bool(v) => v,
                    ConditionFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                } {
                    self.eval(then_br, env)
                } else if let Some(eb) = else_br {
                    self.eval(eb, env)
                } else {
                    Ok(ControlFlow::Val(R0Val::Null))
                }
            }
            Expr::For { var, iter, body } => {
                let seq = match self.eval(iter, env)? {
                    ControlFlow::Val(v) => v,
                    ret @ ControlFlow::Return(_) => return Ok(ret),
                };
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
                while match self.extract_scalar_cond(cond, env)? {
                    ConditionFlow::Bool(v) => v,
                    ConditionFlow::Return(v) => return Ok(ControlFlow::Return(v)),
                } {
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
                    match self.eval(ex, env)? {
                        ControlFlow::Val(v) => eval_args.push((k.clone(), v)),
                        ret @ ControlFlow::Return(_) => return Ok(ret),
                    }
                }
                if let Some(ufn) = self.functions.get(func).cloned() {
                    return Ok(ControlFlow::Val(self.call_user(func, &ufn, eval_args)?));
                }
                Ok(ControlFlow::Val(self.call_intrinsic(func, eval_args)?))
            }
            Expr::FnDef { .. } => Err("Anonymous functions must be bound to an identifier in R0".into()),
        }
    }

    fn is_intrinsic(name: &str) -> bool {
        matches!(
            name,
            "c" | "data.frame"
                | "list"
                | "approx"
                | "ifelse"
                | "pmin"
                | "pmax"
                | "sum"
                | "mean"
                | "min"
                | "max"
                | "any"
                | "all"
                | "anyNA"
                | "%in%"
                | "length"
                | "seq_len"
                | "seq_along"
                | "is.na"
                | "is.nan"
                | "is.finite"
                | "is.infinite"
                | "which"
                | "which.min"
                | "which.max"
                | "order"
                | "sort"
                | "rank"
                | "match"
                | "unique"
                | "duplicated"
                | "abs"
                | "sqrt"
                | "exp"
                | "log"
                | "log10"
                | "floor"
                | "ceiling"
                | "cumsum"
                | "diff"
                | "prod"
                | "var"
                | "sd"
                | "median"
                | "cumprod"
                | "cummin"
                | "cummax"
                | "sin"
                | "cos"
                | "tan"
                | "acos"
                | "asin"
                | "atan"
                | "trunc"
                | "round"
                | "atan2"
                | "numeric"
                | "integer"
                | "logical"
                | "character"
                | "nrow"
                | "ncol"
                | "as.numeric"
                | "as.integer"
                | "as.logical"
                | "rep"
                | "rep_len"
                | "seq"
        )
    }

    /// Bind args to params (named match, then positional, then default) and run the body.
    fn call_user(
        &mut self,
        func: &str,
        ufn: &UserFn,
        args: Vec<(Option<String>, R0Val)>,
    ) -> Result<R0Val, String> {
        let param_names: Vec<&str> = ufn.params.iter().map(|(p, _)| p.as_str()).collect();
        let slots = Self::bind_args(func, args, &param_names, param_names.len())?;
        let mut local_env = ufn.captured_env.clone();
        for ((pname, default_expr), slot) in ufn.params.iter().zip(slots) {
            if let Some(val) = slot {
                local_env.insert(pname.clone(), val);
            } else if let Some(def) = default_expr {
                match self.eval(def, &mut local_env)? {
                    ControlFlow::Val(v) => local_env.insert(pname.clone(), v),
                    ControlFlow::Return(v) => return Ok(v),
                };
            } else {
                return Err(format!("Missing required argument '{}' in call to '{}'", pname, func));
            }
        }
        match self.eval(&ufn.body, &mut local_env)? {
            ControlFlow::Val(v) | ControlFlow::Return(v) => Ok(v),
        }
    }

    fn extract_scalar_cond(
        &mut self,
        expr: &Expr,
        env: &mut HashMap<String, R0Val>,
    ) -> Result<ConditionFlow, String> {
        let v = match self.eval(expr, env)? {
            ControlFlow::Val(v) => v,
            ControlFlow::Return(v) => return Ok(ConditionFlow::Return(v)),
        };
        match v {
            R0Val::Lgl(xs) if xs.len() == 1 => match xs[0] {
                LGL_TRUE => Ok(ConditionFlow::Bool(true)),
                LGL_FALSE => Ok(ConditionFlow::Bool(false)),
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
            self.consume_fuel(a.len().max(b.len()))?;
            let sa = self.materialize_int(a)?;
            let sb = self.materialize_int(b)?;
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
        if let (R0Val::Int(ia), R0Val::Int(ib)) = (a, b) {
            if matches!(op, BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Mod | BinOp::IntDiv) {
                let mut out = Vec::with_capacity(n);
                for i in 0..n {
                    let lhs = ia[if ia.len() == 1 { 0 } else { i }];
                    let rhs = ib[if ib.len() == 1 { 0 } else { i }];
                    if lhs == NA_INT || rhs == NA_INT {
                        out.push(NA_INT);
                        continue;
                    }
                    if matches!(op, BinOp::Mod | BinOp::IntDiv) && rhs == 0 {
                        out.push(NA_INT);
                        continue;
                    }
                    let lhs64 = lhs as i64;
                    let rhs64 = rhs as i64;
                    let value = match op {
                        BinOp::Add => lhs64 + rhs64,
                        BinOp::Sub => lhs64 - rhs64,
                        BinOp::Mul => lhs64 * rhs64,
                        BinOp::IntDiv => {
                            let quotient = lhs64 / rhs64;
                            if lhs64 % rhs64 != 0 && (lhs64 < 0) != (rhs64 < 0) {
                                quotient - 1
                            } else {
                                quotient
                            }
                        }
                        BinOp::Mod => {
                            let quotient = lhs64 / rhs64;
                            let quotient = if lhs64 % rhs64 != 0 && (lhs64 < 0) != (rhs64 < 0) {
                                quotient - 1
                            } else {
                                quotient
                            };
                            lhs64 - quotient * rhs64
                        }
                        _ => unreachable!(),
                    };
                    if !(i32::MIN as i64 + 1..=i32::MAX as i64).contains(&value) {
                        return Err("ErrIntOverflow: integer arithmetic result out of range".into());
                    }
                    out.push(value as i32);
                }
                return Ok(R0Val::Int(out));
            }
        }
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
        let da = self.promote_dbl(a)?;
        let db = self.promote_dbl(b)?;
        match op {
            BinOp::Add | BinOp::Sub | BinOp::Mul | BinOp::Div | BinOp::Pow | BinOp::Mod | BinOp::IntDiv => {
                let mut out = Vec::with_capacity(n);
                for i in 0..n {
                    let va = da[if da.len() == 1 { 0 } else { i }];
                    let vb = db[if db.len() == 1 { 0 } else { i }];
                    // R mirror: the first missing operand's value propagates (payload kept).
                    let res = if is_missing(va) {
                        va
                    } else if is_missing(vb) {
                        vb
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
                    if is_missing(va) || is_missing(vb) {
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

    /// Elements at 0-based `picks`; `None` yields NA of the variant's type. Keeps the type (D3).
    fn gather(v: &R0Val, picks: &[Option<usize>]) -> R0Val {
        match v {
            R0Val::Dbl(xs) => R0Val::Dbl(picks.iter().map(|p| p.map_or(na_real(), |k| xs[k])).collect()),
            R0Val::Int(xs) => R0Val::Int(picks.iter().map(|p| p.map_or(NA_INT, |k| xs[k])).collect()),
            R0Val::Lgl(xs) => R0Val::Lgl(picks.iter().map(|p| p.map_or(LGL_NA, |k| xs[k])).collect()),
            R0Val::Chr(xs) => R0Val::Chr(picks.iter().map(|p| p.and_then(|k| xs[k].clone())).collect()),
            _ => unreachable!("gather called on non-atomic value"),
        }
    }

    fn subset(&mut self, base: &R0Val, idx: &R0Val) -> Result<R0Val, String> {
        self.consume_fuel(base.len().max(idx.len()))?;
        let owned;
        let base = match base {
            R0Val::Range { .. } => {
                owned = R0Val::Int(self.materialize_int(base)?); // Range subsetting returns Int
                &owned
            }
            R0Val::Df { .. } | R0Val::List { .. } => {
                return Err("TypeError: '[' requires an atomic vector; use '[[' or '$' on df/list".into())
            }
            R0Val::Null => return Ok(R0Val::Null),
            _ => base,
        };
        let n = base.len();
        let picks: Vec<Option<usize>> = match idx {
            R0Val::Lgl(mask) => {
                if mask.len() != n && mask.len() != 1 {
                    return Err(format!(
                        "ErrDimensionMismatch: Logical mask length {} does not match vector length {}",
                        mask.len(),
                        n
                    ));
                }
                (0..n)
                    .filter_map(|i| match mask[if mask.len() == 1 { 0 } else { i }] {
                        LGL_TRUE => Some(Some(i)),
                        LGL_NA => Some(None),
                        _ => None,
                    })
                    .collect()
            }
            _ => {
                let indices = self.materialize_int(idx)?;
                let has_neg = indices.iter().any(|&k| k != NA_INT && k < 0);
                if has_neg {
                    if indices.iter().any(|&k| k > 0 || k == NA_INT) {
                        return Err("ErrMixedSubscript: Cannot mix negative indices with positive or NA indices".into());
                    }
                    let mut drop = vec![false; n];
                    for &k in &indices {
                        let pos = (-k) as usize;
                        if pos >= 1 && pos <= n {
                            drop[pos - 1] = true;
                        }
                    }
                    (0..n).filter(|&i| !drop[i]).map(Some).collect()
                } else {
                    indices
                        .iter()
                        .map(|&k| {
                            if k == NA_INT || k < 1 || (k as usize) > n {
                                None // R's 1-based OOB -> NA rule
                            } else {
                                Some((k - 1) as usize)
                            }
                        })
                        .collect()
                }
            }
        };
        Ok(Self::gather(base, &picks))
    }

    fn index_scalar(&mut self, base: &R0Val, idx: &R0Val) -> Result<R0Val, String> {
        let sub = self.materialize_int(idx)?;
        if sub.len() != 1 || sub[0] == NA_INT || sub[0] < 1 || (sub[0] as usize) > base.len() {
            return Err("ErrBoundsTrap: '[[i]]' requires a valid 1-based scalar index".into());
        }
        let k = (sub[0] - 1) as usize;
        match base {
            // `[[i]]` on df/list returns the i-th column (Q2)
            R0Val::Df { cols, .. } | R0Val::List { cols, .. } => Ok(cols[k].clone()),
            _ => elem_at(base, k),
        }
    }

    fn field(&mut self, base: &R0Val, name: &str) -> Result<R0Val, String> {
        match base {
            R0Val::Df { names, cols } | R0Val::List { names, cols } => names
                .iter()
                .position(|n| n == name)
                .map(|i| cols[i].clone())
                .ok_or_else(|| format!("Unknown column '${}'", name)),
            _ => Err("'$' projection requires a df or list".into()),
        }
    }

    fn subset_assign(&mut self, base: R0Val, idx: &R0Val, val: &R0Val) -> Result<R0Val, String> {
        self.consume_fuel(base.len().max(idx.len()).max(val.len()))?;
        let target = match base {
            R0Val::Range { .. } => R0Val::Int(self.materialize_int(&base)?),
            R0Val::Df { .. } | R0Val::List { .. } | R0Val::Null => {
                return Err("TypeError: cannot assign into NULL, df, or list with '['".into())
            }
            t => t,
        };
        let rhs = match val {
            R0Val::Range { .. } => R0Val::Int(self.materialize_int(val)?),
            v => v.clone(),
        };
        // D3: keep the target type; only Int -> Dbl promotion is allowed.
        let (mut target, rhs) = match (target, rhs) {
            (t @ R0Val::Dbl(_), r @ R0Val::Dbl(_))
            | (t @ R0Val::Int(_), r @ R0Val::Int(_))
            | (t @ R0Val::Lgl(_), r @ R0Val::Lgl(_))
            | (t @ R0Val::Chr(_), r @ R0Val::Chr(_)) => (t, r),
            (t @ R0Val::Dbl(_), R0Val::Int(r)) => (t, R0Val::Dbl(R0Val::Int(r).promote_dbl()?)),
            (R0Val::Int(t), r @ R0Val::Dbl(_)) => (R0Val::Dbl(R0Val::Int(t).promote_dbl()?), r),
            _ => return Err("TypeError: incompatible types in subset assignment".into()),
        };
        // B1: a positive index beyond the end extends the vector with NA of its type.
        let maxk = match idx {
            R0Val::Lgl(_) => 0,
            _ => idx
                .materialize_int()?
                .iter()
                .filter(|&&k| k != NA_INT && k > 0)
                .map(|&k| k as usize)
                .max()
                .unwrap_or(0),
        };
        if maxk > target.len() {
            let fill = maxk - target.len();
            match &mut target {
                R0Val::Dbl(v) => v.extend(std::iter::repeat_n(na_real(), fill)),
                R0Val::Int(v) => v.extend(std::iter::repeat_n(NA_INT, fill)),
                R0Val::Lgl(v) => v.extend(std::iter::repeat_n(LGL_NA, fill)),
                R0Val::Chr(v) => v.extend(std::iter::repeat_n(None, fill)),
                _ => {}
            }
        }
        let n = target.len();
        // `slots[j]` is the target position fed by RHS element j; `None` skips it.
        let slots: Vec<Option<usize>> = match idx {
            R0Val::Lgl(mask) => {
                if mask.len() != n && mask.len() != 1 {
                    return Err(format!(
                        "ErrDimensionMismatch: Logical mask length {} does not match vector length {}",
                        mask.len(),
                        n
                    ));
                }
                (0..n)
                    .filter(|&i| mask[if mask.len() == 1 { 0 } else { i }] == LGL_TRUE)
                    .map(Some)
                    .collect() // NA mask positions are skipped (Q1)
            }
            _ => idx
                .materialize_int()?
                .iter()
                .map(|&k| if k == NA_INT || k < 1 || (k as usize) > n { None } else { Some((k - 1) as usize) })
                .collect(), // positive OOB is extended above (B1)
        };
        let rl = rhs.len();
        if rl != 1 && rl != slots.len() {
            return Err(format!(
                "ErrDimensionMismatch: subset assignment RHS length {} must be 1 or match index length {}",
                rl,
                slots.len()
            ));
        }
        for (j, slot) in slots.iter().enumerate() {
            if let Some(p) = *slot {
                let r = if rl == 1 { 0 } else { j };
                match (&mut target, &rhs) {
                    (R0Val::Dbl(t), R0Val::Dbl(s)) => t[p] = s[r],
                    (R0Val::Int(t), R0Val::Int(s)) => t[p] = s[r],
                    (R0Val::Lgl(t), R0Val::Lgl(s)) => t[p] = s[r],
                    (R0Val::Chr(t), R0Val::Chr(s)) => t[p] = s[r].clone(),
                    _ => unreachable!("types unified above"),
                }
            }
        }
        Ok(target)
    }

    /// Bind named args by exact name first, then fill unbound slots in order from positionals.
    /// Only the first `npos` params take positionals, so params after them (e.g. `na.rm`) are named-only.
    fn bind_args(
        func: &str,
        args: Vec<(Option<String>, R0Val)>,
        params: &[&str],
        npos: usize,
    ) -> Result<Vec<Option<R0Val>>, String> {
        let mut slots: Vec<Option<R0Val>> = (0..params.len()).map(|_| None).collect();
        let mut positional = Vec::new();
        for (k, v) in args {
            match k {
                Some(name) => {
                    let i = params
                        .iter()
                        .position(|p| *p == name)
                        .ok_or_else(|| format!("UnknownArgument: {} in {}", name, func))?;
                    if slots[i].is_some() {
                        return Err(format!("DuplicateArgument: {} in {}", name, func));
                    }
                    slots[i] = Some(v);
                }
                None => positional.push(v),
            }
        }
        for v in positional {
            match slots.iter().take(npos).position(|s| s.is_none()) {
                Some(i) => slots[i] = Some(v),
                None => return Err(format!("TooManyArguments: {} takes at most {} positional arguments", func, npos)),
            }
        }
        Ok(slots)
    }

    fn req(slots: &mut [Option<R0Val>], i: usize, func: &str) -> Result<R0Val, String> {
        slots[i].take().ok_or_else(|| format!("{}: missing argument {}", func, i + 1))
    }

    fn parse_na_rm(opt: Option<R0Val>, func: &str) -> Result<bool, String> {
        match opt {
            None => Ok(false),
            Some(R0Val::Lgl(v)) => match v.first().copied() {
                Some(LGL_TRUE) => Ok(true),
                Some(LGL_FALSE) => Ok(false),
                _ => Err(format!("ErrInvalidCondition: na.rm in {} requires non-NA logical value", func)),
            },
            Some(_) => Err(format!("TypeError: na.rm in {} requires a logical (lgl) argument", func)),
        }
    }

    fn parse_decreasing(opt: Option<R0Val>, func: &str) -> Result<bool, String> {
        match opt {
            None => Ok(false),
            Some(R0Val::Lgl(v)) => {
                if v.len() != 1 {
                    return Err(format!("ErrInvalidCondition: decreasing in {} requires non-NA logical value", func));
                }
                match v[0] {
                    LGL_TRUE => Ok(true),
                    LGL_FALSE => Ok(false),
                    _ => Err(format!("ErrInvalidCondition: decreasing in {} requires non-NA logical value", func)),
                }
            }
            Some(_) => Err(format!("TypeError: decreasing in {} requires a logical (lgl) argument", func)),
        }
    }

    fn compute_sample_ranks<T>(
        n: usize,
        valid: Vec<(usize, T)>,
        missing: Vec<usize>,
        eq: impl Fn(&T, &T) -> bool,
    ) -> Vec<f64> {
        let mut ranks = vec![0.0; n];
        let k = valid.len();
        let mut i = 0;
        while i < valid.len() {
            let mut j = i + 1;
            while j < valid.len() && eq(&valid[j].1, &valid[i].1) {
                j += 1;
            }
            let avg_rank = (i + 1 + j) as f64 / 2.0;
            for item in &valid[i..j] {
                ranks[item.0] = avg_rank;
            }
            i = j;
        }
        for (m_idx, &orig_idx) in missing.iter().enumerate() {
            ranks[orig_idx] = (k + 1 + m_idx) as f64;
        }
        ranks
    }

    fn parse_vector_length(opt: Option<R0Val>, func: &str) -> Result<usize, String> {
        let Some(val) = opt else {
            return Ok(0);
        };
        if val.len() != 1 {
            return Err(format!("{func} requires a length-1 scalar length"));
        }
        match val {
            R0Val::Int(v) => {
                let n = v[0];
                if n == NA_INT || n < 0 {
                    return Err(format!("{func} requires a non-negative finite scalar length"));
                }
                Ok(n as usize)
            }
            R0Val::Dbl(v) => {
                let d = v[0];
                if d.is_nan() || d.is_infinite() || d < 0.0 {
                    return Err(format!("{func} requires a non-negative finite scalar length"));
                }
                if d > (i32::MAX as f64) {
                    return Err(format!("ErrIntOverflow: {func} length exceeds maximum vector size"));
                }
                Ok(d as usize)
            }
            R0Val::Range { start, .. } => {
                if start == NA_INT || start < 0 {
                    return Err(format!("{func} requires a non-negative finite scalar length"));
                }
                Ok(start as usize)
            }
            _ => Err(format!("TypeError: {func} requires a numeric length argument")),
        }
    }

    fn parse_non_negative_scalar(val: &R0Val, arg_name: &str) -> Result<usize, String> {
        if val.len() != 1 {
            return Err(format!("invalid '{arg_name}' argument"));
        }
        match val {
            R0Val::Int(v) => {
                let n = v[0];
                if n == NA_INT || n < 0 {
                    return Err(format!("invalid '{arg_name}' argument"));
                }
                Ok(n as usize)
            }
            R0Val::Dbl(v) => {
                let d = v[0];
                if d.is_nan() || d.is_infinite() || d < 0.0 {
                    return Err(format!("invalid '{arg_name}' argument"));
                }
                if d > (i32::MAX as f64) {
                    return Err(format!("ErrIntOverflow: '{arg_name}' exceeds maximum vector size"));
                }
                Ok(d as usize)
            }
            R0Val::Range { start, .. } => {
                if *start == NA_INT || *start < 0 {
                    return Err(format!("invalid '{arg_name}' argument"));
                }
                Ok(*start as usize)
            }
            _ => Err(format!("invalid '{arg_name}' argument")),
        }
    }

    fn parse_round_digits(opt: Option<R0Val>) -> Result<i32, String> {
        let Some(val) = opt else {
            return Ok(0);
        };
        if val.len() != 1 {
            return Err("round: 'digits' must be a scalar integer or numeric".into());
        }
        match val {
            R0Val::Int(v) => {
                let n = v[0];
                if n == NA_INT {
                    return Err("round: 'digits' cannot be NA".into());
                }
                Ok(n)
            }
            R0Val::Dbl(v) => {
                let d = v[0];
                if is_missing(d) {
                    return Err("round: 'digits' cannot be NA/NaN".into());
                }
                if !d.is_finite() {
                    return Err("round: 'digits' must be finite".into());
                }
                Ok(d.trunc() as i32)
            }
            R0Val::Range { start, end, .. } => {
                if start != end || start == NA_INT {
                    return Err("round: 'digits' must be a scalar integer or numeric".into());
                }
                Ok(start)
            }
            _ => Err("TypeError: round: 'digits' must be an integer or numeric scalar".into()),
        }
    }

    fn round_val(x: f64, digits: i32) -> f64 {
        if is_missing(x) || !x.is_finite() {
            return x;
        }
        if digits == 0 {
            x.round_ties_even()
        } else if digits >= 300 {
            x
        } else if digits <= -300 {
            0.0
        } else if digits > 0 {
            let p = 10.0_f64.powi(digits);
            (x * p).round_ties_even() / p
        } else {
            let p = 10.0_f64.powi(-digits);
            (x / p).round_ties_even() * p
        }
    }

    fn parse_scalar_num(val: &R0Val, arg_name: &str) -> Result<(f64, bool), String> {
        if val.len() != 1 {
            return Err(format!("'{arg_name}' must be of length 1"));
        }
        match val {
            R0Val::Int(v) => {
                let n = v[0];
                if n == NA_INT {
                    return Err(format!("'{arg_name}' cannot be NA"));
                }
                Ok((n as f64, true))
            }
            R0Val::Dbl(v) => {
                let d = v[0];
                if d.is_nan() || d.is_infinite() {
                    return Err(format!("'{arg_name}' must be a finite, non-NA number"));
                }
                let is_int = d.fract() == 0.0 && d >= (i32::MIN + 1) as f64 && d <= i32::MAX as f64;
                Ok((d, is_int))
            }
            R0Val::Range { start, .. } => {
                if *start == NA_INT {
                    return Err(format!("'{arg_name}' cannot be NA"));
                }
                Ok((*start as f64, true))
            }
            _ => Err(format!("TypeError: '{arg_name}' must be numeric")),
        }
    }

    fn rep_impl(
        &mut self,
        func: &str,
        x: R0Val,
        times_opt: Option<R0Val>,
        len_out_opt: Option<R0Val>,
        each_opt: Option<R0Val>,
    ) -> Result<R0Val, String> {
        match &x {
            R0Val::Dbl(_) | R0Val::Int(_) | R0Val::Range { .. } | R0Val::Lgl(_) | R0Val::Chr(_) | R0Val::Null => {}
            _ => return Err(format!("TypeError: {func} requires an atomic vector or range")),
        }

        let each = if let Some(e) = each_opt {
            Self::parse_non_negative_scalar(&e, "each")?
        } else {
            1
        };

        let length_out = if let Some(lo) = len_out_opt {
            if matches!(lo, R0Val::Null) {
                None
            } else {
                Some(Self::parse_non_negative_scalar(&lo, "length.out")?)
            }
        } else {
            None
        };

        if let Some(out_len) = length_out {
            if out_len > 0 && x.is_empty() {
                return Err(format!("cannot replicate empty vector with positive length.out in {func}"));
            }
            if each == 0 && out_len > 0 {
                return Err("invalid 'each' argument".into());
            }
            self.consume_fuel(out_len)?;
            if out_len == 0 {
                return match x {
                    R0Val::Dbl(_) => Ok(R0Val::Dbl(Vec::new())),
                    R0Val::Int(_) | R0Val::Range { .. } => Ok(R0Val::Int(Vec::new())),
                    R0Val::Lgl(_) => Ok(R0Val::Lgl(Vec::new())),
                    R0Val::Chr(_) => Ok(R0Val::Chr(Vec::new())),
                    R0Val::Null => Ok(R0Val::Null),
                    _ => unreachable!(),
                };
            }
            return match x {
                R0Val::Dbl(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for i in 0..out_len {
                        out.push(v[(i / each) % v.len()]);
                    }
                    Ok(R0Val::Dbl(out))
                }
                R0Val::Int(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for i in 0..out_len {
                        out.push(v[(i / each) % v.len()]);
                    }
                    Ok(R0Val::Int(out))
                }
                R0Val::Range { start, end: _, step } => {
                    let mut out = Vec::with_capacity(out_len);
                    let rlen = x.len();
                    for i in 0..out_len {
                        let k = (i / each) % rlen;
                        out.push((start as i64 + k as i64 * step as i64) as i32);
                    }
                    Ok(R0Val::Int(out))
                }
                R0Val::Lgl(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for i in 0..out_len {
                        out.push(v[(i / each) % v.len()]);
                    }
                    Ok(R0Val::Lgl(out))
                }
                R0Val::Chr(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for i in 0..out_len {
                        out.push(v[(i / each) % v.len()].clone());
                    }
                    Ok(R0Val::Chr(out))
                }
                R0Val::Null => Ok(R0Val::Null),
                _ => unreachable!(),
            };
        }

        let times_val = times_opt.unwrap_or_else(|| R0Val::Int(vec![1]));
        if times_val.len() > 1 {
            if times_val.len() != x.len() {
                return Err("invalid 'times' argument".into());
            }
            if each != 1 {
                return Err("invalid 'times' argument, given the value of 'each'".into());
            }
            let counts: Vec<usize> = match &times_val {
                R0Val::Int(v) => {
                    let mut res = Vec::with_capacity(v.len());
                    for &n in v {
                        if n == NA_INT || n < 0 {
                            return Err("invalid 'times' argument".into());
                        }
                        res.push(n as usize);
                    }
                    res
                }
                R0Val::Dbl(v) => {
                    let mut res = Vec::with_capacity(v.len());
                    for &d in v {
                        if d.is_nan() || d.is_infinite() || d < 0.0 {
                            return Err("invalid 'times' argument".into());
                        }
                        res.push(d as usize);
                    }
                    res
                }
                R0Val::Range { start, end: _, step } => {
                    let n = times_val.len();
                    let mut res = Vec::with_capacity(n);
                    for i in 0..n {
                        let val = (*start as i64 + i as i64 * *step as i64) as i32;
                        if val < 0 {
                            return Err("invalid 'times' argument".into());
                        }
                        res.push(val as usize);
                    }
                    res
                }
                _ => return Err("invalid 'times' argument".into()),
            };

            let out_len: usize = counts.iter().sum();
            self.consume_fuel(out_len)?;
            if out_len == 0 {
                return match x {
                    R0Val::Dbl(_) => Ok(R0Val::Dbl(Vec::new())),
                    R0Val::Int(_) | R0Val::Range { .. } => Ok(R0Val::Int(Vec::new())),
                    R0Val::Lgl(_) => Ok(R0Val::Lgl(Vec::new())),
                    R0Val::Chr(_) => Ok(R0Val::Chr(Vec::new())),
                    R0Val::Null => Ok(R0Val::Null),
                    _ => unreachable!(),
                };
            }

            return match x {
                R0Val::Dbl(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for (&val, &cnt) in v.iter().zip(&counts) {
                        for _ in 0..cnt {
                            out.push(val);
                        }
                    }
                    Ok(R0Val::Dbl(out))
                }
                R0Val::Int(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for (&val, &cnt) in v.iter().zip(&counts) {
                        for _ in 0..cnt {
                            out.push(val);
                        }
                    }
                    Ok(R0Val::Int(out))
                }
                R0Val::Range { start, end: _, step } => {
                    let mut out = Vec::with_capacity(out_len);
                    let rlen = x.len();
                    for (i, &cnt) in (0..rlen).zip(&counts) {
                        let val = (start as i64 + i as i64 * step as i64) as i32;
                        for _ in 0..cnt {
                            out.push(val);
                        }
                    }
                    Ok(R0Val::Int(out))
                }
                R0Val::Lgl(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for (&val, &cnt) in v.iter().zip(&counts) {
                        for _ in 0..cnt {
                            out.push(val);
                        }
                    }
                    Ok(R0Val::Lgl(out))
                }
                R0Val::Chr(v) => {
                    let mut out = Vec::with_capacity(out_len);
                    for (val, &cnt) in v.iter().zip(&counts) {
                        for _ in 0..cnt {
                            out.push(val.clone());
                        }
                    }
                    Ok(R0Val::Chr(out))
                }
                R0Val::Null => Ok(R0Val::Null),
                _ => unreachable!(),
            };
        }

        let times = Self::parse_non_negative_scalar(&times_val, "times")?;
        let out_len = x.len().saturating_mul(each).saturating_mul(times);
        self.consume_fuel(out_len)?;
        if out_len == 0 {
            return match x {
                R0Val::Dbl(_) => Ok(R0Val::Dbl(Vec::new())),
                R0Val::Int(_) | R0Val::Range { .. } => Ok(R0Val::Int(Vec::new())),
                R0Val::Lgl(_) => Ok(R0Val::Lgl(Vec::new())),
                R0Val::Chr(_) => Ok(R0Val::Chr(Vec::new())),
                R0Val::Null => Ok(R0Val::Null),
                _ => unreachable!(),
            };
        }

        match x {
            R0Val::Dbl(v) => {
                let mut out = Vec::with_capacity(out_len);
                for i in 0..out_len {
                    out.push(v[(i / each) % v.len()]);
                }
                Ok(R0Val::Dbl(out))
            }
            R0Val::Int(v) => {
                let mut out = Vec::with_capacity(out_len);
                for i in 0..out_len {
                    out.push(v[(i / each) % v.len()]);
                }
                Ok(R0Val::Int(out))
            }
            R0Val::Range { start, end: _, step } => {
                let mut out = Vec::with_capacity(out_len);
                let rlen = x.len();
                for i in 0..out_len {
                    let k = (i / each) % rlen;
                    out.push((start as i64 + k as i64 * step as i64) as i32);
                }
                Ok(R0Val::Int(out))
            }
            R0Val::Lgl(v) => {
                let mut out = Vec::with_capacity(out_len);
                for i in 0..out_len {
                    out.push(v[(i / each) % v.len()]);
                }
                Ok(R0Val::Lgl(out))
            }
            R0Val::Chr(v) => {
                let mut out = Vec::with_capacity(out_len);
                for i in 0..out_len {
                    out.push(v[(i / each) % v.len()].clone());
                }
                Ok(R0Val::Chr(out))
            }
            R0Val::Null => Ok(R0Val::Null),
            _ => unreachable!(),
        }
    }

    fn seq_impl(
        &mut self,
        from_opt: Option<R0Val>,
        to_opt: Option<R0Val>,
        by_opt: Option<R0Val>,
        len_out_opt: Option<R0Val>,
        along_with_opt: Option<R0Val>,
    ) -> Result<R0Val, String> {
        let mut length_out = len_out_opt;
        if let Some(along) = along_with_opt {
            if length_out.is_some() {
                return Err("cannot specify both 'length.out' and 'along.with'".into());
            }
            if from_opt.is_none() && to_opt.is_none() && by_opt.is_none() {
                let n = along.len();
                self.consume_fuel(n)?;
                if n == 0 {
                    return Ok(R0Val::Int(Vec::new()));
                }
                return Ok(R0Val::Range {
                    start: 1,
                    end: n as i32,
                    step: 1,
                });
            }
            length_out = Some(R0Val::Int(vec![along.len() as i32]));
        }

        if from_opt.is_none() && to_opt.is_none() && by_opt.is_none() && length_out.is_none() {
            self.consume_fuel(1)?;
            return Ok(R0Val::Range {
                start: 1,
                end: 1,
                step: 1,
            });
        }

        if to_opt.is_none() && by_opt.is_none() && length_out.is_none() {
            if let Some(from) = from_opt {
                if from.is_empty() {
                    self.consume_fuel(0)?;
                    return Ok(R0Val::Int(Vec::new()));
                }
                if from.len() > 1 {
                    let n = from.len();
                    self.consume_fuel(n)?;
                    return Ok(R0Val::Range {
                        start: 1,
                        end: n as i32,
                        step: 1,
                    });
                }
                return match &from {
                R0Val::Int(v) => {
                    let n = v[0];
                    if n == NA_INT {
                        return Err("seq requires finite non-NA argument".into());
                    }
                    if n >= 1 {
                        self.consume_fuel(n as usize)?;
                        Ok(R0Val::Range {
                            start: 1,
                            end: n,
                            step: 1,
                        })
                    } else {
                        let count = (1 - n as i64 + 1) as usize;
                        self.consume_fuel(count)?;
                        Ok(R0Val::Range {
                            start: 1,
                            end: n,
                            step: -1,
                        })
                    }
                }
                R0Val::Dbl(v) => {
                    let d = v[0];
                    if d.is_nan() || d.is_infinite() {
                        return Err("seq requires finite non-NA argument".into());
                    }
                    let n = d as i32;
                    if n >= 1 {
                        self.consume_fuel(n as usize)?;
                        Ok(R0Val::Range {
                            start: 1,
                            end: n,
                            step: 1,
                        })
                    } else {
                        let count = (1 - n as i64 + 1) as usize;
                        self.consume_fuel(count)?;
                        Ok(R0Val::Range {
                            start: 1,
                            end: n,
                            step: -1,
                        })
                    }
                }
                R0Val::Range { start, .. } => {
                    let n = *start;
                    if n == NA_INT {
                        return Err("seq requires finite non-NA argument".into());
                    }
                    if n >= 1 {
                        self.consume_fuel(n as usize)?;
                        Ok(R0Val::Range {
                            start: 1,
                            end: n,
                            step: 1,
                        })
                    } else {
                        let count = (1 - n as i64 + 1) as usize;
                        self.consume_fuel(count)?;
                        Ok(R0Val::Range {
                            start: 1,
                            end: n,
                            step: -1,
                        })
                    }
                }
                _ => Err("TypeError: seq requires a numeric argument".into()),
            };
            }
        }

        if from_opt.is_some() && to_opt.is_some() && by_opt.is_some() && length_out.is_some() {
            return Err("too many arguments".into());
        }

        if let Some(lo_val) = length_out {
            let lo = Self::parse_non_negative_scalar(&lo_val, "length.out")?;
            if lo == 0 {
                self.consume_fuel(0)?;
                return Ok(R0Val::Dbl(Vec::new()));
            }
            if lo == 1 {
                self.consume_fuel(1)?;
                if let Some(f) = from_opt {
                    let (fv, _) = Self::parse_scalar_num(&f, "from")?;
                    if matches!(f, R0Val::Int(_) | R0Val::Range { .. }) {
                        return Ok(R0Val::Int(vec![fv as i32]));
                    } else {
                        return Ok(R0Val::Dbl(vec![fv]));
                    }
                } else {
                    return Ok(R0Val::Dbl(vec![1.0]));
                }
            }
            if let Some(by_val) = by_opt {
                let (by_v, _) = Self::parse_scalar_num(&by_val, "by")?;
                let from_v = if let Some(f) = from_opt {
                    Self::parse_scalar_num(&f, "from")?.0
                } else {
                    1.0
                };
                self.consume_fuel(lo)?;
                let mut out = Vec::with_capacity(lo);
                for i in 0..lo {
                    out.push(from_v + i as f64 * by_v);
                }
                return Ok(R0Val::Dbl(out));
            } else if let Some(to_val) = to_opt {
                let (to_v, _) = Self::parse_scalar_num(&to_val, "to")?;
                let from_v = if let Some(f) = from_opt {
                    Self::parse_scalar_num(&f, "from")?.0
                } else {
                    1.0
                };
                let step = (to_v - from_v) / (lo as f64 - 1.0);
                self.consume_fuel(lo)?;
                let mut out = Vec::with_capacity(lo);
                for i in 0..lo {
                    out.push(from_v + i as f64 * step);
                }
                return Ok(R0Val::Dbl(out));
            } else {
                let from_v = if let Some(f) = from_opt {
                    Self::parse_scalar_num(&f, "from")?.0
                } else {
                    1.0
                };
                self.consume_fuel(lo)?;
                let mut out = Vec::with_capacity(lo);
                for i in 0..lo {
                    out.push(from_v + i as f64);
                }
                return Ok(R0Val::Dbl(out));
            }
        }

        if let Some(by_val) = by_opt {
            let is_by_int = matches!(by_val, R0Val::Int(_) | R0Val::Range { .. });
            let (by_v, _) = Self::parse_scalar_num(&by_val, "by")?;
            if by_v == 0.0 {
                return Err("invalid '(to - from)/by'".into());
            }
            let (from_v, is_from_int) = if let Some(f) = from_opt {
                Self::parse_scalar_num(&f, "from")?
            } else {
                (1.0, true)
            };
            let (to_v, is_to_int) = if let Some(t) = to_opt {
                Self::parse_scalar_num(&t, "to")?
            } else {
                (1.0, true)
            };
            if (from_v < to_v && by_v < 0.0) || (from_v > to_v && by_v > 0.0) {
                return Err("wrong sign in 'by' argument".into());
            }
            let count = (((to_v - from_v) / by_v) + 1e-10).floor() as i64 + 1;
            if count <= 0 {
                self.consume_fuel(0)?;
                return Ok(R0Val::Dbl(Vec::new()));
            }
            let count = count as usize;
            self.consume_fuel(count)?;
            if is_from_int && is_to_int && is_by_int {
                let mut out = Vec::with_capacity(count);
                for i in 0..count {
                    out.push((from_v + i as f64 * by_v) as i32);
                }
                return Ok(R0Val::Int(out));
            } else {
                let mut out = Vec::with_capacity(count);
                for i in 0..count {
                    out.push(from_v + i as f64 * by_v);
                }
                return Ok(R0Val::Dbl(out));
            }
        }

        let (from_v, is_from_int) = if let Some(f) = from_opt {
            Self::parse_scalar_num(&f, "from")?
        } else {
            (1.0, true)
        };
        let (to_v, is_to_int) = if let Some(t) = to_opt {
            Self::parse_scalar_num(&t, "to")?
        } else {
            (1.0, true)
        };

        if is_from_int && is_to_int {
            let start = from_v as i32;
            let end = to_v as i32;
            let step = if start <= end { 1 } else { -1 };
            let count = (end as i64 - start as i64).unsigned_abs() as usize + 1;
            self.consume_fuel(count)?;
            Ok(R0Val::Range { start, end, step })
        } else {
            let step = if from_v <= to_v { 1.0 } else { -1.0 };
            let count = (to_v - from_v).abs().floor() as usize + 1;
            self.consume_fuel(count)?;
            let mut out = Vec::with_capacity(count);
            for i in 0..count {
                out.push(from_v + i as f64 * step);
            }
            Ok(R0Val::Dbl(out))
        }
    }

    /// Common-type concatenation (D3). Lgl or Chr mixed with anything else is a TypeError.
    fn common_concat(&mut self, vals: &[R0Val]) -> Result<R0Val, String> {
        self.consume_fuel(vals.iter().map(R0Val::len).fold(0, usize::saturating_add))?;
        let (mut dbl, mut int, mut lgl, mut lgl_val, mut chr) = (false, false, false, false, false);
        for v in vals {
            match v {
                R0Val::Dbl(_) => dbl = true,
                R0Val::Int(_) | R0Val::Range { .. } => int = true,
                R0Val::Lgl(b) => {
                    lgl = true;
                    lgl_val |= b.iter().any(|&x| x != LGL_NA);
                }
                R0Val::Chr(_) => chr = true,
                R0Val::Null => {}
                _ => return Err("TypeError: c() cannot combine non-atomic values".into()),
            }
        }
        let numeric = dbl || int;
        // E14: an all-NA Lgl promotes to the numeric target; non-NA Lgl mixed with numbers errors.
        if (lgl_val && chr) || (lgl_val && numeric) || (chr && numeric) {
            return Err("TypeError: c() cannot mix logical, character, and numeric values".into());
        }
        if chr {
            // B5: an all-NA logical argument becomes NA_character_ elements.
            let mut out: Vec<Option<String>> = Vec::new();
            for v in vals {
                match v {
                    R0Val::Chr(s) => out.extend(s.iter().cloned()),
                    R0Val::Lgl(b) => out.extend(std::iter::repeat_n(None, b.len())),
                    _ => {}
                }
            }
            return Ok(R0Val::Chr(out));
        }
        if lgl && !numeric {
            let mut out = Vec::new();
            for v in vals {
                if let R0Val::Lgl(b) = v {
                    out.extend(b.iter().copied());
                }
            }
            return Ok(R0Val::Lgl(out));
        }
        if dbl {
            let mut out = Vec::new();
            for v in vals {
                match v {
                    R0Val::Lgl(b) if b.iter().all(|&x| x == LGL_NA) => {
                        out.extend(std::iter::repeat_n(na_real(), b.len()))
                    }
                    _ => out.extend(self.promote_dbl(v)?),
                }
            }
            return Ok(R0Val::Dbl(out));
        }
        if int {
            let mut out = Vec::new();
            for v in vals {
                match v {
                    R0Val::Lgl(b) if b.iter().all(|&x| x == LGL_NA) => {
                        out.extend(std::iter::repeat_n(NA_INT, b.len()))
                    }
                    _ => out.extend(self.materialize_int(v)?),
                }
            }
            return Ok(R0Val::Int(out));
        }
        Ok(R0Val::Null)
    }

    /// Numeric view of an atomic value; Lgl maps to 0/1 with NA as NA_real.
    fn as_f64(&mut self, v: &R0Val) -> Result<Vec<f64>, String> {
        match v {
            R0Val::Lgl(b) => Ok(b
                .iter()
                .map(|&x| match x {
                    LGL_TRUE => 1.0,
                    LGL_FALSE => 0.0,
                    _ => na_real(),
                })
                .collect()),
            other => self.promote_dbl(other),
        }
    }

    /// sum/mean/min/max. sum and min/max keep Int for Int/Lgl/Range input (D19).
    fn reduce(&mut self, func: &str, x: &R0Val, na_rm: bool) -> Result<R0Val, String> {
        self.consume_fuel(x.len())?;
        let is_int = matches!(x, R0Val::Int(_) | R0Val::Lgl(_) | R0Val::Range { .. });
        if is_int && func != "mean" {
            let ints: Vec<i32> = match x {
                R0Val::Lgl(b) => b.iter().map(|&v| if v == LGL_NA { NA_INT } else { v as i32 }).collect(),
                other => self.materialize_int(other)?,
            };
            let (mut acc, mut any, mut lo, mut hi) = (0i64, false, i32::MAX, i32::MIN);
            for v in ints {
                if v == NA_INT {
                    if na_rm {
                        continue;
                    }
                    return Ok(R0Val::Int(vec![NA_INT]));
                }
                any = true;
                acc += v as i64;
                lo = lo.min(v);
                hi = hi.max(v);
            }
            return match func {
                "sum" => i32::try_from(acc)
                    .map(|s| R0Val::Int(vec![s]))
                    .map_err(|_| "ErrIntOverflow: sum exceeds integer range".to_string()),
                _ if !any => Ok(R0Val::Dbl(vec![if func == "min" { f64::INFINITY } else { f64::NEG_INFINITY }])),
                "min" => Ok(R0Val::Int(vec![lo])),
                _ => Ok(R0Val::Int(vec![hi])),
            };
        }
        let xs = self.as_f64(x)?;
        let mut acc = match func {
            "min" => f64::INFINITY,
            "max" => f64::NEG_INFINITY,
            _ => 0.0,
        };
        let mut count = 0usize;
        // R mirror: min/max return NA if any NA is present, else NaN; sum/mean keep the first missing.
        let mut nan_seen: Option<f64> = None;
        for v in xs {
            if is_missing(v) {
                if !na_rm {
                    if matches!(func, "min" | "max") {
                        if is_na_real(v) {
                            return Ok(R0Val::Dbl(vec![v]));
                        }
                        nan_seen.get_or_insert(v);
                        continue;
                    }
                    return Ok(R0Val::Dbl(vec![v]));
                }
                continue;
            }
            count += 1;
            match func {
                "sum" | "mean" => acc += v,
                "min" => acc = acc.min(v),
                _ => acc = acc.max(v),
            }
        }
        if let Some(v) = nan_seen {
            return Ok(R0Val::Dbl(vec![v]));
        }
        if func == "mean" {
            acc = if count == 0 { na_real() } else { acc / count as f64 };
        }
        Ok(R0Val::Dbl(vec![acc]))
    }

    /// any/all logical reduction implementing GNU R 3-valued Kleene logic.
    fn reduce_logical(&mut self, func: &str, x: &R0Val, na_rm: bool) -> Result<R0Val, String> {
        self.consume_fuel(x.len())?;
        match x {
            R0Val::Lgl(_) | R0Val::Int(_) | R0Val::Range { .. } | R0Val::Dbl(_) | R0Val::Null => {}
            _ => return Err(format!("TypeError: invalid 'type' of argument in {}()", func)),
        }
        let is_any = func == "any";
        if x.is_empty() {
            return Ok(R0Val::Lgl(vec![if is_any { LGL_FALSE } else { LGL_TRUE }]));
        }
        let target = if is_any { LGL_TRUE } else { LGL_FALSE };
        let mut has_na = false;

        let mut check = |val: Lgl| -> bool {
            if val == target {
                true
            } else {
                if val == LGL_NA {
                    has_na = true;
                }
                false
            }
        };

        let hit = match x {
            R0Val::Lgl(v) => v.iter().any(|&b| check(b)),
            R0Val::Int(v) => v.iter().any(|&n| {
                let l = if n == NA_INT {
                    LGL_NA
                } else if n != 0 {
                    LGL_TRUE
                } else {
                    LGL_FALSE
                };
                check(l)
            }),
            R0Val::Range { start, end, step } => {
                let mut hit = false;
                let mut curr = *start;
                let ascending = *step > 0;
                loop {
                    if (ascending && curr > *end) || (!ascending && curr < *end) {
                        break;
                    }
                    let l = if curr != 0 { LGL_TRUE } else { LGL_FALSE };
                    if check(l) {
                        hit = true;
                        break;
                    }
                    match curr.checked_add(*step) {
                        Some(next) => curr = next,
                        None => break,
                    }
                }
                hit
            }
            R0Val::Dbl(v) => v.iter().any(|&d| {
                let l = if is_missing(d) {
                    LGL_NA
                } else if d != 0.0 {
                    LGL_TRUE
                } else {
                    LGL_FALSE
                };
                check(l)
            }),
            R0Val::Null => false,
            _ => unreachable!(),
        };

        if hit {
            Ok(R0Val::Lgl(vec![target]))
        } else if has_na && !na_rm {
            Ok(R0Val::Lgl(vec![LGL_NA]))
        } else {
            Ok(R0Val::Lgl(vec![if is_any { LGL_FALSE } else { LGL_TRUE }]))
        }
    }

    fn eval_prod(&mut self, x: &R0Val, na_rm: bool) -> Result<R0Val, String> {
        self.consume_fuel(x.len())?;
        if matches!(x, R0Val::Null) {
            return Ok(R0Val::Dbl(vec![1.0]));
        }
        let xs = self.as_f64(x)?;
        let mut acc = 1.0;
        for v in xs {
            if is_missing(v) {
                if !na_rm {
                    return Ok(R0Val::Dbl(vec![v]));
                }
                continue;
            }
            acc *= v;
        }
        Ok(R0Val::Dbl(vec![acc]))
    }

    fn eval_var_sd(&mut self, func: &str, x: &R0Val, na_rm: bool) -> Result<R0Val, String> {
        self.consume_fuel(x.len())?;
        if matches!(x, R0Val::Null) {
            return Ok(R0Val::Dbl(vec![na_real()]));
        }
        let xs = self.as_f64(x)?;
        let mut filtered = Vec::new();
        for v in xs {
            if is_missing(v) {
                if !na_rm {
                    return Ok(R0Val::Dbl(vec![v]));
                }
                continue;
            }
            filtered.push(v);
        }
        if filtered.len() < 2 {
            return Ok(R0Val::Dbl(vec![na_real()]));
        }
        let n = filtered.len() as f64;
        let mean = filtered.iter().sum::<f64>() / n;
        let var = filtered.iter().map(|&v| (v - mean).powi(2)).sum::<f64>() / (n - 1.0);
        if func == "var" {
            Ok(R0Val::Dbl(vec![var]))
        } else {
            Ok(R0Val::Dbl(vec![var.sqrt()]))
        }
    }

    fn eval_median(&mut self, x: &R0Val, na_rm: bool) -> Result<R0Val, String> {
        self.consume_fuel(x.len())?;
        if matches!(x, R0Val::Null) {
            return Ok(R0Val::Dbl(vec![na_real()]));
        }
        let xs = self.as_f64(x)?;
        let mut filtered = Vec::new();
        for v in xs {
            if is_missing(v) {
                if !na_rm {
                    return Ok(R0Val::Dbl(vec![v]));
                }
                continue;
            }
            filtered.push(v);
        }
        if filtered.is_empty() {
            return Ok(R0Val::Dbl(vec![na_real()]));
        }
        filtered.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
        let n = filtered.len();
        let med = if n % 2 == 1 {
            filtered[n / 2]
        } else {
            (filtered[(n / 2) - 1] + filtered[n / 2]) / 2.0
        };
        Ok(R0Val::Dbl(vec![med]))
    }

    fn call_intrinsic(&mut self, func: &str, args: Vec<(Option<String>, R0Val)>) -> Result<R0Val, String> {
        // Variadic intrinsics take their args as-is (names ignored for c, required for data.frame/list).
        match func {
            "c" => return self.common_concat(&args.into_iter().map(|(_, v)| v).collect::<Vec<_>>()),
            "data.frame" | "list" => {
                let mut names = Vec::new();
                let mut cols = Vec::new();
                for (k, v) in args {
                    names.push(k.ok_or_else(|| format!("{}() columns must be named (e.g. top = c(...))", func))?);
                    cols.push(v);
                }
                return if func == "data.frame" {
                    R0Val::df(names, cols)
                } else {
                    Ok(R0Val::List { names, cols })
                };
            }
            _ => {}
        }
        let (params, npos): (&[&str], usize) = match func {
            "approx" => (&["x", "y", "xout"], 3),
            "ifelse" => (&["test", "yes", "no"], 3),
            "pmin" | "pmax" => (&["x", "y", "na.rm"], 2),
            "sum" | "mean" | "min" | "max" | "any" | "all" | "prod" | "var" | "sd" | "median" => (&["x", "na.rm"], 1),
            "%in%" => (&["x", "table"], 2),
            "match" => (&["x", "table", "nomatch"], 3),
            "sort" | "order" => (&["x", "decreasing"], 2),
            "length" | "nrow" | "ncol" | "seq_len" | "seq_along" | "is.na" | "is.nan" | "is.finite" | "is.infinite" | "anyNA" | "which"
            | "which.min" | "which.max" | "unique" | "duplicated" | "rank"
            | "abs" | "sqrt" | "exp" | "log"
            | "log10" | "floor" | "ceiling" | "cumsum" | "diff"
            | "cumprod" | "cummin" | "cummax"
            | "sin" | "cos" | "tan" | "acos" | "asin" | "atan" | "trunc"
            | "as.numeric" | "as.integer" | "as.logical" => (&["x"], 1),
            "round" => (&["x", "digits"], 2),
            "atan2" => (&["y", "x"], 2),
            "numeric" | "integer" | "logical" | "character" => (&["length"], 1),
            "rep_len" => (&["x", "length.out"], 2),
            "rep" => (&["x", "times", "length.out", "each"], 4),
            "seq" => (&["from", "to", "by", "length.out", "along.with"], 4),
            _ => return Err(format!("Unknown function or intrinsic: '{}'", func)),
        };
        let mut s = Self::bind_args(func, args, params, npos)?;
        match func {
            "length" => Ok(R0Val::Int(vec![Self::req(&mut s, 0, func)?.len() as i32])),
            "seq_len" | "seq_along" => {
                let x = Self::req(&mut s, 0, func)?;
                let n = if func == "seq_len" {
                    let ints = self.materialize_int(&x)?;
                    if ints.len() != 1 || ints[0] == NA_INT || ints[0] < 0 {
                        return Err("seq_len requires a non-negative length-1 integer argument".into());
                    }
                    ints[0]
                } else {
                    i32::try_from(x.len()).map_err(|_| "ErrIntOverflow: seq_along length exceeds i32::MAX")?
                };
                if n == 0 {
                    Ok(R0Val::Int(Vec::new()))
                } else {
                    Ok(R0Val::Range { start: 1, end: n, step: 1 })
                }
            }
            "is.nan" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                match &x {
                    R0Val::Dbl(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if d.is_nan() && !is_na_real(d) { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Int(_) | R0Val::Range { .. } => Ok(R0Val::Lgl(vec![LGL_FALSE; x.len()])),
                    _ => Err("Unsupported type for is.nan()".into()),
                }
            }
            "is.na" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                match &x {
                    R0Val::Dbl(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if is_missing(d) { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Int(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if d == NA_INT { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Lgl(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if d == LGL_NA { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Chr(v) => Ok(R0Val::Lgl(v.iter().map(|s| if s.is_none() { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Range { .. } => Ok(R0Val::Lgl(vec![LGL_FALSE; x.len()])),
                    _ => Err("Unsupported type for is.na()".into()),
                }
            }
            "is.finite" | "is.infinite" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                let is_inf = func == "is.infinite";
                match &x {
                    R0Val::Dbl(v) => Ok(R0Val::Lgl(
                        v.iter()
                            .map(|&d| {
                                let b = if is_inf { d.is_infinite() } else { d.is_finite() };
                                if b { LGL_TRUE } else { LGL_FALSE }
                            })
                            .collect(),
                    )),
                    R0Val::Int(v) => Ok(R0Val::Lgl(
                        v.iter()
                            .map(|&n| {
                                let b = if is_inf { false } else { n != NA_INT };
                                if b { LGL_TRUE } else { LGL_FALSE }
                            })
                            .collect(),
                    )),
                    R0Val::Lgl(v) => Ok(R0Val::Lgl(
                        v.iter()
                            .map(|&val| {
                                let b = if is_inf { false } else { val != LGL_NA };
                                if b { LGL_TRUE } else { LGL_FALSE }
                            })
                            .collect(),
                    )),
                    R0Val::Range { .. } => Ok(R0Val::Lgl(vec![if is_inf { LGL_FALSE } else { LGL_TRUE }; x.len()])),
                    R0Val::Chr(v) => Ok(R0Val::Lgl(vec![LGL_FALSE; v.len()])),
                    R0Val::Null => Ok(R0Val::Lgl(Vec::new())),
                    _ => Err(format!("Unsupported type for {}()", func)),
                }
            }
            "anyNA" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                let has_na = match &x {
                    R0Val::Dbl(v) => v.iter().any(|&d| is_missing(d)),
                    R0Val::Int(v) => v.contains(&NA_INT),
                    R0Val::Lgl(v) => v.contains(&LGL_NA),
                    R0Val::Chr(v) => v.iter().any(|s| s.is_none()),
                    R0Val::Range { .. } | R0Val::Null => false,
                    _ => return Err(format!("Unsupported type for {}()", func)),
                };
                Ok(R0Val::Lgl(vec![if has_na { LGL_TRUE } else { LGL_FALSE }]))
            }
            "which" => {
                let mask = Self::req(&mut s, 0, func)?;
                self.consume_fuel(mask.len())?;
                match mask {
                    R0Val::Lgl(mask) => Ok(R0Val::Int(
                        mask.iter()
                            .enumerate()
                            .filter(|(_, b)| **b == LGL_TRUE)
                            .map(|(i, _)| (i + 1) as i32)
                            .collect(),
                    )),
                    _ => Err("which() requires a logical (lgl) vector".into()),
                }
            }
            "which.min" | "which.max" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                if matches!(x, R0Val::Null) || x.is_empty() {
                    return Ok(R0Val::Int(Vec::new()));
                }
                match &x {
                    R0Val::Chr(_) => Err(format!("TypeError: {} requires numeric or logical vector", func)),
                    R0Val::Range { step, .. } => {
                        let n = x.len() as i32;
                        let idx = if func == "which.min" {
                            if *step >= 0 { 1 } else { n }
                        } else {
                            if *step >= 0 { n } else { 1 }
                        };
                        Ok(R0Val::Int(vec![idx]))
                    }
                    _ => {
                        let vals = self.as_f64(&x)?;
                        let is_min = func == "which.min";
                        let mut best_idx: Option<usize> = None;
                        let mut best_val = 0.0;
                        for (i, &val) in vals.iter().enumerate() {
                            if is_missing(val) {
                                continue;
                            }
                            match best_idx {
                                None => {
                                    best_idx = Some(i + 1);
                                    best_val = val;
                                }
                                Some(_) => {
                                    if is_min {
                                        if val < best_val {
                                            best_val = val;
                                            best_idx = Some(i + 1);
                                        }
                                    } else {
                                        if val > best_val {
                                            best_val = val;
                                            best_idx = Some(i + 1);
                                        }
                                    }
                                }
                            }
                        }
                        match best_idx {
                            Some(idx) => Ok(R0Val::Int(vec![idx as i32])),
                            None => Ok(R0Val::Int(Vec::new())),
                        }
                    }
                }
            }
            "unique" | "duplicated" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                if matches!(x, R0Val::Null) {
                    return if func == "unique" {
                        Ok(R0Val::Null)
                    } else {
                        Ok(R0Val::Lgl(Vec::new()))
                    };
                }
                match x {
                    R0Val::Range { .. } => {
                        let n = x.len();
                        if func == "unique" {
                            Ok(x)
                        } else {
                            Ok(R0Val::Lgl(vec![LGL_FALSE; n]))
                        }
                    }
                    R0Val::Int(v) => {
                        let mut seen = std::collections::HashSet::new();
                        let mut dups = Vec::with_capacity(v.len());
                        let mut uniq = Vec::new();
                        for item in v {
                            let is_dup = !seen.insert(item);
                            dups.push(if is_dup { LGL_TRUE } else { LGL_FALSE });
                            if !is_dup {
                                uniq.push(item);
                            }
                        }
                        if func == "unique" {
                            Ok(R0Val::Int(uniq))
                        } else {
                            Ok(R0Val::Lgl(dups))
                        }
                    }
                    R0Val::Lgl(v) => {
                        let mut seen = [false; 3];
                        let mut dups = Vec::with_capacity(v.len());
                        let mut uniq = Vec::new();
                        for item in v {
                            let idx = (item as usize).min(2);
                            let is_dup = seen[idx];
                            seen[idx] = true;
                            dups.push(if is_dup { LGL_TRUE } else { LGL_FALSE });
                            if !is_dup {
                                uniq.push(item);
                            }
                        }
                        if func == "unique" {
                            Ok(R0Val::Lgl(uniq))
                        } else {
                            Ok(R0Val::Lgl(dups))
                        }
                    }
                    R0Val::Chr(v) => {
                        let mut seen = std::collections::HashSet::new();
                        let mut dups = Vec::with_capacity(v.len());
                        let mut uniq = Vec::new();
                        for item in v {
                            let is_dup = !seen.insert(item.clone());
                            dups.push(if is_dup { LGL_TRUE } else { LGL_FALSE });
                            if !is_dup {
                                uniq.push(item);
                            }
                        }
                        if func == "unique" {
                            Ok(R0Val::Chr(uniq))
                        } else {
                            Ok(R0Val::Lgl(dups))
                        }
                    }
                    R0Val::Dbl(v) => {
                        let mut seen = std::collections::HashSet::new();
                        let mut dups = Vec::with_capacity(v.len());
                        let mut uniq = Vec::new();
                        for item in v {
                            let key = if is_missing(item) {
                                if is_na_real(item) {
                                    u64::MAX - 1
                                } else {
                                    u64::MAX
                                }
                            } else if item == 0.0 {
                                0.0f64.to_bits()
                            } else {
                                item.to_bits()
                            };
                            let is_dup = !seen.insert(key);
                            dups.push(if is_dup { LGL_TRUE } else { LGL_FALSE });
                            if !is_dup {
                                uniq.push(item);
                            }
                        }
                        if func == "unique" {
                            Ok(R0Val::Dbl(uniq))
                        } else {
                            Ok(R0Val::Lgl(dups))
                        }
                    }
                    _ => Err(format!("Unsupported type for {}()", func)),
                }
            }
            "sort" => {
                let x = Self::req(&mut s, 0, func)?;
                let decreasing = Self::parse_decreasing(s[1].take(), func)?;
                let n = x.len();
                let fuel_units = if n <= 1 {
                    n
                } else {
                    let log2_n = (n as f64).log2().ceil() as usize;
                    n.saturating_mul(log2_n).max(n)
                };
                self.consume_fuel(fuel_units)?;
                if matches!(x, R0Val::Null) {
                    return Ok(R0Val::Null);
                }
                match x {
                    R0Val::Range { start, end, step } => {
                        if n == 0 {
                            return Ok(R0Val::Int(Vec::new()));
                        }
                        let (min_val, max_val) = if step >= 0 { (start, end) } else { (end, start) };
                        if decreasing {
                            Ok(R0Val::Range { start: max_val, end: min_val, step: -1 })
                        } else {
                            Ok(R0Val::Range { start: min_val, end: max_val, step: 1 })
                        }
                    }
                    R0Val::Int(v) => {
                        let mut non_na: Vec<i32> = v.into_iter().filter(|&val| val != NA_INT).collect();
                        if decreasing {
                            non_na.sort_by(|a, b| b.cmp(a));
                        } else {
                            non_na.sort();
                        }
                        Ok(R0Val::Int(non_na))
                    }
                    R0Val::Dbl(v) => {
                        let mut non_na: Vec<f64> = v.into_iter().filter(|&val| !is_missing(val)).collect();
                        if decreasing {
                            non_na.sort_by(|a, b| b.partial_cmp(a).unwrap_or(std::cmp::Ordering::Equal));
                        } else {
                            non_na.sort_by(|a, b| a.partial_cmp(b).unwrap_or(std::cmp::Ordering::Equal));
                        }
                        Ok(R0Val::Dbl(non_na))
                    }
                    R0Val::Lgl(v) => {
                        let mut non_na: Vec<Lgl> = v.into_iter().filter(|&val| val != LGL_NA).collect();
                        if decreasing {
                            non_na.sort_by(|a, b| b.cmp(a));
                        } else {
                            non_na.sort();
                        }
                        Ok(R0Val::Lgl(non_na))
                    }
                    R0Val::Chr(v) => {
                        let mut non_na: Vec<String> = v.into_iter().flatten().collect();
                        if decreasing {
                            non_na.sort_by(|a, b| b.cmp(a));
                        } else {
                            non_na.sort();
                        }
                        Ok(R0Val::Chr(non_na.into_iter().map(Some).collect()))
                    }
                    _ => Err(format!("TypeError: unsupported type for {}()", func)),
                }
            }
            "order" => {
                let x = Self::req(&mut s, 0, func)?;
                let decreasing = Self::parse_decreasing(s[1].take(), func)?;
                let n = x.len();
                let fuel_units = if n <= 1 {
                    n
                } else {
                    let log2_n = (n as f64).log2().ceil() as usize;
                    n.saturating_mul(log2_n).max(n)
                };
                self.consume_fuel(fuel_units)?;
                if matches!(x, R0Val::Null) || n == 0 {
                    return Ok(R0Val::Int(Vec::new()));
                }
                match x {
                    R0Val::Range { step, .. } => {
                        let indices: Vec<i32> = if (step >= 0 && !decreasing) || (step < 0 && decreasing) {
                            (1..=n as i32).collect()
                        } else {
                            (1..=n as i32).rev().collect()
                        };
                        Ok(R0Val::Int(indices))
                    }
                    R0Val::Int(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut na_indices = Vec::new();
                        for (i, val) in v.into_iter().enumerate() {
                            let idx = (i + 1) as i32;
                            if val == NA_INT {
                                na_indices.push(idx);
                            } else {
                                valid.push((idx, val));
                            }
                        }
                        if decreasing {
                            valid.sort_by(|a, b| b.1.cmp(&a.1));
                        } else {
                            valid.sort_by(|a, b| a.1.cmp(&b.1));
                        }
                        let mut out: Vec<i32> = valid.into_iter().map(|(idx, _)| idx).collect();
                        out.extend(na_indices);
                        Ok(R0Val::Int(out))
                    }
                    R0Val::Dbl(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut na_indices = Vec::new();
                        for (i, val) in v.into_iter().enumerate() {
                            let idx = (i + 1) as i32;
                            if is_missing(val) {
                                na_indices.push(idx);
                            } else {
                                valid.push((idx, val));
                            }
                        }
                        if decreasing {
                            valid.sort_by(|a, b| b.1.partial_cmp(&a.1).unwrap_or(std::cmp::Ordering::Equal));
                        } else {
                            valid.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                        }
                        let mut out: Vec<i32> = valid.into_iter().map(|(idx, _)| idx).collect();
                        out.extend(na_indices);
                        Ok(R0Val::Int(out))
                    }
                    R0Val::Lgl(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut na_indices = Vec::new();
                        for (i, val) in v.into_iter().enumerate() {
                            let idx = (i + 1) as i32;
                            if val == LGL_NA {
                                na_indices.push(idx);
                            } else {
                                valid.push((idx, val));
                            }
                        }
                        if decreasing {
                            valid.sort_by(|a, b| b.1.cmp(&a.1));
                        } else {
                            valid.sort_by(|a, b| a.1.cmp(&b.1));
                        }
                        let mut out: Vec<i32> = valid.into_iter().map(|(idx, _)| idx).collect();
                        out.extend(na_indices);
                        Ok(R0Val::Int(out))
                    }
                    R0Val::Chr(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut na_indices = Vec::new();
                        for (i, opt_s) in v.into_iter().enumerate() {
                            let idx = (i + 1) as i32;
                            match opt_s {
                                Some(s) => valid.push((idx, s)),
                                None => na_indices.push(idx),
                            }
                        }
                        if decreasing {
                            valid.sort_by(|a, b| b.1.cmp(&a.1));
                        } else {
                            valid.sort_by(|a, b| a.1.cmp(&b.1));
                        }
                        let mut out: Vec<i32> = valid.into_iter().map(|(idx, _)| idx).collect();
                        out.extend(na_indices);
                        Ok(R0Val::Int(out))
                    }
                    _ => Err(format!("TypeError: unsupported type for {}()", func)),
                }
            }
            "rank" => {
                let x = Self::req(&mut s, 0, func)?;
                let n = x.len();
                let fuel_units = if n <= 1 {
                    n
                } else {
                    let log2_n = (n as f64).log2().ceil() as usize;
                    n.saturating_mul(log2_n).max(n)
                };
                self.consume_fuel(fuel_units)?;
                if matches!(x, R0Val::Null) || n == 0 {
                    return Ok(R0Val::Dbl(Vec::new()));
                }
                match x {
                    R0Val::Range { step, .. } => {
                        let ranks: Vec<f64> = if step >= 0 {
                            (1..=n).map(|i| i as f64).collect()
                        } else {
                            (1..=n).rev().map(|i| i as f64).collect()
                        };
                        Ok(R0Val::Dbl(ranks))
                    }
                    R0Val::Int(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut missing = Vec::new();
                        for (i, val) in v.into_iter().enumerate() {
                            if val == NA_INT {
                                missing.push(i);
                            } else {
                                valid.push((i, val));
                            }
                        }
                        valid.sort_by(|a, b| a.1.cmp(&b.1));
                        let ranks = Self::compute_sample_ranks(n, valid, missing, |a, b| a == b);
                        Ok(R0Val::Dbl(ranks))
                    }
                    R0Val::Dbl(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut missing = Vec::new();
                        for (i, val) in v.into_iter().enumerate() {
                            if is_missing(val) {
                                missing.push(i);
                            } else {
                                valid.push((i, val));
                            }
                        }
                        valid.sort_by(|a, b| a.1.partial_cmp(&b.1).unwrap_or(std::cmp::Ordering::Equal));
                        let ranks = Self::compute_sample_ranks(n, valid, missing, |a, b| a == b);
                        Ok(R0Val::Dbl(ranks))
                    }
                    R0Val::Lgl(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut missing = Vec::new();
                        for (i, val) in v.into_iter().enumerate() {
                            if val == LGL_NA {
                                missing.push(i);
                            } else {
                                valid.push((i, val));
                            }
                        }
                        valid.sort_by(|a, b| a.1.cmp(&b.1));
                        let ranks = Self::compute_sample_ranks(n, valid, missing, |a, b| a == b);
                        Ok(R0Val::Dbl(ranks))
                    }
                    R0Val::Chr(v) => {
                        let mut valid = Vec::with_capacity(n);
                        let mut missing = Vec::new();
                        for (i, opt_s) in v.into_iter().enumerate() {
                            match opt_s {
                                Some(s) => valid.push((i, s)),
                                None => missing.push(i),
                            }
                        }
                        valid.sort_by(|a, b| a.1.cmp(&b.1));
                        let ranks = Self::compute_sample_ranks(n, valid, missing, |a, b| a == b);
                        Ok(R0Val::Dbl(ranks))
                    }
                    _ => Err(format!("TypeError: unsupported type for {}()", func)),
                }
            }
            "ifelse" => {
                let test = Self::req(&mut s, 0, func)?;
                let yes = Self::req(&mut s, 1, func)?;
                let no = Self::req(&mut s, 2, func)?;
                let R0Val::Lgl(cond) = test else {
                    return Err("ifelse first argument must be logical (lgl)".into());
                };
                let n = cond.len();
                self.consume_fuel(n)?;
                for len in [yes.len(), no.len()] {
                    if n > 0 && len != 1 && len != n {
                        return Err(format!("ErrDimensionMismatch: ifelse branch length {} vs test length {}", len, n));
                    }
                }
                let (yl, nl) = (yes.len(), no.len());
                let both = self.common_concat(&[yes, no])?;
                let picks: Vec<Option<usize>> = cond
                    .iter()
                    .enumerate()
                    .map(|(i, &c)| match c {
                        LGL_TRUE => Some(if yl == 1 { 0 } else { i }),
                        LGL_FALSE => Some(yl + if nl == 1 { 0 } else { i }),
                        _ => None,
                    })
                    .collect();
                Ok(Self::gather(&both, &picks))
            }
            "pmin" | "pmax" => {
                let av = Self::req(&mut s, 0, func)?;
                let bv = Self::req(&mut s, 1, func)?;
                self.consume_fuel(av.len().max(bv.len()))?;
                let na_rm = Self::parse_na_rm(s[2].take(), func)?;
                if let (R0Val::Int(a), R0Val::Int(b)) = (&av, &bv) {
                    let n = Self::broadcast_len(a.len(), b.len())?;
                    self.consume_fuel(n)?;
                    let is_max = func == "pmax";
                    let mut out = Vec::with_capacity(n);
                    for i in 0..n {
                        let va = a[if a.len() == 1 { 0 } else { i }];
                        let vb = b[if b.len() == 1 { 0 } else { i }];
                        out.push(match (va == NA_INT, vb == NA_INT) {
                            (true, true) => NA_INT,
                            (true, false) => if na_rm { vb } else { va },
                            (false, true) => if na_rm { va } else { vb },
                            (false, false) => if is_max { va.max(vb) } else { va.min(vb) },
                        });
                    }
                    return Ok(R0Val::Int(out));
                }
                let a = self.as_f64(&av)?;
                let b = self.as_f64(&bv)?;
                let n = Self::broadcast_len(a.len(), b.len())?;
                let is_max = func == "pmax";
                let mut out = Vec::with_capacity(n);
                for i in 0..n {
                    let va = a[if a.len() == 1 { 0 } else { i }];
                    let vb = b[if b.len() == 1 { 0 } else { i }];
                    out.push(match (is_missing(va), is_missing(vb)) {
                        // R mirror: both missing -> second operand; single missing -> that operand.
                        (true, true) => if na_rm { na_real() } else { vb },
                        (true, false) => if na_rm { vb } else { va },
                        (false, true) => if na_rm { va } else { vb },
                        (false, false) => if is_max { va.max(vb) } else { va.min(vb) },
                    });
                }
                Ok(R0Val::Dbl(out))
            }
            "abs" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                match x {
                    // Int in, Int out (R behavior). NA_INT is i32::MIN, so abs never overflows.
                    R0Val::Int(v) => Ok(R0Val::Int(
                        v.iter()
                            .map(|&x| if x == NA_INT { NA_INT } else { x.abs() })
                            .collect(),
                    )),
                    x => Ok(R0Val::Dbl(
                        self.as_f64(&x)?
                            .into_iter()
                            .map(|v| if is_missing(v) { v } else { v.abs() })
                            .collect(),
                    )),
                }
            }
            "sqrt" | "exp" | "log" | "log10" | "floor" | "ceiling" => {
                let value = Self::req(&mut s, 0, func)?;
                self.consume_fuel(value.len())?;
                let x = value.promote_dbl()?;
                let out = x
                    .into_iter()
                    .map(|v| {
                        if is_missing(v) {
                            return na_real();
                        }
                        match func {
                            "sqrt" => v.sqrt(),
                            "exp" => v.exp(),
                            "log" => v.ln(),
                            "log10" => v.log10(),
                            "floor" => v.floor(),
                            _ => v.ceil(),
                        }
                    })
                    .collect();
                Ok(R0Val::Dbl(out))
            }
            "sin" | "cos" | "tan" | "acos" | "asin" | "atan" | "trunc" => {
                let value = Self::req(&mut s, 0, func)?;
                self.consume_fuel(value.len())?;
                if matches!(value, R0Val::Null) {
                    return Ok(R0Val::Dbl(Vec::new()));
                }
                let x = value.promote_dbl()?;
                let out = x
                    .into_iter()
                    .map(|v| {
                        if is_missing(v) {
                            v
                        } else {
                            match func {
                                "sin" => v.sin(),
                                "cos" => v.cos(),
                                "tan" => v.tan(),
                                "acos" => v.acos(),
                                "asin" => v.asin(),
                                "atan" => v.atan(),
                                "trunc" => v.trunc(),
                                _ => unreachable!(),
                            }
                        }
                    })
                    .collect();
                Ok(R0Val::Dbl(out))
            }
            "round" => {
                let x_val = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x_val.len())?;
                if matches!(x_val, R0Val::Null) {
                    return Ok(R0Val::Dbl(Vec::new()));
                }
                let digits = Self::parse_round_digits(s[1].take())?;
                let x = x_val.promote_dbl()?;
                let out = x.into_iter().map(|v| Self::round_val(v, digits)).collect();
                Ok(R0Val::Dbl(out))
            }
            "atan2" => {
                let y_val = Self::req(&mut s, 0, func)?;
                let x_val = Self::req(&mut s, 1, func)?;
                let ny = y_val.len();
                let nx = x_val.len();
                if matches!(y_val, R0Val::Null) || matches!(x_val, R0Val::Null) || ny == 0 || nx == 0 {
                    return Ok(R0Val::Dbl(Vec::new()));
                }
                let n = ny.max(nx);
                if (ny > nx && ny % nx != 0) || (nx > ny && nx % ny != 0) {
                    return Err(format!(
                        "ErrDimensionMismatch: longer object length {} is not a multiple of shorter object length {} in atan2",
                        n,
                        ny.min(nx)
                    ));
                }
                self.consume_fuel(n)?;
                let y = y_val.promote_dbl()?;
                let x = x_val.promote_dbl()?;
                let mut out = Vec::with_capacity(n);
                for i in 0..n {
                    let vy = y[i % ny];
                    let vx = x[i % nx];
                    let res = if is_missing(vy) {
                        vy
                    } else if is_missing(vx) {
                        vx
                    } else {
                        vy.atan2(vx)
                    };
                    out.push(res);
                }
                Ok(R0Val::Dbl(out))
            }
            "sum" | "mean" | "min" | "max" => {
                let x = Self::req(&mut s, 0, func)?;
                let na_rm = Self::parse_na_rm(s[1].take(), func)?;
                self.reduce(func, &x, na_rm)
            }
            "prod" => {
                let x = Self::req(&mut s, 0, func)?;
                let na_rm = Self::parse_na_rm(s[1].take(), func)?;
                self.eval_prod(&x, na_rm)
            }
            "var" | "sd" => {
                let x = Self::req(&mut s, 0, func)?;
                let na_rm = Self::parse_na_rm(s[1].take(), func)?;
                self.eval_var_sd(func, &x, na_rm)
            }
            "median" => {
                let x = Self::req(&mut s, 0, func)?;
                let na_rm = Self::parse_na_rm(s[1].take(), func)?;
                self.eval_median(&x, na_rm)
            }
            "any" | "all" => {
                let x = Self::req(&mut s, 0, func)?;
                let na_rm = Self::parse_na_rm(s[1].take(), func)?;
                self.reduce_logical(func, &x, na_rm)
            }
            "cumsum" => {
                let value = Self::req(&mut s, 0, func)?;
                self.consume_fuel(value.len())?;
                let x = value.promote_dbl()?;
                let mut run = 0.0;
                Ok(R0Val::Dbl(
                    x.into_iter()
                        .map(|v| {
                            run = if is_missing(run) {
                                run
                            } else if is_missing(v) {
                                v
                            } else {
                                run + v
                            };
                            run
                        })
                        .collect(),
                ))
            }
            "cumprod" => {
                let value = Self::req(&mut s, 0, func)?;
                self.consume_fuel(value.len())?;
                if matches!(value, R0Val::Null) {
                    return Ok(R0Val::Dbl(Vec::new()));
                }
                let x = self.as_f64(&value)?;
                let mut run = 1.0;
                Ok(R0Val::Dbl(
                    x.into_iter()
                        .map(|v| {
                            run = if is_missing(run) {
                                run
                            } else if is_missing(v) {
                                v
                            } else {
                                run * v
                            };
                            run
                        })
                        .collect(),
                ))
            }
            "cummin" | "cummax" => {
                let value = Self::req(&mut s, 0, func)?;
                self.consume_fuel(value.len())?;
                if matches!(value, R0Val::Null) {
                    return Ok(R0Val::Dbl(Vec::new()));
                }
                let x = self.as_f64(&value)?;
                let mut run: Option<f64> = None;
                Ok(R0Val::Dbl(
                    x.into_iter()
                        .map(|v| {
                            let next = match run {
                                Some(r) => {
                                    if is_missing(r) {
                                        r
                                    } else if is_missing(v) {
                                        v
                                    } else if func == "cummin" {
                                        r.min(v)
                                    } else {
                                        r.max(v)
                                    }
                                }
                                None => v,
                            };
                            run = Some(next);
                            next
                        })
                        .collect(),
                ))
            }
            "diff" => {
                let value = Self::req(&mut s, 0, func)?;
                self.consume_fuel(value.len())?;
                let x = value.promote_dbl()?;
                Ok(R0Val::Dbl(x.windows(2).map(|w| w[1] - w[0]).collect()))
            }
            "approx" => {
                // Linear interpolation: approx(x, y, xout)
                let xv = Self::req(&mut s, 0, func)?;
                let yv = Self::req(&mut s, 1, func)?;
                let ov = Self::req(&mut s, 2, func)?;
                self.consume_fuel(xv.len().max(yv.len()).max(ov.len()))?;
                self.consume_fuel(
                    ov.len()
                        .saturating_mul(xv.len().saturating_sub(1))
                        .max(ov.len()),
                )?;
                let xs = xv.promote_dbl()?;
                let ys = yv.promote_dbl()?;
                let xout = ov.promote_dbl()?;
                if xs.len() != ys.len() {
                    return Err(format!("ErrDimensionMismatch: approx x has length {}, y has length {}", xs.len(), ys.len()));
                }
                let mut out = Vec::with_capacity(xout.len());
                for &tx in &xout {
                    if is_missing(tx) || xs.is_empty() || tx < xs[0] || tx > *xs.last().unwrap() {
                        out.push(na_real());
                        continue;
                    }
                    let mut interp = na_real();
                    for i in 0..xs.len().saturating_sub(1) {
                        if tx >= xs[i] && tx <= xs[i + 1] {
                            let t = (tx - xs[i]) / (xs[i + 1] - xs[i]);
                            interp = ys[i] + t * (ys[i + 1] - ys[i]);
                            break;
                        }
                    }
                    out.push(interp);
                }
                Ok(R0Val::Dbl(out))
            }
            "%in%" => {
                // Set membership; NA matches NA (D17). Chr uses string equality, numeric/Lgl via dbl.
                let x = Self::req(&mut s, 0, func)?;
                let table = Self::req(&mut s, 1, func)?;
                self.consume_fuel(
                    x.len()
                        .saturating_mul(table.len())
                        .max(x.len())
                        .max(table.len()),
                )?;
                match (&x, &table) {
                    (R0Val::Chr(xv), R0Val::Chr(tv)) => Ok(R0Val::Lgl(
                        xv.iter().map(|e| if tv.contains(e) { LGL_TRUE } else { LGL_FALSE }).collect(),
                    )),
                    (R0Val::Chr(_), _) | (_, R0Val::Chr(_)) => {
                        Err("TypeError: %in% cannot compare character with non-character values".into())
                    }
                    _ => {
                        let xv = self.as_f64(&x)?;
                        let tv = if matches!(table, R0Val::Null) { Vec::new() } else { self.as_f64(&table)? };
                        Ok(R0Val::Lgl(
                            xv.iter()
                                .map(|&e| {
                                    let hit = tv.iter().any(|&t| {
                                        if is_missing(e) || is_missing(t) {
                                            is_missing(e) && is_missing(t) && is_na_real(e) == is_na_real(t)
                                        } else {
                                            e == t
                                        }
                                    });
                                    if hit { LGL_TRUE } else { LGL_FALSE }
                                })
                                .collect(),
                        ))
                    }
                }
            }
            "match" => {
                let x = Self::req(&mut s, 0, func)?;
                let table = Self::req(&mut s, 1, func)?;
                let nomatch = match s[2].take() {
                    None => NA_INT,
                    Some(v) => {
                        let ints = match v {
                            R0Val::Int(iv) => iv,
                            R0Val::Range { start, .. } => vec![start],
                            R0Val::Dbl(dv) => dv
                                .into_iter()
                                .map(|d| {
                                    if is_missing(d) || d < (i32::MIN + 1) as f64 || d > i32::MAX as f64 {
                                        NA_INT
                                    } else {
                                        d as i32
                                    }
                                })
                                .collect(),
                            R0Val::Lgl(lv) => lv
                                .into_iter()
                                .map(|b| match b {
                                    LGL_TRUE => 1,
                                    LGL_FALSE => 0,
                                    _ => NA_INT,
                                })
                                .collect(),
                            _ => return Err(format!("TypeError: nomatch in {} must be integer or numeric", func)),
                        };
                        ints.first().copied().unwrap_or(NA_INT)
                    }
                };
                self.consume_fuel(
                    x.len()
                        .saturating_mul(table.len())
                        .max(x.len())
                        .max(table.len()),
                )?;
                if matches!(x, R0Val::Null) {
                    return Ok(R0Val::Int(Vec::new()));
                }
                if matches!(table, R0Val::Null) {
                    return Ok(R0Val::Int(vec![nomatch; x.len()]));
                }
                match (&x, &table) {
                    (R0Val::Chr(xv), R0Val::Chr(tv)) => {
                        let out: Vec<i32> = xv
                            .iter()
                            .map(|e| match tv.iter().position(|t| t == e) {
                                Some(pos) => (pos + 1) as i32,
                                None => nomatch,
                            })
                            .collect();
                        Ok(R0Val::Int(out))
                    }
                    (R0Val::Chr(_), _) | (_, R0Val::Chr(_)) => {
                        Err("TypeError: match cannot compare character with non-character values".into())
                    }
                    _ => {
                        let xv = self.as_f64(&x)?;
                        let tv = self.as_f64(&table)?;
                        let out: Vec<i32> = xv
                            .iter()
                            .map(|&e| {
                                let pos = tv.iter().position(|&t| {
                                    if is_missing(e) || is_missing(t) {
                                        is_missing(e) && is_missing(t) && is_na_real(e) == is_na_real(t)
                                    } else {
                                        e == t
                                    }
                                });
                                match pos {
                                    Some(p) => (p + 1) as i32,
                                    None => nomatch,
                                }
                            })
                            .collect();
                        Ok(R0Val::Int(out))
                    }
                }
            }
            "numeric" | "integer" | "logical" | "character" => {
                let n = Self::parse_vector_length(s[0].take(), func)?;
                self.consume_fuel(n)?;
                match func {
                    "numeric" => Ok(R0Val::Dbl(vec![0.0; n])),
                    "integer" => Ok(R0Val::Int(vec![0; n])),
                    "logical" => Ok(R0Val::Lgl(vec![LGL_FALSE; n])),
                    "character" => Ok(R0Val::Chr(vec![Some(String::new()); n])),
                    _ => unreachable!(),
                }
            }
            "nrow" | "ncol" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(1)?;
                match x {
                    R0Val::Df { cols, .. } => {
                        let count = if func == "nrow" {
                            cols.first().map_or(0, |c| c.len())
                        } else {
                            cols.len()
                        };
                        Ok(R0Val::Int(vec![count as i32]))
                    }
                    _ => Ok(R0Val::Null),
                }
            }
            "as.numeric" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                match x {
                    R0Val::Dbl(_) => Ok(x),
                    R0Val::Int(v) => Ok(R0Val::Dbl(
                        v.into_iter()
                            .map(|n| if n == NA_INT { na_real() } else { n as f64 })
                            .collect(),
                    )),
                    R0Val::Range { .. } => Ok(R0Val::Dbl(x.promote_dbl()?)),
                    R0Val::Lgl(v) => Ok(R0Val::Dbl(
                        v.into_iter()
                            .map(|b| match b {
                                LGL_TRUE => 1.0,
                                LGL_FALSE => 0.0,
                                _ => na_real(),
                            })
                            .collect(),
                    )),
                    R0Val::Chr(v) => Ok(R0Val::Dbl(
                        v.into_iter()
                            .map(|opt| match opt {
                                Some(str) => str.trim().parse::<f64>().unwrap_or_else(|_| na_real()),
                                None => na_real(),
                            })
                            .collect(),
                    )),
                    R0Val::Null => Ok(R0Val::Dbl(Vec::new())),
                    _ => Err("TypeError: cannot coerce non-atomic value to numeric".into()),
                }
            }
            "as.integer" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                match x {
                    R0Val::Int(_) => Ok(x),
                    R0Val::Range { .. } => Ok(R0Val::Int(x.materialize_int()?)),
                    R0Val::Dbl(v) => Ok(R0Val::Int(
                        v.into_iter()
                            .map(|d| {
                                if d.is_nan() || d.is_infinite() {
                                    NA_INT
                                } else {
                                    let trunc = d.trunc();
                                    if trunc < (i32::MIN + 1) as f64 || trunc > i32::MAX as f64 {
                                        NA_INT
                                    } else {
                                        trunc as i32
                                    }
                                }
                            })
                            .collect(),
                    )),
                    R0Val::Lgl(v) => Ok(R0Val::Int(
                        v.into_iter()
                            .map(|b| match b {
                                LGL_TRUE => 1,
                                LGL_FALSE => 0,
                                _ => NA_INT,
                            })
                            .collect(),
                    )),
                    R0Val::Chr(v) => Ok(R0Val::Int(
                        v.into_iter()
                            .map(|opt| match opt {
                                Some(str) => {
                                    let trimmed = str.trim();
                                    if let Ok(n) = trimmed.parse::<i32>() {
                                        if n == NA_INT { NA_INT } else { n }
                                    } else if let Ok(d) = trimmed.parse::<f64>() {
                                        if d.is_nan() || d.is_infinite() {
                                            NA_INT
                                        } else {
                                            let trunc = d.trunc();
                                            if trunc < (i32::MIN + 1) as f64 || trunc > i32::MAX as f64 {
                                                NA_INT
                                            } else {
                                                trunc as i32
                                            }
                                        }
                                    } else {
                                        NA_INT
                                    }
                                }
                                None => NA_INT,
                            })
                            .collect(),
                    )),
                    R0Val::Null => Ok(R0Val::Int(Vec::new())),
                    _ => Err("TypeError: cannot coerce non-atomic value to integer".into()),
                }
            }
            "as.logical" => {
                let x = Self::req(&mut s, 0, func)?;
                self.consume_fuel(x.len())?;
                match x {
                    R0Val::Lgl(_) => Ok(x),
                    R0Val::Int(v) => Ok(R0Val::Lgl(
                        v.into_iter()
                            .map(|n| {
                                if n == NA_INT {
                                    LGL_NA
                                } else if n != 0 {
                                    LGL_TRUE
                                } else {
                                    LGL_FALSE
                                }
                            })
                            .collect(),
                    )),
                    R0Val::Range { .. } => {
                        let ints = x.materialize_int()?;
                        Ok(R0Val::Lgl(
                            ints.into_iter()
                                .map(|n| {
                                    if n == NA_INT {
                                        LGL_NA
                                    } else if n != 0 {
                                        LGL_TRUE
                                    } else {
                                        LGL_FALSE
                                    }
                                })
                                .collect(),
                        ))
                    }
                    R0Val::Dbl(v) => Ok(R0Val::Lgl(
                        v.into_iter()
                            .map(|d| {
                                if is_missing(d) {
                                    LGL_NA
                                } else if d != 0.0 {
                                    LGL_TRUE
                                } else {
                                    LGL_FALSE
                                }
                            })
                            .collect(),
                    )),
                    R0Val::Chr(v) => Ok(R0Val::Lgl(
                        v.into_iter()
                            .map(|opt| match opt {
                                Some(str) => {
                                    let trimmed = str.trim();
                                    if trimmed == "T" || trimmed.eq_ignore_ascii_case("true") {
                                        LGL_TRUE
                                    } else if trimmed == "F" || trimmed.eq_ignore_ascii_case("false") {
                                        LGL_FALSE
                                    } else {
                                        LGL_NA
                                    }
                                }
                                None => LGL_NA,
                            })
                            .collect(),
                    )),
                    R0Val::Null => Ok(R0Val::Lgl(Vec::new())),
                    _ => Err("TypeError: cannot coerce non-atomic value to logical".into()),
                }
            }
            "rep_len" => {
                let x = Self::req(&mut s, 0, func)?;
                let length_out = Self::req(&mut s, 1, func)?;
                self.rep_impl(func, x, None, Some(length_out), None)
            }
            "rep" => {
                let x = Self::req(&mut s, 0, func)?;
                let times = s[1].take();
                let length_out = s[2].take();
                let each = s[3].take();
                self.rep_impl(func, x, times, length_out, each)
            }
            "seq" => {
                let from = s[0].take();
                let to = s[1].take();
                let by = s[2].take();
                let length_out = s[3].take();
                let along_with = s[4].take();
                self.seq_impl(from, to, by, length_out, along_with)
            }
            _ => unreachable!("dispatch table covers all names"),
        }
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
    fn fuel_accounting_saturates_without_overflow() {
        let mut vm = R0Vm::new(i64::MIN);
        assert!(vm.consume_fuel(usize::MAX).unwrap_err().starts_with("ErrFuelExhausted:"));
        assert_eq!(vm.fuel, i64::MIN);
    }

    #[test]
    fn range_subsetting_charges_fuel_before_materialization() {
        let err = R0Vm::new(100_000).eval_script("x <- 1:2000000000; x[1]").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
    }

    #[test]
    fn range_missingness_intrinsics_charge_before_allocating() {
        for src in ["is.na(1:2000000000)", "is.nan(1:2000000000)"] {
            let err = R0Vm::new(100_000).eval_script(src).unwrap_err();
            assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
        }
    }

    #[test]
    fn vector_intrinsics_charge_fuel_for_output_and_nested_work() {
        let mut vm = vm_with(vec![("mask", R0Val::Lgl(vec![LGL_TRUE; 100]))]);
        vm.fuel = 50;
        assert!(vm
            .eval_script("which(mask)")
            .unwrap_err()
            .starts_with("ErrFuelExhausted:"));

        let mut vm = vm_with(vec![("ints", R0Val::Int(vec![1; 100]))]);
        vm.fuel = 50;
        assert!(vm
            .eval_script("abs(ints)")
            .unwrap_err()
            .starts_with("ErrFuelExhausted:"));

        let mut vm = vm_with(vec![("mask", R0Val::Lgl(vec![LGL_TRUE; 100]))]);
        vm.fuel = 50;
        assert!(vm
            .eval_script("ifelse(mask, 1, 2)")
            .unwrap_err()
            .starts_with("ErrFuelExhausted:"));

        let values = R0Val::Dbl((0..100).map(|v| v as f64).collect());
        let mut vm = vm_with(vec![("x", values.clone()), ("table", values)]);
        vm.fuel = 1_000;
        assert!(vm
            .eval_script("x %in% table")
            .unwrap_err()
            .starts_with("ErrFuelExhausted:"));

        let values = R0Val::Dbl((0..100).map(|v| v as f64).collect());
        let mut vm = vm_with(vec![
            ("x", values.clone()),
            ("y", values.clone()),
            ("xout", values),
        ]);
        vm.fuel = 1_000;
        assert!(vm
            .eval_script("approx(x, y, xout)")
            .unwrap_err()
            .starts_with("ErrFuelExhausted:"));
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

    fn vm_with(vars: Vec<(&str, R0Val)>) -> R0Vm {
        let mut vm = R0Vm::new(100_000);
        for (k, v) in vars {
            vm.globals.insert(k.into(), v);
        }
        vm
    }

    fn assert_na_dbl(got: Result<R0Val, String>) {
        match got.unwrap() {
            R0Val::Dbl(v) => assert!(v.len() == 1 && is_na_real(v[0]), "{:?}", v),
            other => panic!("expected Dbl, got {:?}", other),
        }
    }

    #[test]
    fn intrinsic_type_rules() {
        same(run("c(1L, 2L)"), R0Val::Int(vec![1, 2]));
        same(run("c(1L, 2.5)"), R0Val::Dbl(vec![1.0, 2.5]));
        same(run("c(TRUE, FALSE)"), R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE]));
        same(run("c(\"a\", \"b\")"), R0Val::Chr(vec![Some("a".into()), Some("b".into())]));
        let err = run("c(TRUE, 1L)").unwrap_err();
        assert!(err.starts_with("TypeError:"), "{err}");
        let err = run("c(\"a\", 1L)").unwrap_err();
        assert!(err.starts_with("TypeError:"), "{err}");
        same(run("sum(c(TRUE, TRUE))"), R0Val::Int(vec![2]));
        same(run("sum(1:3)"), R0Val::Int(vec![6]));
        same(run("sum(c(1.5, 2))"), R0Val::Dbl(vec![3.5]));
        same(run("mean(c(TRUE, FALSE))"), R0Val::Dbl(vec![0.5]));
        same(run("min(c(3L, 1L))"), R0Val::Int(vec![1]));
        same(run("max(c(TRUE, FALSE))"), R0Val::Int(vec![1]));
        same(run("abs(-2L)"), R0Val::Int(vec![2]));
        same(run("floor(2L)"), R0Val::Dbl(vec![2.0]));
        same(run("which(c(FALSE, TRUE, TRUE))"), R0Val::Int(vec![2, 3]));
        same(run("length(c(1, 2))"), R0Val::Int(vec![2]));
        same(run("ifelse(c(TRUE, FALSE), 1L, 2L)"), R0Val::Int(vec![1, 2]));
        same(run("ifelse(c(TRUE, NA), 1, 2)"), R0Val::Dbl(vec![1.0, na_real()]));
        same(run("pmax(c(1, 5), 3)"), R0Val::Dbl(vec![3.0, 5.0]));
        same(run("cumsum(c(1, 2, 3))"), R0Val::Dbl(vec![1.0, 3.0, 6.0]));
        same(run("diff(c(1, 4, 9))"), R0Val::Dbl(vec![3.0, 5.0]));
    }

    #[test]
    fn c_all_na_logical_promotes_to_numeric() {
        match run("c(1, NA, 3)").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 3);
                assert!(v[0] == 1.0 && is_na_real(v[1]) && v[2] == 3.0);
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        same(run("c(1L, NA, 3L)"), R0Val::Int(vec![1, NA_INT, 3]));
        same(run("c(NA, NA)"), R0Val::Lgl(vec![LGL_NA, LGL_NA]));
    }

    #[test]
    fn c_non_na_logical_mixed_with_numeric_errors() {
        assert!(run("c(TRUE, 1)").unwrap_err().contains("TypeError"));
        assert!(run("c(TRUE, FALSE, 1)").unwrap_err().contains("TypeError"));
        assert!(run("c(\"a\", TRUE)").unwrap_err().contains("TypeError"));
    }

    #[test]
    fn approx_and_named_args() {
        same(run("approx(c(1,2,3), c(10,20,30), xout = 2.5)"), R0Val::Dbl(vec![25.0]));
        same(run("c(1.5, 2.5) |> approx(c(1,2,3), c(10,20,30), xout = _)"), R0Val::Dbl(vec![15.0, 25.0]));
        same(run("approx(c(1,2,3), c(10,20,30), 2.5)"), R0Val::Dbl(vec![25.0]));
        same(run("sum(c(1, NA_real_, 2), na.rm = TRUE)"), R0Val::Dbl(vec![3.0]));
        same(run("sum(c(1, NA_real_))"), R0Val::Dbl(vec![na_real()]));
        let err = run("approx(c(1,2), c(1,2), bogus = 1)").unwrap_err();
        assert_eq!(err, "UnknownArgument: bogus in approx");
        let err = run("length(c(1), na.rm = TRUE)").unwrap_err();
        assert_eq!(err, "UnknownArgument: na.rm in length");
        let err = run("sum(1, 2, 3)").unwrap_err();
        assert!(err.starts_with("TooManyArguments:"), "{err}");
    }

    #[test]
    fn list_df_and_in() {
        let got = run("list(a = 1)").unwrap();
        assert!(format!("{:?}", got).starts_with("List"), "{:?}", got);
        let err = run("data.frame(a = c(1, 2), b = 1)").unwrap_err();
        assert!(err.starts_with("ErrDimensionMismatch:"), "{err}");
        same(run("1L %in% c(1L, 2L)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("NA_real_ %in% c(1, NA_real_)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("NA %in% NA"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("c(1, 2) %in% c(2L)"), R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE]));
        same(run("\"a\" %in% c(\"a\", \"b\")"), R0Val::Lgl(vec![LGL_TRUE]));
        let err = run("\"a\" %in% c(1L)").unwrap_err();
        assert!(err.starts_with("TypeError:"), "{err}");
    }

    #[test]
    fn missing_nan_semantics_cover_abs_cumsum_and_membership() {
        match run("abs(NaN)").unwrap() {
            R0Val::Dbl(v) => assert!(v[0].is_nan() && !is_na_real(v[0])),
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("cumsum(c(1, NaN, 2))").unwrap() {
            R0Val::Dbl(v) => assert!(v[0] == 1.0 && v[1].is_nan() && v[2].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }
        same(run("NaN %in% c(NA_real_, NaN)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("NA_real_ %in% NaN"), R0Val::Lgl(vec![LGL_FALSE]));
    }

    #[test]
    fn unknown_intrinsic_name() {
        assert_eq!(run("nosuch(1)").unwrap_err(), "Unknown function or intrinsic: 'nosuch'");
    }

    #[test]
    fn oob_and_range_subsetting_preserve_type() {
        let mut vm = vm_with(vec![("x", R0Val::Dbl(vec![1.0, 2.0, 3.0]))]);
        assert_na_dbl(vm.eval_script("x[99L]"));
        same(run("(1:3)[99L]"), R0Val::Int(vec![NA_INT]));
        same(run("(1:3)[2L]"), R0Val::Int(vec![2]));
        same(run("(1:3)[-2L]"), R0Val::Int(vec![1, 3]));
        same(vm.eval_script("x[-2L]"), R0Val::Dbl(vec![1.0, 3.0]));
    }

    #[test]
    fn mixed_and_bounds_traps() {
        let mut vm = vm_with(vec![
            ("x", R0Val::Dbl(vec![1.0, 2.0, 3.0])),
            ("idx", R0Val::Int(vec![1, -2])),
        ]);
        let err = vm.eval_script("x[idx]").unwrap_err();
        assert!(err.starts_with("ErrMixedSubscript:"), "{err}");
        let err = run("(1:3)[[5L]]").unwrap_err();
        assert!(err.starts_with("ErrBoundsTrap:"), "{err}");
        same(run("(1:3)[[2L]]"), R0Val::Int(vec![2]));
    }

    #[test]
    fn logical_mask_length_and_na() {
        let mut vm = vm_with(vec![
            ("x", R0Val::Dbl(vec![1.0, 2.0, 3.0])),
            ("m", R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE])),
            ("na", R0Val::Lgl(vec![LGL_NA, LGL_TRUE, LGL_FALSE])),
        ]);
        let err = vm.eval_script("x[m]").unwrap_err();
        assert!(err.starts_with("ErrDimensionMismatch:"), "{err}");
        let got = vm.eval_script("x[na]").unwrap();
        match got {
            R0Val::Dbl(v) => assert!(v.len() == 2 && is_na_real(v[0]) && v[1] == 2.0, "{:?}", v),
            other => panic!("{:?}", other),
        }
    }

    #[test]
    fn subset_assign_mask_and_index() {
        let mut vm = vm_with(vec![("x", R0Val::Dbl(vec![-1.0, 2.0, -3.0]))]);
        same(vm.eval_script("x[x < 0] <- 0; x"), R0Val::Dbl(vec![0.0, 2.0, 0.0]));
        let mut vm = vm_with(vec![
            ("x", R0Val::Dbl(vec![1.0, 2.0, 3.0])),
            ("na", R0Val::Lgl(vec![LGL_NA, LGL_TRUE, LGL_FALSE])),
        ]);
        same(vm.eval_script("x[na] <- 9; x"), R0Val::Dbl(vec![1.0, 9.0, 3.0]));
        same(vm.eval_script("x[5L] <- 5; x"), R0Val::Dbl(vec![1.0, 9.0, 3.0, f64::NAN, 5.0]));
    }

    #[test]
    fn subset_assign_type_rules() {
        let mut vm = vm_with(vec![("y", R0Val::Int(vec![1, 2, 3]))]);
        same(vm.eval_script("y[2L] <- 2.5; y"), R0Val::Dbl(vec![1.0, 2.5, 3.0]));
        let mut vm = vm_with(vec![
            ("x", R0Val::Dbl(vec![1.0, 2.0])),
            ("r3", R0Val::Dbl(vec![1.0, 2.0, 3.0])),
        ]);
        let err = vm.eval_script("x[1L] <- TRUE").unwrap_err();
        assert!(err.starts_with("TypeError:"), "{err}");
        let err = vm.eval_script("x[1:2] <- r3").unwrap_err();
        assert!(err.starts_with("ErrDimensionMismatch:"), "{err}");
    }

    #[test]
    fn null_df_and_list_access() {
        let mut vm = vm_with(vec![
            ("n", R0Val::Null),
            ("d", R0Val::df(vec!["a".into()], vec![R0Val::Dbl(vec![1.0, 2.0])]).unwrap()),
        ]);
        let err = vm.eval_script("n[1L] <- 1").unwrap_err();
        assert!(err.starts_with("TypeError:"), "{err}");
        same(vm.eval_script("d$a"), R0Val::Dbl(vec![1.0, 2.0]));
        same(vm.eval_script("d[[1L]]"), R0Val::Dbl(vec![1.0, 2.0]));
        assert_eq!(vm.eval_script("d$zz").unwrap_err(), "Unknown column '$zz'");
        let err = vm.eval_script("d[1L]").unwrap_err();
        assert!(err.starts_with("TypeError:"), "{err}");
    }

    #[test]
    fn na_nan_propagation() {
        let nan = run("NaN + 1").unwrap();
        assert!(matches!(&nan, R0Val::Dbl(v) if v[0].is_nan() && !is_na_real(v[0])), "{:?}", nan);
        let na = run("NA_real_ + 1").unwrap();
        assert!(matches!(&na, R0Val::Dbl(v) if is_na_real(v[0])), "{:?}", na);
    }

    #[test]
    fn is_nan_true_for_nan_false_for_na() {
        same(run("is.nan(NaN)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("is.nan(NA_real_)"), R0Val::Lgl(vec![LGL_FALSE]));
    }

    #[test]
    fn oob_assign_extends_dbl_with_na() {
        match run("x <- c(1,2,3)\n x[5] <- 9\n x").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 5, "{:?}", v);
                assert!(v[0] == 1.0 && v[1] == 2.0 && v[2] == 3.0 && is_na_real(v[3]) && v[4] == 9.0, "{:?}", v);
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
    }

    #[test]
    fn oob_assign_extends_int_and_lgl_with_na() {
        match run("x <- 1:3\n x[5] <- 9L\n x").unwrap() {
            R0Val::Int(v) => assert_eq!(v, vec![1, 2, 3, NA_INT, 9]),
            other => panic!("expected Int, got {:?}", other),
        }
        match run("x <- c(TRUE, FALSE)\n x[4] <- TRUE\n x").unwrap() {
            R0Val::Lgl(v) => assert_eq!(v, vec![LGL_TRUE, LGL_FALSE, LGL_NA, LGL_TRUE]),
            other => panic!("expected Lgl, got {:?}", other),
        }
    }

    #[test]
    fn chr_c_with_all_na_logical_gives_na_character() {
        match run("c(\"a\", NA)").unwrap() {
            R0Val::Chr(v) => assert_eq!(v, vec![Some("a".to_string()), None]),
            other => panic!("expected Chr, got {:?}", other),
        }
    }

    #[test]
    fn is_na_on_character_vector() {
        same(run("is.na(c(\"a\", NA))"), R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE]));
    }

    /// Classify doubles the way R prints them: NA, NaN, or the value.
    fn cls(v: &[f64]) -> Vec<String> {
        v.iter()
            .map(|&x| if is_na_real(x) { "NA".to_string() } else if x.is_nan() { "NaN".to_string() } else { format!("{x}") })
            .collect()
    }

    fn dbl_cls(src: &str) -> Vec<String> {
        match run(src).unwrap() {
            R0Val::Dbl(v) => cls(&v),
            other => panic!("{src}: expected Dbl, got {:?}", other),
        }
    }

    #[test]
    fn r_mirror_arithmetic_first_missing_operand_wins() {
        // Mirrors R 4.x on this platform: the first missing operand's payload propagates.
        assert_eq!(dbl_cls("NaN + NA_real_"), vec!["NaN"]);
        assert_eq!(dbl_cls("NA_real_ + NaN"), vec!["NA"]);
        assert_eq!(dbl_cls("NaN * NA_real_"), vec!["NaN"]);
        assert_eq!(dbl_cls("NA_real_ * NaN"), vec!["NA"]);
        assert_eq!(dbl_cls("NA_real_ - NaN"), vec!["NA"]);
        assert_eq!(dbl_cls("NaN - NA_real_"), vec!["NaN"]);
    }

    #[test]
    fn r_mirror_pmin_pmax_missing_rules() {
        assert_eq!(dbl_cls("pmin(NA_real_, NaN)"), vec!["NaN"]);
        assert_eq!(dbl_cls("pmin(NaN, NA_real_)"), vec!["NA"]);
        assert_eq!(dbl_cls("pmax(NA_real_, NaN)"), vec!["NaN"]);
        assert_eq!(dbl_cls("pmax(NaN, NA_real_)"), vec!["NA"]);
        assert_eq!(dbl_cls("pmin(NaN, 1)"), vec!["NaN"]);
        assert_eq!(dbl_cls("pmin(NA_real_, 1)"), vec!["NA"]);
        assert_eq!(
            dbl_cls("pmin(c(1, NA_real_, NaN), c(NaN, 2, NA_real_))"),
            vec!["NaN", "NA", "NA"]
        );
    }

    #[test]
    fn seq_len_and_seq_along_edge_cases() {
        same(run("seq_len(0L)"), R0Val::Int(vec![]));
        same(run("seq_len(3L)"), R0Val::Range { start: 1, end: 3, step: 1 });
        same(run("seq_along(c())"), R0Val::Int(vec![]));
        same(run("seq_along(c(10, 20))"), R0Val::Range { start: 1, end: 2, step: 1 });
        assert!(run("seq_len(-1L)").is_err());
        assert!(run("seq_len(NA_integer_)").is_err());
        assert!(run("seq_len(c(1L, 2L))").is_err());
        assert!(run("seq_len(c())").is_err());
    }

    #[test]
    fn unary_pos_and_neg_handle_ranges_and_reject_non_numeric() {
        same(run("-(1:3)"), R0Val::Int(vec![-1, -2, -3]));
        same(run("+(1:3)"), R0Val::Range { start: 1, end: 3, step: 1 });
        same(run("+c(1L, 2L)"), R0Val::Int(vec![1, 2]));
        assert!(run("+TRUE").is_err());
        assert!(run("+\"a\"").is_err());
        assert!(run("-\"a\"").is_err());
    }

    #[test]
    fn locked_prelude_and_user_function_arg_binding() {
        assert!(run("sum <- function(x) 0").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("c <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        same(run("f <- function(a, b = 10) a + b; f(b = 2, 5)"), R0Val::Dbl(vec![7.0]));
        assert!(run("f <- function(a) a; f(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("f <- function(a) a; f(z = 1)").unwrap_err().starts_with("UnknownArgument:"));
        assert!(run("f <- function(a) a; f(a = 1, a = 2)").unwrap_err().starts_with("DuplicateArgument:"));
    }

    #[test]
    fn na_rm_validation_rules() {
        same(run("sum(c(1L, NA_integer_), na.rm = c(TRUE, FALSE))"), R0Val::Int(vec![1]));
        same(run("pmin(1L, NA_integer_, na.rm = c(FALSE, TRUE))"), R0Val::Int(vec![NA_INT]));
        assert!(run("sum(c(1, 2), na.rm = NA)").unwrap_err().starts_with("ErrInvalidCondition:"));
        assert!(run("sum(c(1, 2), na.rm = 1)").unwrap_err().starts_with("TypeError:"));
        assert!(run("pmax(1, 2, na.rm = NA)").unwrap_err().starts_with("ErrInvalidCondition:"));
        assert!(run("pmax(1, 2, na.rm = \"TRUE\")").unwrap_err().starts_with("TypeError:"));
        assert!(run("sum(c(1, 2), na.rm = c(TRUE)[FALSE])").unwrap_err().starts_with("ErrInvalidCondition:"));
    }

    #[test]
    fn r_mirror_min_max_na_beats_nan_and_sum_first_missing() {
        assert_eq!(dbl_cls("min(c(NaN, NA_real_))"), vec!["NA"]);
        assert_eq!(dbl_cls("max(c(NA_real_, NaN))"), vec!["NA"]);
        assert_eq!(dbl_cls("min(c(NaN, 1))"), vec!["NaN"]);
        assert_eq!(dbl_cls("sum(c(NaN, NA_real_))"), vec!["NaN"]);
        assert_eq!(dbl_cls("sum(c(NA_real_, NaN))"), vec!["NA"]);
        assert_eq!(dbl_cls("mean(c(NaN, 1))"), vec!["NaN"]);
    }

    #[test]
    fn constructors_default_and_normal_cases() {
        same(run("numeric()"), R0Val::Dbl(vec![]));
        same(run("integer()"), R0Val::Int(vec![]));
        same(run("logical()"), R0Val::Lgl(vec![]));
        same(run("character()"), R0Val::Chr(vec![]));

        same(run("numeric(0)"), R0Val::Dbl(vec![]));
        same(run("integer(0L)"), R0Val::Int(vec![]));
        same(run("logical(0)"), R0Val::Lgl(vec![]));
        same(run("character(0L)"), R0Val::Chr(vec![]));

        same(run("numeric(3)"), R0Val::Dbl(vec![0.0, 0.0, 0.0]));
        same(run("integer(3L)"), R0Val::Int(vec![0, 0, 0]));
        same(run("logical(2)"), R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE]));
        same(run("character(2)"), R0Val::Chr(vec![Some(String::new()), Some(String::new())]));

        same(run("numeric(length = 2)"), R0Val::Dbl(vec![0.0, 0.0]));
        same(run("integer(length = 1L)"), R0Val::Int(vec![0]));
        same(run("logical(length = 0)"), R0Val::Lgl(vec![]));
        same(run("character(length = 1)"), R0Val::Chr(vec![Some(String::new())]));
        same(run("numeric(1:1)"), R0Val::Dbl(vec![0.0]));
    }

    #[test]
    fn constructors_pre_allocation_fuel_exhaustion() {
        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("numeric(100)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("integer(100L)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("logical(100)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("character(100)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
    }

    #[test]
    fn constructors_invalid_length_validation() {
        assert!(run("numeric(-1)").is_err());
        assert!(run("integer(-1L)").is_err());
        assert!(run("logical(-5)").is_err());
        assert!(run("character(-2L)").is_err());

        assert!(run("numeric(NA_real_)").is_err());
        assert!(run("integer(NA_integer_)").is_err());
        assert!(run("logical(NA)").is_err());
        assert!(run("character(NA)").is_err());
        assert!(run("numeric(NaN)").is_err());
        assert!(run("numeric(Inf)").is_err());
        assert!(run("character(-Inf)").is_err());

        assert!(run("numeric(c(1, 2))").is_err());
        assert!(run("integer(1:2)").is_err());
        assert!(run("numeric(c())").is_err());
        assert!(run("integer(integer(0))").is_err());

        assert!(run("numeric(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("numeric(len = 3)").unwrap_err().starts_with("UnknownArgument:"));
        assert!(run("numeric(length = 1, length = 2)").unwrap_err().starts_with("DuplicateArgument:"));
        assert!(run("numeric(length = 1, 2)").unwrap_err().starts_with("TooManyArguments:"));
    }

    #[test]
    fn shape_nrow_ncol_dataframe_and_vectors() {
        same(run("nrow(data.frame(a = c(1, 2), b = c(3, 4)))"), R0Val::Int(vec![2]));
        same(run("ncol(data.frame(a = c(1, 2), b = c(3, 4)))"), R0Val::Int(vec![2]));
        same(run("nrow(data.frame())"), R0Val::Int(vec![0]));
        same(run("ncol(data.frame())"), R0Val::Int(vec![0]));
        same(run("nrow(x = data.frame(a = 1:3))"), R0Val::Int(vec![3]));
        same(run("ncol(x = data.frame(a = 1:3))"), R0Val::Int(vec![1]));

        same(run("nrow(1:3)"), R0Val::Null);
        same(run("ncol(1:3)"), R0Val::Null);
        same(run("nrow(c(1.0, 2.0))"), R0Val::Null);
        same(run("ncol(c(\"a\", \"b\"))"), R0Val::Null);
        same(run("nrow(c())"), R0Val::Null);
        same(run("ncol(c())"), R0Val::Null);
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("nrow(n)"), R0Val::Null);
        same(vm.eval_script("ncol(n)"), R0Val::Null);
        same(run("nrow(list(a = 1:2))"), R0Val::Null);
        same(run("ncol(list(a = 1:2))"), R0Val::Null);

        assert!(run("nrow()").is_err());
        assert!(run("ncol()").is_err());
        assert!(run("nrow(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("ncol(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("nrow(y = 1)").unwrap_err().starts_with("UnknownArgument:"));
    }

    #[test]
    fn locked_prelude_constructors_and_shape() {
        assert!(run("numeric <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("integer <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("logical <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("character <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("nrow <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("ncol <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
    }

    #[test]
    fn as_numeric_coercion_rules() {
        same(run("as.numeric(c(1.5, -2.5))"), R0Val::Dbl(vec![1.5, -2.5]));
        assert_eq!(dbl_cls("as.numeric(c(1.5, NA_real_, NaN))"), vec!["1.5", "NA", "NaN"]);
        same(run("as.numeric(c(1L, 2L))"), R0Val::Dbl(vec![1.0, 2.0]));
        assert_eq!(dbl_cls("as.numeric(c(1L, NA_integer_, 3L))"), vec!["1", "NA", "3"]);
        same(run("as.numeric(1:3)"), R0Val::Dbl(vec![1.0, 2.0, 3.0]));
        assert_eq!(dbl_cls("as.numeric(c(TRUE, FALSE, NA))"), vec!["1", "0", "NA"]);
        assert_eq!(
            dbl_cls("as.numeric(c(\"1.25\", \"-3.5\", \"1e2\", \"Inf\", \"-Inf\", \"NaN\", \"NA\", \"invalid\", NA))"),
            vec!["1.25", "-3.5", "100", "inf", "-inf", "NaN", "NA", "NA", "NA"]
        );
        same(run("as.numeric(c())"), R0Val::Dbl(vec![]));
        same(run("as.numeric(x = 10L)"), R0Val::Dbl(vec![10.0]));
        assert!(run("as.numeric(data.frame(a = 1:2))").unwrap_err().starts_with("TypeError:"));
        assert!(run("as.numeric(list(a = 1:2))").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn as_integer_coercion_rules() {
        same(run("as.integer(c(1L, -5L, NA_integer_))"), R0Val::Int(vec![1, -5, NA_INT]));
        same(run("as.integer(1:3)"), R0Val::Int(vec![1, 2, 3]));
        same(
            run("as.integer(c(1.1, 1.9, -1.1, -1.9, 0.0, -0.0))"),
            R0Val::Int(vec![1, 1, -1, -1, 0, 0]),
        );
        same(
            run("as.integer(c(NA_real_, NaN, Inf, -Inf, 1e12, -1e12))"),
            R0Val::Int(vec![NA_INT, NA_INT, NA_INT, NA_INT, NA_INT, NA_INT]),
        );
        same(run("as.integer(c(TRUE, FALSE, NA))"), R0Val::Int(vec![1, 0, NA_INT]));
        same(
            run("as.integer(c(\"1\", \"-5\", \"1.9\", \"-2.5\", \"1e3\", \"1e12\", \"Inf\", \"NaN\", \"NA\", \"invalid\", NA))"),
            R0Val::Int(vec![1, -5, 1, -2, 1000, NA_INT, NA_INT, NA_INT, NA_INT, NA_INT, NA_INT]),
        );
        same(run("as.integer(c())"), R0Val::Int(vec![]));
        same(run("as.integer(x = TRUE)"), R0Val::Int(vec![1]));
        assert!(run("as.integer(data.frame(a = 1:2))").unwrap_err().starts_with("TypeError:"));
        assert!(run("as.integer(list(a = 1:2))").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn as_logical_coercion_rules() {
        same(
            run("as.logical(c(TRUE, FALSE, NA))"),
            R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE, LGL_NA]),
        );
        same(
            run("as.logical(c(0L, 1L, -1L, 100L, NA_integer_))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_NA]),
        );
        same(
            run("as.logical(0:2)"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE, LGL_TRUE]),
        );
        same(
            run("as.logical(c(0.0, -0.0, 1.5, -0.1, Inf, -Inf, NA_real_, NaN))"),
            R0Val::Lgl(vec![
                LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_NA, LGL_NA,
            ]),
        );
        same(
            run("as.logical(c(\"T\", \"TRUE\", \"True\", \"true\", \"F\", \"FALSE\", \"False\", \"false\", \"NA\", \"abc\", \"\", NA))"),
            R0Val::Lgl(vec![
                LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_FALSE,
                LGL_NA, LGL_NA, LGL_NA, LGL_NA,
            ]),
        );
        same(run("as.logical(c())"), R0Val::Lgl(vec![]));
        same(run("as.logical(x = 1.0)"), R0Val::Lgl(vec![LGL_TRUE]));
        assert!(run("as.logical(data.frame(a = 1:2))").unwrap_err().starts_with("TypeError:"));
        assert!(run("as.logical(list(a = 1:2))").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn coercions_pre_allocation_fuel_exhaustion() {
        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("as.numeric(1:1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("as.integer(1:1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("as.logical(1:1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
    }

    #[test]
    fn coercions_argument_validation_and_prelude_locking() {
        assert!(run("as.numeric()").is_err());
        assert!(run("as.integer()").is_err());
        assert!(run("as.logical()").is_err());

        assert!(run("as.numeric(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("as.integer(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("as.logical(1, 2)").unwrap_err().starts_with("TooManyArguments:"));

        assert!(run("as.numeric(y = 1)").unwrap_err().starts_with("UnknownArgument:"));
        assert!(run("as.numeric(x = 1, x = 2)").unwrap_err().starts_with("DuplicateArgument:"));

        assert!(run("as.numeric <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("as.integer <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("as.logical <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
    }

    #[test]
    fn rep_len_basic_cycling_and_type_preservation() {
        same(run("rep_len(c(1.5, 2.5), 5)"), R0Val::Dbl(vec![1.5, 2.5, 1.5, 2.5, 1.5]));
        same(run("rep_len(c(1.5, 2.5), 1)"), R0Val::Dbl(vec![1.5]));
        same(run("rep_len(c(1.5, 2.5), 0)"), R0Val::Dbl(vec![]));

        same(run("rep_len(c(10L, 20L), 3)"), R0Val::Int(vec![10, 20, 10]));
        same(run("rep_len(1:3, 5)"), R0Val::Int(vec![1, 2, 3, 1, 2]));
        same(run("rep_len(1:3, 2)"), R0Val::Int(vec![1, 2]));
        same(run("rep_len(1:3, 0)"), R0Val::Int(vec![]));

        same(run("rep_len(c(TRUE, FALSE), 3)"), R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE, LGL_TRUE]));
        same(run("rep_len(c(\"a\", \"b\"), 3)"), R0Val::Chr(vec![Some("a".into()), Some("b".into()), Some("a".into())]));

        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("rep_len(n, 0)"), R0Val::Null);

        assert_eq!(dbl_cls("rep_len(c(NA_real_, 1.0), 3)"), vec!["NA", "1", "NA"]);

        same(run("rep_len(x = 1:2, length.out = 3)"), R0Val::Int(vec![1, 2, 1]));
        same(run("rep_len(length.out = 3, x = 1:2)"), R0Val::Int(vec![1, 2, 1]));

        // Error cases
        assert!(run("rep_len(c(), 3)").is_err());
        assert!(run("rep_len(numeric(0), 3)").is_err());
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        assert!(vm.eval_script("rep_len(n, 3)").is_err());

        assert!(run("rep_len(1:3, -1)").is_err());
        assert!(run("rep_len(1:3, NA_integer_)").is_err());
        assert!(run("rep_len(1:3, NA_real_)").is_err());
        assert!(run("rep_len(1:3, 1:2)").is_err());
        assert!(run("rep_len()").is_err());
        assert!(run("rep_len(1:3)").is_err());
        assert!(run("rep_len(1, 2, 3)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("rep_len(1, foo = 2)").unwrap_err().starts_with("UnknownArgument:"));
    }

    #[test]
    fn rep_argument_combinations_and_precedence() {
        same(run("rep(1:3, 2)"), R0Val::Int(vec![1, 2, 3, 1, 2, 3]));
        same(run("rep(1:3, times = 2)"), R0Val::Int(vec![1, 2, 3, 1, 2, 3]));
        same(run("rep(1:3, each = 2)"), R0Val::Int(vec![1, 1, 2, 2, 3, 3]));
        same(run("rep(1:3, times = 2, each = 2)"), R0Val::Int(vec![1, 1, 2, 2, 3, 3, 1, 1, 2, 2, 3, 3]));
        same(run("rep(1:3, length.out = 5)"), R0Val::Int(vec![1, 2, 3, 1, 2]));
        same(run("rep(1:3, each = 2, length.out = 5)"), R0Val::Int(vec![1, 1, 2, 2, 3]));
        same(run("rep(1:3, times = 2, length.out = 5)"), R0Val::Int(vec![1, 2, 3, 1, 2]));

        // Vector times
        same(run("rep(1:3, c(2, 1, 3))"), R0Val::Int(vec![1, 1, 2, 3, 3, 3]));
        same(run("rep(c(1.5, 2.5), c(2, 1))"), R0Val::Dbl(vec![1.5, 1.5, 2.5]));
        same(run("rep(c(TRUE, FALSE), c(1, 2))"), R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE, LGL_FALSE]));
        same(run("rep(c(\"a\", \"b\"), c(2, 1))"), R0Val::Chr(vec![Some("a".into()), Some("a".into()), Some("b".into())]));

        // Zero counts
        same(run("rep(1:3, 0)"), R0Val::Int(vec![]));
        same(run("rep(1:3, times = 0)"), R0Val::Int(vec![]));
        same(run("rep(1:3, each = 0)"), R0Val::Int(vec![]));
        same(run("rep(1:3, length.out = 0)"), R0Val::Int(vec![]));
        same(run("rep(numeric(0), times = 5)"), R0Val::Dbl(vec![]));
        same(run("rep(numeric(0), length.out = 0)"), R0Val::Dbl(vec![]));

        // Errors
        assert!(run("rep(numeric(0), length.out = 5)").is_err());
        assert!(run("rep(1:3, c(2, 1))").is_err());
        assert!(run("rep(1:3, c(2, 1, 3), each = 2)").is_err());
        assert!(run("rep(1:3, times = -1)").is_err());
        assert!(run("rep(1:3, each = -1)").is_err());
        assert!(run("rep(1:3, length.out = -1)").is_err());
        assert!(run("rep(1:3, times = NA_integer_)").is_err());
        assert!(run("rep(1:3, each = NA_integer_)").is_err());
        assert!(run("rep(1:3, length.out = NA_integer_)").is_err());
        assert!(run("rep()").is_err());
        assert!(run("rep(1, 2, 3, 4, 5)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("rep(1, foo = 2)").unwrap_err().starts_with("UnknownArgument:"));
    }

    #[test]
    fn seq_patterns_and_edge_cases() {
        same(run("seq()"), R0Val::Range { start: 1, end: 1, step: 1 });
        same(run("seq(5)"), R0Val::Range { start: 1, end: 5, step: 1 });
        same(run("seq(5L)"), R0Val::Range { start: 1, end: 5, step: 1 });
        same(run("seq(0)"), R0Val::Range { start: 1, end: 0, step: -1 });
        same(run("seq(-2)"), R0Val::Range { start: 1, end: -2, step: -1 });
        same(run("seq(c())"), R0Val::Int(vec![]));
        same(run("seq(numeric(0))"), R0Val::Int(vec![]));
        same(run("seq(c(10, 20, 30))"), R0Val::Range { start: 1, end: 3, step: 1 });

        same(run("seq(1, 5)"), R0Val::Range { start: 1, end: 5, step: 1 });
        same(run("seq(5, 1)"), R0Val::Range { start: 5, end: 1, step: -1 });
        same(run("seq(to = 5)"), R0Val::Range { start: 1, end: 5, step: 1 });
        same(run("seq(1.5, 4.5)"), R0Val::Dbl(vec![1.5, 2.5, 3.5, 4.5]));
        same(run("seq(4.5, 1.5)"), R0Val::Dbl(vec![4.5, 3.5, 2.5, 1.5]));

        // by parameter
        same(run("seq(1L, 5L, by = 2L)"), R0Val::Int(vec![1, 3, 5]));
        same(run("seq(1, 5, by = 2)"), R0Val::Dbl(vec![1.0, 3.0, 5.0]));
        same(run("seq(1, 6, by = 2)"), R0Val::Dbl(vec![1.0, 3.0, 5.0]));
        same(run("seq(5, 1, by = -2)"), R0Val::Dbl(vec![5.0, 3.0, 1.0]));

        // length.out parameter
        same(run("seq(1, 5, length.out = 5)"), R0Val::Dbl(vec![1.0, 2.0, 3.0, 4.0, 5.0]));
        same(run("seq(1, 5, length.out = 3)"), R0Val::Dbl(vec![1.0, 3.0, 5.0]));
        same(run("seq(1, 5, length.out = 1)"), R0Val::Dbl(vec![1.0]));
        same(run("seq(1L, 5L, length.out = 1L)"), R0Val::Int(vec![1]));
        same(run("seq(1, 5, length.out = 0)"), R0Val::Dbl(vec![]));
        same(run("seq(1, by = 2, length.out = 4)"), R0Val::Dbl(vec![1.0, 3.0, 5.0, 7.0]));

        // along.with
        same(run("seq(along.with = c(10, 20))"), R0Val::Range { start: 1, end: 2, step: 1 });
        same(run("seq(along.with = c())"), R0Val::Int(vec![]));

        // Errors
        assert!(run("seq(1, 5, by = -1)").is_err());
        assert!(run("seq(5, 1, by = 1)").is_err());
        assert!(run("seq(1, 5, by = 0)").is_err());
        assert!(run("seq(1, 5, length.out = -1)").is_err());
        assert!(run("seq(1, 5, by = 1, length.out = 5)").is_err());
        assert!(run("seq(NA_integer_)").is_err());
        assert!(run("seq(NA_real_)").is_err());
        assert!(run("seq(\"a\")").is_err());
        assert!(run("seq(1, 2, 3, 4, 5)").unwrap_err().starts_with("TooManyArguments:"));
    }

    #[test]
    fn rep_seq_pre_allocation_fuel_exhaustion() {
        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("rep_len(1:3, 1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("rep(1:3, times = 1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("rep(1:3, length.out = 1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("seq(1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("seq(1, 1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("seq(1, 10, length.out = 1000)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");

        let mut vm = R0Vm::new(10);
        let err = vm.eval_script("seq(1, 1000, by = 1)").unwrap_err();
        assert!(err.starts_with("ErrFuelExhausted:"), "{err}");
    }

    #[test]
    fn locked_prelude_rep_seq() {
        assert!(run("rep <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("rep_len <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
        assert!(run("seq <- function(x) x").unwrap_err().contains("Cannot shadow built-in intrinsic"));
    }

    #[test]
    fn is_finite_and_is_infinite_semantics() {
        same(
            run("is.finite(c(1.0, 0.0, -0.0, Inf, -Inf, NaN, NA_real_))"),
            R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_FALSE]),
        );
        same(
            run("is.infinite(c(1.0, 0.0, -0.0, Inf, -Inf, NaN, NA_real_))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE, LGL_FALSE, LGL_FALSE]),
        );
        same(
            run("is.finite(c(1L, 0L, -1L, NA_integer_))"),
            R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_TRUE, LGL_FALSE]),
        );
        same(
            run("is.infinite(c(1L, 0L, -1L, NA_integer_))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_FALSE]),
        );
        same(
            run("is.finite(c(TRUE, FALSE, NA))"),
            R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_FALSE]),
        );
        same(
            run("is.infinite(c(TRUE, FALSE, NA))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
        );
        same(
            run("is.finite(1:3)"),
            R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_TRUE]),
        );
        same(
            run("is.infinite(1:3)"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
        );
        same(
            run("is.finite(c(\"a\", \"b\", NA))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
        );
        same(
            run("is.infinite(c(\"a\", \"b\", NA))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE]),
        );
        same(run("is.finite(c())"), R0Val::Lgl(vec![]));
        same(run("is.infinite(c())"), R0Val::Lgl(vec![]));
        assert!(run("is.finite(list(a = 1))").unwrap_err().contains("Unsupported type"));
        assert!(run("is.infinite(data.frame(a = 1))").unwrap_err().contains("Unsupported type"));
    }

    #[test]
    fn any_and_all_kleene_three_valued_logic() {
        // GNU R truth table for any
        same(run("any(c(TRUE, TRUE))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("any(c(TRUE, FALSE))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("any(c(TRUE, NA))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("any(c(FALSE, FALSE))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("any(c(FALSE, NA))"), R0Val::Lgl(vec![LGL_NA]));
        same(run("any(c(NA, NA))"), R0Val::Lgl(vec![LGL_NA]));
        same(run("any(logical(0))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("any(c())"), R0Val::Lgl(vec![LGL_FALSE]));

        // na.rm for any
        same(run("any(c(FALSE, NA), na.rm = TRUE)"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("any(c(TRUE, NA), na.rm = TRUE)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("any(c(NA, NA), na.rm = TRUE)"), R0Val::Lgl(vec![LGL_FALSE]));

        // GNU R truth table for all
        same(run("all(c(TRUE, TRUE))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("all(c(TRUE, FALSE))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("all(c(TRUE, NA))"), R0Val::Lgl(vec![LGL_NA]));
        same(run("all(c(FALSE, FALSE))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("all(c(FALSE, NA))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("all(c(NA, NA))"), R0Val::Lgl(vec![LGL_NA]));
        same(run("all(logical(0))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("all(c())"), R0Val::Lgl(vec![LGL_TRUE]));

        // na.rm for all
        same(run("all(c(FALSE, NA), na.rm = TRUE)"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("all(c(TRUE, NA), na.rm = TRUE)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("all(c(NA, NA), na.rm = TRUE)"), R0Val::Lgl(vec![LGL_TRUE]));

        // Int, Range, and Dbl coercions
        same(run("any(c(0, 0))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("any(c(0, 1))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("any(c(0, NA_real_))"), R0Val::Lgl(vec![LGL_NA]));
        same(run("any(c(1, NA_real_))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("all(c(1, 2))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("all(c(1, 0))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("all(c(1, NA_real_))"), R0Val::Lgl(vec![LGL_NA]));
        same(run("all(c(0, NA_real_))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("any(1:3)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("all(1:3)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("any(0:2)"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("all(0:2)"), R0Val::Lgl(vec![LGL_FALSE]));

        // Type errors
        assert!(run("any(\"TRUE\")").unwrap_err().starts_with("TypeError:"));
        assert!(run("all(\"FALSE\")").unwrap_err().starts_with("TypeError:"));
        assert!(run("any(list(a = 1, b = 2))").unwrap_err().starts_with("TypeError:"));
        assert!(run("all(data.frame(a = 1))").unwrap_err().starts_with("TypeError:"));

        // na.rm argument validation
        assert!(run("any(c(TRUE, FALSE), na.rm = NA)").unwrap_err().starts_with("ErrInvalidCondition:"));
        assert!(run("all(c(TRUE, FALSE), na.rm = \"TRUE\")").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn any_na_semantics() {
        same(run("anyNA(c(1.0, 2.0))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("anyNA(c(1.0, NA_real_))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("anyNA(c(1.0, NaN))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("anyNA(c(1L, 2L))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("anyNA(c(1L, NA_integer_))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("anyNA(c(TRUE, FALSE))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("anyNA(c(TRUE, NA))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("anyNA(c(\"a\", \"b\"))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("anyNA(c(\"a\", NA))"), R0Val::Lgl(vec![LGL_TRUE]));
        same(run("anyNA(1:10)"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("anyNA(logical(0))"), R0Val::Lgl(vec![LGL_FALSE]));
        same(run("anyNA(c())"), R0Val::Lgl(vec![LGL_FALSE]));
        assert!(run("anyNA(list(a = 1))").unwrap_err().contains("Unsupported type"));
    }

    #[test]
    fn predicates_and_logical_reductions_fuel_and_locking() {
        for src in [
            "is.finite(1:2000000000)",
            "is.infinite(1:2000000000)",
            "any(1:2000000000)",
            "all(1:2000000000)",
            "anyNA(1:2000000000)",
        ] {
            let err = R0Vm::new(100_000).eval_script(src).unwrap_err();
            assert!(err.starts_with("ErrFuelExhausted:"), "{src} failed to exhaust fuel: {err}");
        }

        for func in ["is.finite", "is.infinite", "any", "all", "anyNA"] {
            let code = format!("{func} <- function(x) x");
            let err = run(&code).unwrap_err();
            assert!(
                err.contains("Cannot shadow built-in intrinsic"),
                "{func} was not locked against shadowing: {err}"
            );
        }
    }

    #[test]
    fn trig_functions_values_and_na_propagation() {
        // Values at 0
        same(run("sin(0)"), R0Val::Dbl(vec![0.0]));
        same(run("cos(0)"), R0Val::Dbl(vec![1.0]));
        same(run("tan(0)"), R0Val::Dbl(vec![0.0]));
        same(run("asin(0)"), R0Val::Dbl(vec![0.0]));
        same(run("acos(1)"), R0Val::Dbl(vec![0.0]));
        same(run("atan(0)"), R0Val::Dbl(vec![0.0]));

        // Non-zero values
        let pi = std::f64::consts::PI;
        match run("sin(c(0.0, 1.5707963267948966))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 2);
                assert!((v[0] - 0.0).abs() < 1e-12);
                assert!((v[1] - 1.0).abs() < 1e-12);
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("cos(c(0.0, 3.141592653589793))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 2);
                assert!((v[0] - 1.0).abs() < 1e-12);
                assert!((v[1] - (-1.0)).abs() < 1e-12);
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan(c(0.0, 1.0))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 2);
                assert!((v[0] - 0.0).abs() < 1e-12);
                assert!((v[1] - (pi / 4.0)).abs() < 1e-12);
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        // Domain boundaries producing NaN
        match run("asin(c(2.0, -2.0))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 2);
                assert!(v[0].is_nan() && !is_na_real(v[0]));
                assert!(v[1].is_nan() && !is_na_real(v[1]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("acos(c(2.0, -2.0))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 2);
                assert!(v[0].is_nan() && !is_na_real(v[0]));
                assert!(v[1].is_nan() && !is_na_real(v[1]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        // Int / Range promotion
        same(run("cos(0L)"), R0Val::Dbl(vec![1.0]));
        same(run("sin(0L:0L)"), R0Val::Dbl(vec![0.0]));

        // NA_real_ and NaN preservation
        for f in ["sin", "cos", "tan", "asin", "acos", "atan"] {
            let code = format!("{f}(c(0.0, NA_real_, NaN))");
            match run(&code).unwrap() {
                R0Val::Dbl(v) => {
                    assert_eq!(v.len(), 3, "failed on {f}");
                    assert!(is_na_real(v[1]), "NA not preserved in {f}");
                    assert!(!is_na_real(v[2]) && v[2].is_nan(), "NaN not preserved in {f}");
                }
                other => panic!("{f}: expected Dbl, got {:?}", other),
            }
        }

        // Empty input
        same(run("sin(c())"), R0Val::Dbl(vec![]));
        same(run("cos(numeric(0))"), R0Val::Dbl(vec![]));

        // Type errors
        assert!(run("sin(\"a\")").unwrap_err().starts_with("TypeError:"));
        assert!(run("cos(TRUE)").unwrap_err().starts_with("TypeError:"));
        assert!(run("tan(list(a = 1))").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn trunc_values_and_na_propagation() {
        same(
            run("trunc(c(1.2, 1.9, -1.2, -1.9, 0.0, -0.0))"),
            R0Val::Dbl(vec![1.0, 1.0, -1.0, -1.0, 0.0, -0.0]),
        );
        same(
            run("trunc(c(Inf, -Inf))"),
            R0Val::Dbl(vec![f64::INFINITY, f64::NEG_INFINITY]),
        );
        same(run("trunc(1:3)"), R0Val::Dbl(vec![1.0, 2.0, 3.0]));
        same(run("trunc(c(1L, -2L))"), R0Val::Dbl(vec![1.0, -2.0]));

        match run("trunc(c(1.5, NA_real_, NaN, NA))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 4);
                assert_eq!(v[0], 1.0);
                assert!(is_na_real(v[1]));
                assert!(!is_na_real(v[2]) && v[2].is_nan());
                assert!(is_na_real(v[3]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        same(run("trunc(c())"), R0Val::Dbl(vec![]));
        same(run("trunc(numeric(0))"), R0Val::Dbl(vec![]));
        assert!(run("trunc(\"abc\")").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn round_ties_even_and_digits_rules() {
        // IEEE-754 ties to even (banker's rounding) with default digits = 0
        same(
            run("round(c(0.5, 1.5, 2.5, 3.5, 4.5))"),
            R0Val::Dbl(vec![0.0, 2.0, 2.0, 4.0, 4.0]),
        );
        same(
            run("round(c(-0.5, -1.5, -2.5, -3.5, -4.5))"),
            R0Val::Dbl(vec![-0.0, -2.0, -2.0, -4.0, -4.0]),
        );

        // Positive digits
        same(run("round(c(1.2345, 1.2355), 3)"), R0Val::Dbl(vec![1.234, 1.236]));
        same(run("round(c(1.25, 1.35), 1)"), R0Val::Dbl(vec![1.2, 1.4]));

        // Negative digits
        same(
            run("round(c(1234, 1250, 1350), -2)"),
            R0Val::Dbl(vec![1200.0, 1200.0, 1400.0]),
        );
        same(run("round(c(15, 25), -1)"), R0Val::Dbl(vec![20.0, 20.0]));

        // Scalar integer / double / range digits
        same(run("round(1.5, digits = 0L)"), R0Val::Dbl(vec![2.0]));
        same(run("round(1.5, digits = 0.0)"), R0Val::Dbl(vec![2.0]));
        same(run("round(1.234, digits = 2:2)"), R0Val::Dbl(vec![1.23]));

        // Named arguments
        same(run("round(digits = 2, x = 1.234)"), R0Val::Dbl(vec![1.23]));

        // Preserves NA and NaN
        match run("round(c(1.5, NA_real_, NaN))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 3);
                assert_eq!(v[0], 2.0);
                assert!(is_na_real(v[1]));
                assert!(!is_na_real(v[2]) && v[2].is_nan());
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("round(c(1L, NA_integer_))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 2);
                assert_eq!(v[0], 1.0);
                assert!(is_na_real(v[1]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        // Infinities and empty vectors
        same(run("round(c(Inf, -Inf))"), R0Val::Dbl(vec![f64::INFINITY, f64::NEG_INFINITY]));
        same(run("round(c())"), R0Val::Dbl(vec![]));
        same(run("round(numeric(0))"), R0Val::Dbl(vec![]));

        // Digits validation errors
        assert!(run("round(1.5, digits = NA)").unwrap_err().contains("round:"));
        assert!(run("round(1.5, digits = NA_integer_)").unwrap_err().contains("round:"));
        assert!(run("round(1.5, digits = NA_real_)").unwrap_err().contains("round:"));
        assert!(run("round(1.5, digits = NaN)").unwrap_err().contains("round:"));
        assert!(run("round(1.5, digits = 1:2)").unwrap_err().contains("round:"));
        assert!(run("round(1.5, digits = \"2\")").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn atan2_quadrants_recycling_and_na() {
        let pi = std::f64::consts::PI;

        // Quadrants
        match run("atan2(c(0, 1, 0, -1, 1, -1), c(1, 0, -1, 0, 1, -1))").unwrap() {
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
        match run("atan2(y = 1, x = 0)").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 1);
                assert!((v[0] - (pi / 2.0)).abs() < 1e-12);
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan2(x = 0, y = 1)").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 1);
                assert!((v[0] - (pi / 2.0)).abs() < 1e-12);
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        // R recycling: length 4 with length 1, length 4 with length 2
        match run("atan2(c(0, 1, 0, -1), 1)").unwrap() {
            R0Val::Dbl(v) => assert_eq!(v.len(), 4),
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan2(c(0, 1, 0, -1), c(1, 0))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 4);
                assert!((v[0] - 0.0).abs() < 1e-12); // (0, 1)
                assert!((v[1] - (pi / 2.0)).abs() < 1e-12); // (1, 0)
                assert!((v[2] - 0.0).abs() < 1e-12); // (0, 1) recycled
                assert!((v[3] - (-pi / 2.0)).abs() < 1e-12); // (-1, 0) recycled
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        // Non-multiple recycling errors
        assert!(run("atan2(c(1, 2), c(1, 2, 3))").unwrap_err().starts_with("ErrDimensionMismatch:"));
        assert!(run("atan2(1:3, 1:2)").unwrap_err().starts_with("ErrDimensionMismatch:"));

        // Empty inputs
        same(run("atan2(c(), 1)"), R0Val::Dbl(vec![]));
        same(run("atan2(1, c())"), R0Val::Dbl(vec![]));
        same(run("atan2(numeric(0), numeric(0))"), R0Val::Dbl(vec![]));
        same(run("atan2(c(), c())"), R0Val::Dbl(vec![]));

        // Missing argument
        assert!(run("atan2(1)").unwrap_err().contains("atan2: missing argument 2"));

        // NA / NaN preservation & B8 priority (first operand wins)
        match run("atan2(NA_real_, 1)").unwrap() {
            R0Val::Dbl(v) => assert!(is_na_real(v[0])),
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan2(1, NA_real_)").unwrap() {
            R0Val::Dbl(v) => assert!(is_na_real(v[0])),
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan2(NaN, 1)").unwrap() {
            R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan2(1, NaN)").unwrap() {
            R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan2(NA_real_, NaN)").unwrap() {
            R0Val::Dbl(v) => assert!(is_na_real(v[0])),
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("atan2(NaN, NA_real_)").unwrap() {
            R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }
    }

    #[test]
    fn trig_and_rounding_pre_allocation_fuel_and_locking() {
        for src in [
            "sin(1:2000000000)",
            "cos(1:2000000000)",
            "tan(1:2000000000)",
            "acos(1:2000000000)",
            "asin(1:2000000000)",
            "atan(1:2000000000)",
            "trunc(1:2000000000)",
            "round(1:2000000000)",
            "atan2(1:2000000000, 1.0)",
            "atan2(1.0, 1:2000000000)",
        ] {
            let err = R0Vm::new(100_000).eval_script(src).unwrap_err();
            assert!(err.starts_with("ErrFuelExhausted:"), "{src} failed to exhaust fuel: {err}");
        }

        for func in ["sin", "cos", "tan", "acos", "asin", "atan", "trunc", "round", "atan2"] {
            let code = format!("{func} <- function(x) x");
            let err = run(&code).unwrap_err();
            assert!(
                err.contains("Cannot shadow built-in intrinsic"),
                "{func} was not locked against shadowing: {err}"
            );
        }
    }

    #[test]
    fn statistical_reductions_prod_var_sd_median() {
        // prod
        same(run("prod(c(2, 3, 4))"), R0Val::Dbl(vec![24.0]));
        same(run("prod(numeric(0))"), R0Val::Dbl(vec![1.0]));
        same(run("prod(c())"), R0Val::Dbl(vec![1.0]));
        same(run("prod(1L:4L)"), R0Val::Dbl(vec![24.0]));
        same(run("prod(c(TRUE, TRUE, TRUE))"), R0Val::Dbl(vec![1.0]));
        same(run("prod(c(TRUE, FALSE, TRUE))"), R0Val::Dbl(vec![0.0]));
        same(run("prod(c(2, NA_real_, 4))"), R0Val::Dbl(vec![na_real()]));
        same(run("prod(c(2, NA_real_, 4), na.rm = TRUE)"), R0Val::Dbl(vec![8.0]));
        same(run("prod(c(NA_real_, NA_real_), na.rm = TRUE)"), R0Val::Dbl(vec![1.0]));
        match run("prod(c(2, NaN, 4))").unwrap() {
            R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }

        // var
        same(run("var(c(1, 2, 3))"), R0Val::Dbl(vec![1.0]));
        same(run("var(c(1, 2, 3, 4, 5))"), R0Val::Dbl(vec![2.5]));
        same(run("var(1L:5L)"), R0Val::Dbl(vec![2.5]));
        same(run("var(c(5))"), R0Val::Dbl(vec![na_real()]));
        same(run("var(numeric(0))"), R0Val::Dbl(vec![na_real()]));
        same(run("var(c(1, NA_real_, 3))"), R0Val::Dbl(vec![na_real()]));
        same(run("var(c(1, NA_real_, 3), na.rm = TRUE)"), R0Val::Dbl(vec![2.0]));
        same(run("var(c(1, NA_real_), na.rm = TRUE)"), R0Val::Dbl(vec![na_real()]));
        match run("var(c(1, NaN, 3))").unwrap() {
            R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }

        // sd
        same(run("sd(c(1, 2, 3))"), R0Val::Dbl(vec![1.0]));
        match run("sd(c(1, 3))").unwrap() {
            R0Val::Dbl(v) => assert!((v[0] - std::f64::consts::SQRT_2).abs() < 1e-12),
            other => panic!("expected Dbl, got {:?}", other),
        }
        same(run("sd(c(5))"), R0Val::Dbl(vec![na_real()]));
        same(run("sd(numeric(0))"), R0Val::Dbl(vec![na_real()]));
        same(run("sd(c(1, NA_real_, 3))"), R0Val::Dbl(vec![na_real()]));
        match run("sd(c(1, NA_real_, 3), na.rm = TRUE)").unwrap() {
            R0Val::Dbl(v) => assert!((v[0] - std::f64::consts::SQRT_2).abs() < 1e-12),
            other => panic!("expected Dbl, got {:?}", other),
        }
        same(run("sd(c(1, NA_real_), na.rm = TRUE)"), R0Val::Dbl(vec![na_real()]));
        match run("sd(c(1, NaN, 3))").unwrap() {
            R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }

        // median
        same(run("median(c(1, 2, 100))"), R0Val::Dbl(vec![2.0]));
        same(run("median(c(100, 1, 2))"), R0Val::Dbl(vec![2.0]));
        same(run("median(c(1, 2, 3, 4))"), R0Val::Dbl(vec![2.5]));
        same(run("median(c(4, 1, 3, 2))"), R0Val::Dbl(vec![2.5]));
        same(run("median(c(42))"), R0Val::Dbl(vec![42.0]));
        same(run("median(numeric(0))"), R0Val::Dbl(vec![na_real()]));
        same(run("median(c(1, NA_real_, 3))"), R0Val::Dbl(vec![na_real()]));
        same(run("median(c(1, NA_real_, 3), na.rm = TRUE)"), R0Val::Dbl(vec![2.0]));
        same(run("median(c(1, 2, NA_real_, 10), na.rm = TRUE)"), R0Val::Dbl(vec![2.0]));
        same(run("median(c(1, 2, 3, NA_real_, 10), na.rm = TRUE)"), R0Val::Dbl(vec![2.5]));
        same(run("median(c(NA_real_, NA_real_), na.rm = TRUE)"), R0Val::Dbl(vec![na_real()]));
        match run("median(c(1, NaN, 3))").unwrap() {
            R0Val::Dbl(v) => assert!(!is_na_real(v[0]) && v[0].is_nan()),
            other => panic!("expected Dbl, got {:?}", other),
        }
    }

    #[test]
    fn cumulative_reductions_cumprod_cummin_cummax() {
        // cumprod
        same(run("cumprod(c(1, 2, 3, 4))"), R0Val::Dbl(vec![1.0, 2.0, 6.0, 24.0]));
        same(run("cumprod(numeric(0))"), R0Val::Dbl(vec![]));
        same(run("cumprod(c())"), R0Val::Dbl(vec![]));
        match run("cumprod(c(2, 3, NA_real_, 4, 5))").unwrap() {
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
        match run("cumprod(c(2, 3, NaN, 4, 5))").unwrap() {
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
        match run("cumprod(c(NA_real_, 1, 2))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 3);
                assert!(is_na_real(v[0]) && is_na_real(v[1]) && is_na_real(v[2]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        // cummin
        same(run("cummin(c(3, 1, 4, 1, 5, 9))"), R0Val::Dbl(vec![3.0, 1.0, 1.0, 1.0, 1.0, 1.0]));
        same(run("cummin(numeric(0))"), R0Val::Dbl(vec![]));
        same(run("cummin(c())"), R0Val::Dbl(vec![]));
        match run("cummin(c(3, 1, NA_real_, 0))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 4);
                assert_eq!(v[0], 3.0);
                assert_eq!(v[1], 1.0);
                assert!(is_na_real(v[2]));
                assert!(is_na_real(v[3]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("cummin(c(3, 1, NaN, 0))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 4);
                assert_eq!(v[0], 3.0);
                assert_eq!(v[1], 1.0);
                assert!(!is_na_real(v[2]) && v[2].is_nan());
                assert!(!is_na_real(v[3]) && v[3].is_nan());
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("cummin(c(NA_real_, 1, 2))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 3);
                assert!(is_na_real(v[0]) && is_na_real(v[1]) && is_na_real(v[2]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }

        // cummax
        same(run("cummax(c(1, 3, 2, 5, 4))"), R0Val::Dbl(vec![1.0, 3.0, 3.0, 5.0, 5.0]));
        same(run("cummax(numeric(0))"), R0Val::Dbl(vec![]));
        same(run("cummax(c())"), R0Val::Dbl(vec![]));
        match run("cummax(c(1, 3, NA_real_, 10))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 4);
                assert_eq!(v[0], 1.0);
                assert_eq!(v[1], 3.0);
                assert!(is_na_real(v[2]));
                assert!(is_na_real(v[3]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("cummax(c(1, 3, NaN, 10))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 4);
                assert_eq!(v[0], 1.0);
                assert_eq!(v[1], 3.0);
                assert!(!is_na_real(v[2]) && v[2].is_nan());
                assert!(!is_na_real(v[3]) && v[3].is_nan());
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        match run("cummax(c(NA_real_, 1, 2))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 3);
                assert!(is_na_real(v[0]) && is_na_real(v[1]) && is_na_real(v[2]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
    }

    #[test]
    fn statistical_and_cumulative_argument_validation_fuel_and_locking() {
        for src in [
            "prod(1:2000000000)",
            "var(1:2000000000)",
            "sd(1:2000000000)",
            "median(1:2000000000)",
            "cumprod(1:2000000000)",
            "cummin(1:2000000000)",
            "cummax(1:2000000000)",
        ] {
            let err = R0Vm::new(100_000).eval_script(src).unwrap_err();
            assert!(err.starts_with("ErrFuelExhausted:"), "{src} failed to exhaust fuel: {err}");
        }

        for func in ["prod", "var", "sd", "median", "cumprod", "cummin", "cummax"] {
            let code = format!("{func} <- function(x) x");
            let err = run(&code).unwrap_err();
            assert!(
                err.contains("Cannot shadow built-in intrinsic"),
                "{func} was not locked against shadowing: {err}"
            );
        }

        assert!(run("prod(1, na.rm = \"foo\")").unwrap_err().starts_with("TypeError:"));
        assert!(run("var(1, na.rm = NA)").unwrap_err().starts_with("ErrInvalidCondition:"));
        assert!(run("sd(1, na.rm = 1)").unwrap_err().starts_with("TypeError:"));
        assert!(run("median(1, unknown = 1)").unwrap_err().starts_with("UnknownArgument:"));
        assert!(run("cumprod(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("cummin(1, na.rm = TRUE)").unwrap_err().starts_with("UnknownArgument:"));
        assert!(run("cummax(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
    }

    #[test]
    fn which_min_and_which_max_semantics() {
        same(run("which.min(c(5, 3, 1, 1, 4))"), R0Val::Int(vec![3]));
        same(run("which.max(c(5, 3, 5, 1, 4))"), R0Val::Int(vec![1]));
        same(run("which.min(c(NA, 5, 2, NA, 2))"), R0Val::Int(vec![3]));
        same(run("which.max(c(NA, 1, 5, 5, 2))"), R0Val::Int(vec![3]));
        same(run("which.min(c(NaN, 10, -5))"), R0Val::Int(vec![3]));
        same(run("which.max(c(NaN, 10, -5))"), R0Val::Int(vec![2]));
        same(run("which.min(c(NA, NaN, NA_real_))"), R0Val::Int(vec![]));
        same(run("which.max(c(NA, NaN, NA_real_))"), R0Val::Int(vec![]));
        same(run("which.min(numeric(0))"), R0Val::Int(vec![]));
        same(run("which.max(integer(0))"), R0Val::Int(vec![]));
        same(run("which.min(c())"), R0Val::Int(vec![]));
        same(run("which.max(c())"), R0Val::Int(vec![]));
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("which.min(n)"), R0Val::Int(vec![]));
        same(vm.eval_script("which.max(n)"), R0Val::Int(vec![]));
        same(run("which.min(1:10)"), R0Val::Int(vec![1]));
        same(run("which.max(1:10)"), R0Val::Int(vec![10]));
        same(run("which.min(10:1)"), R0Val::Int(vec![10]));
        same(run("which.max(10:1)"), R0Val::Int(vec![1]));
        same(run("which.min(c(TRUE, FALSE, TRUE))"), R0Val::Int(vec![2]));
        same(run("which.max(c(FALSE, TRUE, FALSE))"), R0Val::Int(vec![2]));
        same(run("which.min(c(Inf, -Inf, 0))"), R0Val::Int(vec![2]));
        same(run("which.max(c(Inf, -Inf, 0))"), R0Val::Int(vec![1]));
        assert!(run("which.min(c(\"a\", \"b\"))").unwrap_err().starts_with("TypeError:"));
        assert!(run("which.max(c(\"a\", \"b\"))").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn match_semantics() {
        same(run("match(c(1, 2, 3), c(2, 3, 4))"), R0Val::Int(vec![NA_INT, 1, 2]));
        same(run("match(c(1, 2, 3), c(2, 3, 4), nomatch = 0L)"), R0Val::Int(vec![0, 1, 2]));
        same(run("match(c(1, 2, 3), c(2, 3, 4), nomatch = 0)"), R0Val::Int(vec![0, 1, 2]));
        same(run("match(c(1, 4), c(1, 2, 1))"), R0Val::Int(vec![1, NA_INT]));
        same(run("match(c(\"b\", \"x\"), c(\"a\", \"b\", \"c\"))"), R0Val::Int(vec![2, NA_INT]));
        same(run("match(c(\"a\", NA), c(\"b\", NA))"), R0Val::Int(vec![NA_INT, 2]));
        same(run("match(c(NA, NaN), c(1, NA))"), R0Val::Int(vec![2, NA_INT]));
        same(run("match(c(NA, NaN), c(NaN, 1))"), R0Val::Int(vec![NA_INT, 1]));
        same(run("match(1:3, 2:4)"), R0Val::Int(vec![NA_INT, 1, 2]));
        same(run("match(c(1, 2), numeric(0))"), R0Val::Int(vec![NA_INT, NA_INT]));
        same(run("match(numeric(0), c(1, 2))"), R0Val::Int(vec![]));
        same(run("match(c(), c(1, 2))"), R0Val::Int(vec![]));
        same(run("match(c(1, 2), c())"), R0Val::Int(vec![NA_INT, NA_INT]));
        same(run("match(c(\"a\"), c())"), R0Val::Int(vec![NA_INT]));
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("match(n, c(1, 2))"), R0Val::Int(vec![]));
        same(vm.eval_script("match(c(1, 2), n)"), R0Val::Int(vec![NA_INT, NA_INT]));
        assert!(run("match(\"a\", 1)").unwrap_err().starts_with("TypeError:"));
        assert!(run("match(1, \"a\")").unwrap_err().starts_with("TypeError:"));
    }

    #[test]
    fn unique_and_duplicated_semantics() {
        same(run("unique(c(1L, 2L, 1L, 3L, 2L))"), R0Val::Int(vec![1, 2, 3]));
        same(
            run("duplicated(c(1L, 2L, 1L, 3L, 2L))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_FALSE, LGL_TRUE]),
        );
        same(run("unique(c(1L, NA_integer_, 2L, NA_integer_))"), R0Val::Int(vec![1, NA_INT, 2]));
        same(
            run("duplicated(c(1L, NA_integer_, 2L, NA_integer_))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_TRUE]),
        );
        same(run("unique(c(TRUE, FALSE, NA, TRUE, FALSE, NA))"), R0Val::Lgl(vec![LGL_TRUE, LGL_FALSE, LGL_NA]));
        same(
            run("duplicated(c(TRUE, FALSE, NA, TRUE, FALSE, NA))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE, LGL_TRUE]),
        );
        same(
            run("unique(c(\"a\", \"b\", \"a\", NA, \"b\", NA))"),
            R0Val::Chr(vec![Some("a".into()), Some("b".into()), None]),
        );
        same(
            run("duplicated(c(\"a\", \"b\", \"a\", NA, \"b\", NA))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_FALSE, LGL_TRUE, LGL_TRUE]),
        );
        same(run("unique(1:5)"), R0Val::Range { start: 1, end: 5, step: 1 });
        same(run("duplicated(1:5)"), R0Val::Lgl(vec![LGL_FALSE; 5]));
        same(run("unique(c())"), R0Val::Null);
        same(run("duplicated(c())"), R0Val::Lgl(vec![]));
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("unique(n)"), R0Val::Null);
        same(vm.eval_script("duplicated(n)"), R0Val::Lgl(vec![]));
        same(run("duplicated(c(0.0, -0.0))"), R0Val::Lgl(vec![LGL_FALSE, LGL_TRUE]));
        same(run("unique(c(0.0, -0.0))"), R0Val::Dbl(vec![0.0]));

        match run("unique(c(1.5, 2.5, 1.5, NaN, NA_real_, NaN, NA_real_))").unwrap() {
            R0Val::Dbl(v) => {
                assert_eq!(v.len(), 4);
                assert_eq!(v[0], 1.5);
                assert_eq!(v[1], 2.5);
                assert!(v[2].is_nan() && !is_na_real(v[2]));
                assert!(is_na_real(v[3]));
            }
            other => panic!("expected Dbl, got {:?}", other),
        }
        same(
            run("duplicated(c(1.5, 2.5, 1.5, NaN, NA_real_, NaN, NA_real_))"),
            R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE]),
        );
    }

    #[test]
    fn search_and_deduplication_fuel_locking_and_arg_validation() {
        for src in [
            "which.min(1:2000000000)",
            "which.max(1:2000000000)",
            "unique(1:2000000000)",
            "duplicated(1:2000000000)",
            "match(1:100000, 1:100000)",
        ] {
            let err = R0Vm::new(100_000).eval_script(src).unwrap_err();
            assert!(err.starts_with("ErrFuelExhausted:"), "{src} failed to exhaust fuel: {err}");
        }

        for func in ["which.min", "which.max", "match", "unique", "duplicated"] {
            let code = format!("{func} <- function(x) x");
            let err = run(&code).unwrap_err();
            assert!(
                err.contains("Cannot shadow built-in intrinsic"),
                "{func} was not locked against shadowing: {err}"
            );
        }

        assert!(run("which.min(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("which.max(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("unique(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("duplicated(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("match(1)").unwrap_err().contains("missing argument 2"));
        assert!(run("match(1, 2, 3, 4)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("match(1, 2, nomatch = \"foo\")").unwrap_err().starts_with("TypeError:"));
        assert!(run("which.min(list(a = 1))").unwrap_err().starts_with("TypeError:"));
        assert!(run("unique(list(a = 1))").unwrap_err().contains("Unsupported type"));
    }

    #[test]
    fn sort_semantics() {
        same(run("sort(c(3L, 1L, 4L, 2L))"), R0Val::Int(vec![1, 2, 3, 4]));
        same(run("sort(c(3L, 1L, 4L, 2L), decreasing = TRUE)"), R0Val::Int(vec![4, 3, 2, 1]));
        same(run("sort(c(3L, 1L, 4L, 2L), TRUE)"), R0Val::Int(vec![4, 3, 2, 1]));
        same(run("sort(c(3L, NA_integer_, 1L, NA_integer_, 2L))"), R0Val::Int(vec![1, 2, 3]));
        same(run("sort(c(3.5, 1.2, 4.8, 2.1))"), R0Val::Dbl(vec![1.2, 2.1, 3.5, 4.8]));
        same(run("sort(c(3.5, 1.2, 4.8, 2.1), decreasing = TRUE)"), R0Val::Dbl(vec![4.8, 3.5, 2.1, 1.2]));
        same(run("sort(c(3.0, NA_real_, 1.0, NaN, 2.0))"), R0Val::Dbl(vec![1.0, 2.0, 3.0]));
        same(run("sort(c(TRUE, FALSE, TRUE, NA, FALSE))"), R0Val::Lgl(vec![LGL_FALSE, LGL_FALSE, LGL_TRUE, LGL_TRUE]));
        same(run("sort(c(TRUE, FALSE, TRUE, NA, FALSE), decreasing = TRUE)"), R0Val::Lgl(vec![LGL_TRUE, LGL_TRUE, LGL_FALSE, LGL_FALSE]));
        same(
            run("sort(c(\"banana\", \"apple\", \"cherry\", NA))"),
            R0Val::Chr(vec![Some("apple".into()), Some("banana".into()), Some("cherry".into())]),
        );
        same(
            run("sort(c(\"banana\", \"apple\", \"cherry\", NA), decreasing = TRUE)"),
            R0Val::Chr(vec![Some("cherry".into()), Some("banana".into()), Some("apple".into())]),
        );
        same(run("sort(1:5)"), R0Val::Range { start: 1, end: 5, step: 1 });
        same(run("sort(1:5, decreasing = TRUE)"), R0Val::Range { start: 5, end: 1, step: -1 });
        same(run("sort(5:1)"), R0Val::Range { start: 1, end: 5, step: 1 });
        same(run("sort(5:1, decreasing = TRUE)"), R0Val::Range { start: 5, end: 1, step: -1 });
        same(run("sort(numeric(0))"), R0Val::Dbl(vec![]));
        same(run("sort(integer(0))"), R0Val::Int(vec![]));
        same(run("sort(logical(0))"), R0Val::Lgl(vec![]));
        same(run("sort(character(0))"), R0Val::Chr(vec![]));
        same(run("sort(c())"), R0Val::Null);
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("sort(n)"), R0Val::Null);
        same(run("sort(c(NA, NA))"), R0Val::Lgl(vec![]));
        same(run("sort(c(NA_integer_, NA_integer_))"), R0Val::Int(vec![]));
        same(run("sort(c(NA_real_, NaN))"), R0Val::Dbl(vec![]));
        same(run("sort(c(NA_character_))"), R0Val::Chr(vec![]));
    }

    #[test]
    fn order_semantics() {
        same(run("order(c(30L, 10L, 20L))"), R0Val::Int(vec![2, 3, 1]));
        same(run("order(c(30L, 10L, 20L), decreasing = TRUE)"), R0Val::Int(vec![1, 3, 2]));
        same(run("order(c(30L, 10L, 20L), TRUE)"), R0Val::Int(vec![1, 3, 2]));
        same(run("order(c(2, 1, 2, 1))"), R0Val::Int(vec![2, 4, 1, 3]));
        same(run("order(c(2, 1, 2, 1), decreasing = TRUE)"), R0Val::Int(vec![1, 3, 2, 4]));
        same(run("order(c(1, NA, 3, NA, 2))"), R0Val::Int(vec![1, 5, 3, 2, 4]));
        same(run("order(c(1, NA, 3, NA, 2), decreasing = TRUE)"), R0Val::Int(vec![3, 5, 1, 2, 4]));
        same(run("order(c(NaN, 1, NA, 2))"), R0Val::Int(vec![2, 4, 1, 3]));
        same(run("order(c(NaN, 1, NA, 2), decreasing = TRUE)"), R0Val::Int(vec![4, 2, 1, 3]));
        same(run("order(c(TRUE, FALSE, NA))"), R0Val::Int(vec![2, 1, 3]));
        same(run("order(c(TRUE, FALSE, NA), decreasing = TRUE)"), R0Val::Int(vec![1, 2, 3]));
        same(run("order(c(\"b\", \"a\", NA, \"c\"))"), R0Val::Int(vec![2, 1, 4, 3]));
        same(run("order(c(\"b\", \"a\", NA, \"c\"), decreasing = TRUE)"), R0Val::Int(vec![4, 1, 2, 3]));
        same(run("order(1:5)"), R0Val::Int(vec![1, 2, 3, 4, 5]));
        same(run("order(1:5, decreasing = TRUE)"), R0Val::Int(vec![5, 4, 3, 2, 1]));
        same(run("order(5:1)"), R0Val::Int(vec![5, 4, 3, 2, 1]));
        same(run("order(5:1, decreasing = TRUE)"), R0Val::Int(vec![1, 2, 3, 4, 5]));
        same(run("order(numeric(0))"), R0Val::Int(vec![]));
        same(run("order(c())"), R0Val::Int(vec![]));
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("order(n)"), R0Val::Int(vec![]));
        same(run("order(c(NA, NA))"), R0Val::Int(vec![1, 2]));
    }

    #[test]
    fn rank_semantics() {
        same(run("rank(c(30, 10, 20))"), R0Val::Dbl(vec![3.0, 1.0, 2.0]));
        same(run("rank(c(3, 1, 4, 1, 5))"), R0Val::Dbl(vec![3.0, 1.5, 4.0, 1.5, 5.0]));
        same(run("rank(c(10, 20, 20, 20, 30))"), R0Val::Dbl(vec![1.0, 3.0, 3.0, 3.0, 5.0]));
        same(run("rank(c(10, NA, 20, NA, 10))"), R0Val::Dbl(vec![1.5, 4.0, 3.0, 5.0, 1.5]));
        same(run("rank(c(NaN, 1, NA, 2))"), R0Val::Dbl(vec![3.0, 1.0, 4.0, 2.0]));
        same(run("rank(c(NA, NA))"), R0Val::Dbl(vec![1.0, 2.0]));
        same(run("rank(1:5)"), R0Val::Dbl(vec![1.0, 2.0, 3.0, 4.0, 5.0]));
        same(run("rank(5:1)"), R0Val::Dbl(vec![5.0, 4.0, 3.0, 2.0, 1.0]));
        same(run("rank(c(\"b\", \"a\", \"b\"))"), R0Val::Dbl(vec![2.5, 1.0, 2.5]));
        same(run("rank(c(TRUE, FALSE, TRUE))"), R0Val::Dbl(vec![2.5, 1.0, 2.5]));
        same(run("rank(numeric(0))"), R0Val::Dbl(vec![]));
        same(run("rank(c())"), R0Val::Dbl(vec![]));
        let mut vm = vm_with(vec![("n", R0Val::Null)]);
        same(vm.eval_script("rank(n)"), R0Val::Dbl(vec![]));
    }

    #[test]
    fn sorting_and_ranking_fuel_locking_and_arg_validation() {
        for src in [
            "sort(1:2000000000)",
            "order(1:2000000000)",
            "rank(1:2000000000)",
        ] {
            let err = R0Vm::new(100_000).eval_script(src).unwrap_err();
            assert!(err.starts_with("ErrFuelExhausted:"), "{src} failed to exhaust fuel: {err}");
        }

        for func in ["sort", "order", "rank"] {
            let code = format!("{func} <- function(x) x");
            let err = run(&code).unwrap_err();
            assert!(
                err.contains("Cannot shadow built-in intrinsic"),
                "{func} was not locked against shadowing: {err}"
            );
        }

        assert!(run("sort(1, 2, 3)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("order(1, 2, 3)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("rank(1, 2)").unwrap_err().starts_with("TooManyArguments:"));
        assert!(run("sort(1, decreasing = \"foo\")").unwrap_err().starts_with("TypeError:"));
        assert!(run("sort(1, decreasing = NA)").unwrap_err().starts_with("ErrInvalidCondition:"));
        assert!(run("order(1, decreasing = 1)").unwrap_err().starts_with("TypeError:"));
        assert!(run("sort(list(a = 1))").unwrap_err().starts_with("TypeError:"));
        assert!(run("order(list(a = 1))").unwrap_err().starts_with("TypeError:"));
        assert!(run("rank(list(a = 1))").unwrap_err().starts_with("TypeError:"));
    }
}

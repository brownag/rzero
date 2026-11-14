//! Virtual machine that evaluates the AST.

use std::collections::HashMap;

use crate::ast::{BinOp, Expr, UnaryOp};
use crate::lexer::tokenize;
use crate::parser::Parser;
use crate::value::{
    is_missing, is_na_real, na_real, R0Val, KLEENE_AND_LUT, KLEENE_NOT_LUT, KLEENE_OR_LUT, LGL_FALSE,
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
        self.fuel -= units.max(1) as i64;
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
            "sum" | "mean" | "min" | "max" => (&["x", "na.rm"], 1),
            "%in%" => (&["x", "table"], 2),
            "length" | "seq_len" | "seq_along" | "is.na" | "is.nan" | "which" | "abs" | "sqrt" | "exp" | "log"
            | "log10" | "floor" | "ceiling" | "cumsum" | "diff" => (&["x"], 1),
            _ => return Err(format!("Unknown function or intrinsic: '{}'", func)),
        };
        let mut s = Self::bind_args(func, args, params, npos)?;
        match func {
            "length" => Ok(R0Val::Int(vec![Self::req(&mut s, 0, func)?.len() as i32])),
            "seq_len" | "seq_along" => {
                let x = Self::req(&mut s, 0, func)?;
                let n = if func == "seq_len" {
                    self.consume_fuel(x.len())?;
                    x.materialize_int()?.first().copied().ok_or("seq_len requires a length-1 argument")?
                } else {
                    x.len() as i32
                };
                Ok(R0Val::Range { start: 1, end: n, step: 1 })
            }
            "is.nan" => {
                let x = Self::req(&mut s, 0, func)?;
                match &x {
                    R0Val::Dbl(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if d.is_nan() && !is_na_real(d) { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Int(_) | R0Val::Range { .. } => Ok(R0Val::Lgl(vec![LGL_FALSE; x.len()])),
                    _ => Err("Unsupported type for is.nan()".into()),
                }
            }
            "is.na" => {
                let x = Self::req(&mut s, 0, func)?;
                match &x {
                    R0Val::Dbl(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if is_missing(d) { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Int(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if d == NA_INT { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Lgl(v) => Ok(R0Val::Lgl(v.iter().map(|&d| if d == LGL_NA { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Chr(v) => Ok(R0Val::Lgl(v.iter().map(|s| if s.is_none() { LGL_TRUE } else { LGL_FALSE }).collect())),
                    R0Val::Range { .. } => Ok(R0Val::Lgl(vec![LGL_FALSE; x.len()])),
                    _ => Err("Unsupported type for is.na()".into()),
                }
            }
            "which" => match Self::req(&mut s, 0, func)? {
                R0Val::Lgl(mask) => Ok(R0Val::Int(
                    mask.iter()
                        .enumerate()
                        .filter(|(_, b)| **b == LGL_TRUE)
                        .map(|(i, _)| (i + 1) as i32)
                        .collect(),
                )),
                _ => Err("which() requires a logical (lgl) vector".into()),
            },
            "ifelse" => {
                let test = Self::req(&mut s, 0, func)?;
                let yes = Self::req(&mut s, 1, func)?;
                let no = Self::req(&mut s, 2, func)?;
                let R0Val::Lgl(cond) = test else {
                    return Err("ifelse first argument must be logical (lgl)".into());
                };
                let n = cond.len();
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
                let na_rm = matches!(s[2].take(), Some(R0Val::Lgl(v)) if v.first() == Some(&LGL_TRUE));
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
            "abs" => match Self::req(&mut s, 0, func)? {
                // Int in, Int out (R behavior). NA_INT is i32::MIN, so abs never overflows.
                R0Val::Int(v) => Ok(R0Val::Int(v.iter().map(|&x| if x == NA_INT { NA_INT } else { x.abs() }).collect())),
                x => {
                    self.consume_fuel(x.len())?;
                    Ok(R0Val::Dbl(self.as_f64(&x)?.into_iter().map(|v| if is_missing(v) { v } else { v.abs() }).collect()))
                }
            },
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
            "sum" | "mean" | "min" | "max" => {
                let x = Self::req(&mut s, 0, func)?;
                let na_rm = matches!(s[1].take(), Some(R0Val::Lgl(v)) if v.first() == Some(&LGL_TRUE));
                self.reduce(func, &x, na_rm)
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
                match (&x, &table) {
                    (R0Val::Chr(xv), R0Val::Chr(tv)) => Ok(R0Val::Lgl(
                        xv.iter().map(|e| if tv.contains(e) { LGL_TRUE } else { LGL_FALSE }).collect(),
                    )),
                    (R0Val::Chr(_), _) | (_, R0Val::Chr(_)) => {
                        Err("TypeError: %in% cannot compare character with non-character values".into())
                    }
                    _ => {
                        self.consume_fuel(x.len().max(table.len()))?;
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
    fn range_subsetting_charges_fuel_before_materialization() {
        let err = R0Vm::new(100_000).eval_script("x <- 1:2000000000; x[1]").unwrap_err();
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
    fn r_mirror_min_max_na_beats_nan_and_sum_first_missing() {
        assert_eq!(dbl_cls("min(c(NaN, NA_real_))"), vec!["NA"]);
        assert_eq!(dbl_cls("max(c(NA_real_, NaN))"), vec!["NA"]);
        assert_eq!(dbl_cls("min(c(NaN, 1))"), vec!["NaN"]);
        assert_eq!(dbl_cls("sum(c(NaN, NA_real_))"), vec!["NaN"]);
        assert_eq!(dbl_cls("sum(c(NA_real_, NaN))"), vec!["NA"]);
        assert_eq!(dbl_cls("mean(c(NaN, 1))"), vec!["NaN"]);
    }
}

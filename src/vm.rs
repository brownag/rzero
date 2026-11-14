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
        let owned;
        let base = match base {
            R0Val::Range { .. } => {
                owned = R0Val::Int(base.materialize_int()?); // Range subsetting returns Int
                &owned
            }
            R0Val::Df { .. } | R0Val::List { .. } => {
                return Err("TypeError: '[' requires an atomic vector; use '[[' or '$' on df/list".into())
            }
            R0Val::Null => return Ok(R0Val::Null),
            _ => base,
        };
        let n = base.len();
        self.consume_fuel(idx.len().max(n))?;
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
                let indices = idx.materialize_int()?;
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
        let sub = idx.materialize_int()?;
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
        let target = match base {
            R0Val::Range { .. } => R0Val::Int(base.materialize_int()?),
            R0Val::Df { .. } | R0Val::List { .. } | R0Val::Null => {
                return Err("TypeError: cannot assign into NULL, df, or list with '['".into())
            }
            t => t,
        };
        let rhs = match val {
            R0Val::Range { .. } => R0Val::Int(val.materialize_int()?),
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
                .collect(), // OOB ignored (Q3)
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
    fn call_intrinsic_still_stubbed() {
        assert!(run("sum(1)").unwrap_err().starts_with("ErrNotImplemented: call_intrinsic"));
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
        same(vm.eval_script("x[99L] <- 5; x"), R0Val::Dbl(vec![1.0, 9.0, 3.0]));
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
}

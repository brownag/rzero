//! Parser producing the AST from tokens.

use crate::ast::{BinOp, Expr, UnaryOp};
use crate::lexer::Tok;

pub struct Parser {
    tokens: Vec<Tok>,
    pos: usize,
}

impl Parser {
    pub fn new(tokens: Vec<Tok>) -> Self {
        Self { tokens, pos: 0 }
    }

    fn peek(&self) -> &Tok {
        self.tokens.get(self.pos).unwrap_or(&Tok::Eof)
    }

    fn next(&mut self) -> Tok {
        let t = self.peek().clone();
        if self.pos < self.tokens.len() {
            self.pos += 1;
        }
        t
    }

    fn eat_terms(&mut self) {
        while matches!(self.peek(), Tok::Term) {
            self.next();
        }
    }

    fn expect(&mut self, expected: &Tok) -> Result<(), String> {
        let got = self.next();
        if &got == expected {
            Ok(())
        } else {
            Err(format!("Expected {:?}, got {:?}", expected, got))
        }
    }

    pub fn parse_program(&mut self) -> Result<Expr, String> {
        let mut stmts = Vec::new();
        self.eat_terms();
        while !matches!(self.peek(), Tok::Eof) {
            stmts.push(self.parse_expr(0)?);
            self.eat_terms();
        }
        Ok(Expr::Block(stmts))
    }

    fn parse_block(&mut self) -> Result<Expr, String> {
        self.expect(&Tok::LBrace)?;
        let mut stmts = Vec::new();
        self.eat_terms();
        while !matches!(self.peek(), Tok::RBrace | Tok::Eof) {
            stmts.push(self.parse_expr(0)?);
            self.eat_terms();
        }
        self.expect(&Tok::RBrace)?;
        Ok(Expr::Block(stmts))
    }

    fn parse_expr(&mut self, min_bp: u8) -> Result<Expr, String> {
        let mut lhs = self.parse_prefix()?;
        loop {
            let op = self.peek();
            // Postfix operators: `[`, `[[`, `$`
            if let Some(l_bp) = postfix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                match self.next() {
                    Tok::LBrack => {
                        let first = if matches!(self.peek(), Tok::Comma) {
                            None
                        } else {
                            Some(Box::new(self.parse_expr(0)?))
                        };
                        if matches!(self.peek(), Tok::Comma) {
                            self.next();
                            let second = if matches!(self.peek(), Tok::RBrack) {
                                None
                            } else {
                                Some(Box::new(self.parse_expr(0)?))
                            };
                            if matches!(self.peek(), Tok::Comma) {
                                return Err("Too many subscripts: '[' accepts at most 2".into());
                            }
                            self.expect(&Tok::RBrack)?;
                            lhs = Expr::Index2 {
                                target: Box::new(lhs),
                                i: first,
                                j: second,
                            };
                        } else {
                            let idx = first.ok_or("Empty subscript")?;
                            self.expect(&Tok::RBrack)?;
                            lhs = Expr::Index1 {
                                target: Box::new(lhs),
                                idx,
                            };
                        }
                    }
                    Tok::LBrack2 => {
                        let idx = self.parse_expr(0)?;
                        self.expect(&Tok::RBrack2)?;
                        lhs = Expr::IndexScalar {
                            target: Box::new(lhs),
                            idx: Box::new(idx),
                        };
                    }
                    Tok::Dollar => {
                        if let Tok::Ident(field) = self.next() {
                            lhs = Expr::Field {
                                target: Box::new(lhs),
                                name: field,
                            };
                        } else {
                            return Err("Expected column identifier after '$'".into());
                        }
                    }
                    _ => unreachable!(),
                }
                continue;
            }
            // Infix operators
            if let Some((l_bp, r_bp)) = infix_bp(op) {
                if l_bp < min_bp {
                    break;
                }
                let op_tok = self.next();
                let rhs = self.parse_expr(r_bp)?;
                lhs = match op_tok {
                    Tok::Assign => Expr::Assign {
                        target: Box::new(lhs),
                        value: Box::new(rhs),
                    },
                    Tok::Pipe => desugar_pipe(lhs, rhs)?,
                    Tok::InOp => Expr::Call {
                        func: "%in%".into(),
                        args: vec![(None, lhs), (None, rhs)],
                    },
                    _ => {
                        let bin_op = match op_tok {
                            Tok::Plus => BinOp::Add,
                            Tok::Minus => BinOp::Sub,
                            Tok::Star => BinOp::Mul,
                            Tok::Slash => BinOp::Div,
                            Tok::Caret => BinOp::Pow,
                            Tok::Mod => BinOp::Mod,
                            Tok::IntDiv => BinOp::IntDiv,
                            Tok::Colon => BinOp::Colon,
                            Tok::Eq => BinOp::Eq,
                            Tok::Ne => BinOp::Ne,
                            Tok::Lt => BinOp::Lt,
                            Tok::Le => BinOp::Le,
                            Tok::Gt => BinOp::Gt,
                            Tok::Ge => BinOp::Ge,
                            Tok::And => BinOp::And,
                            Tok::And2 => BinOp::And2,
                            Tok::Or => BinOp::Or,
                            Tok::Or2 => BinOp::Or2,
                            _ => unreachable!(),
                        };
                        Expr::Binary(bin_op, Box::new(lhs), Box::new(rhs))
                    }
                };
                continue;
            }
            break;
        }
        Ok(lhs)
    }

    fn parse_prefix(&mut self) -> Result<Expr, String> {
        match self.next() {
            Tok::DblLit(v) => Ok(Expr::Dbl(v)),
            Tok::IntLit(v) => Ok(Expr::Int(v)),
            Tok::LglLit(v) => Ok(Expr::Lgl(v)),
            Tok::StrLit(s) => Ok(Expr::Str(s)),
            Tok::NaChr => Ok(Expr::ChrNA),
            Tok::Placeholder => Ok(Expr::Placeholder),
            Tok::LParen => {
                let e = self.parse_expr(0)?;
                self.expect(&Tok::RParen)?;
                Ok(e)
            }
            Tok::LBrace => {
                self.pos -= 1;
                self.parse_block()
            }
            Tok::Minus => Ok(Expr::Unary(UnaryOp::Neg, Box::new(self.parse_expr(20)?))),
            Tok::Plus => Ok(Expr::Unary(UnaryOp::Pos, Box::new(self.parse_expr(20)?))),
            Tok::Bang => Ok(Expr::Unary(UnaryOp::Not, Box::new(self.parse_expr(7)?))),
            Tok::Break => Ok(Expr::Break),
            Tok::Next => Ok(Expr::Next),
            Tok::Return => {
                self.expect(&Tok::LParen)?;
                let e = if matches!(self.peek(), Tok::RParen) {
                    Expr::Null
                } else {
                    self.parse_expr(0)?
                };
                self.expect(&Tok::RParen)?;
                Ok(Expr::Return(Box::new(e)))
            }
            Tok::If => {
                self.expect(&Tok::LParen)?;
                let cond = self.parse_expr(0)?;
                self.expect(&Tok::RParen)?;
                let then_br = self.parse_expr(0)?;
                self.eat_terms();
                let else_br = if matches!(self.peek(), Tok::Else) {
                    self.next();
                    Some(Box::new(self.parse_expr(0)?))
                } else {
                    None
                };
                Ok(Expr::If {
                    cond: Box::new(cond),
                    then_br: Box::new(then_br),
                    else_br,
                })
            }
            Tok::For => {
                self.expect(&Tok::LParen)?;
                let var = match self.next() {
                    Tok::Ident(s) => s,
                    t => return Err(format!("Expected loop variable, got {:?}", t)),
                };
                self.expect(&Tok::In)?;
                let iter = self.parse_expr(0)?;
                self.expect(&Tok::RParen)?;
                let body = self.parse_expr(0)?;
                Ok(Expr::For {
                    var,
                    iter: Box::new(iter),
                    body: Box::new(body),
                })
            }
            Tok::While => {
                self.expect(&Tok::LParen)?;
                let cond = self.parse_expr(0)?;
                self.expect(&Tok::RParen)?;
                let body = self.parse_expr(0)?;
                Ok(Expr::While {
                    cond: Box::new(cond),
                    body: Box::new(body),
                })
            }
            Tok::Fn => {
                self.expect(&Tok::LParen)?;
                let mut params = Vec::new();
                while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                    let pname = match self.next() {
                        Tok::Ident(s) => s,
                        t => return Err(format!("Expected param name, got {:?}", t)),
                    };
                    // Optional `: type` annotation (`top: dbl`)
                    if matches!(self.peek(), Tok::Colon) {
                        self.next();
                        let _type_name = self.next();
                    }
                    let default = if matches!(self.peek(), Tok::Assign) {
                        self.next();
                        Some(self.parse_expr(0)?)
                    } else {
                        None
                    };
                    params.push((pname, default));
                    if matches!(self.peek(), Tok::Comma) {
                        self.next();
                    }
                }
                self.expect(&Tok::RParen)?;
                // Optional return type annotation `-> dbl`
                if matches!(self.peek(), Tok::Arrow) {
                    self.next();
                    let _ret_type = self.next();
                }
                let body = self.parse_expr(0)?;
                Ok(Expr::FnDef {
                    params,
                    body: Box::new(body),
                })
            }
            Tok::Ident(name) => {
                if matches!(self.peek(), Tok::LParen) {
                    self.next();
                    let mut args = Vec::new();
                    while !matches!(self.peek(), Tok::RParen | Tok::Eof) {
                        // Check named argument `na.rm = TRUE`
                        let arg = if let Tok::Ident(k) = self.peek().clone() {
                            if self.tokens.get(self.pos + 1) == Some(&Tok::Assign) {
                                self.next();
                                self.next();
                                (Some(k), self.parse_expr(0)?)
                            } else {
                                (None, self.parse_expr(0)?)
                            }
                        } else {
                            (None, self.parse_expr(0)?)
                        };
                        args.push(arg);
                        if matches!(self.peek(), Tok::Comma) {
                            self.next();
                        }
                    }
                    self.expect(&Tok::RParen)?;
                    Ok(Expr::Call { func: name, args })
                } else {
                    Ok(Expr::Ident(name))
                }
            }
            other => Err(format!("Unexpected token in expression: {:?}", other)),
        }
    }
}

fn postfix_bp(t: &Tok) -> Option<u8> {
    match t {
        Tok::LBrack | Tok::LBrack2 | Tok::Dollar => Some(21),
        _ => None,
    }
}

fn infix_bp(t: &Tok) -> Option<(u8, u8)> {
    match t {
        Tok::Assign => Some((2, 1)), // Right-associative
        Tok::Or | Tok::Or2 => Some((3, 4)),
        Tok::And | Tok::And2 => Some((5, 6)),
        Tok::Eq | Tok::Ne | Tok::Lt | Tok::Le | Tok::Gt | Tok::Ge => Some((9, 10)),
        Tok::Pipe => Some((11, 12)),
        Tok::Plus | Tok::Minus => Some((13, 14)),
        Tok::Star | Tok::Slash => Some((15, 16)),
        Tok::Mod | Tok::IntDiv | Tok::InOp => Some((17, 18)),
        Tok::Colon => Some((19, 20)),
        Tok::Caret => Some((20, 19)), // Right-associative
        _ => None,
    }
}

/// Rewrites `lhs |> f(a, b)` -> `f(lhs, a, b)` or replaces every `_` placeholder with `lhs`
fn desugar_pipe(lhs: Expr, rhs: Expr) -> Result<Expr, String> {
    match rhs {
        Expr::Call { func, mut args } => {
            let mut replaced = false;
            for (_, expr) in args.iter_mut() {
                if matches!(expr, Expr::Placeholder) {
                    *expr = lhs.clone();
                    replaced = true;
                }
            }
            if !replaced {
                args.insert(0, (None, lhs));
            }
            Ok(Expr::Call { func, args })
        }
        _ => Err("Right-hand side of '|>' pipe must be a function call".into()),
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::lexer::tokenize;

    fn parse(src: &str) -> Result<String, String> {
        let toks = tokenize(src).unwrap();
        Parser::new(toks)
            .parse_program()
            .map(|e| format!("{:?}", e))
    }

    #[test]
    fn precedence_mul_over_add() {
        assert_eq!(
            parse("1 + 2 * 3").unwrap(),
            "Block([Binary(Add, Dbl(1.0), Binary(Mul, Dbl(2.0), Dbl(3.0)))])"
        );
    }

    #[test]
    fn unary_minus_below_pow() {
        assert_eq!(
            parse("-2^2").unwrap(),
            "Block([Unary(Neg, Binary(Pow, Dbl(2.0), Dbl(2.0)))])"
        );
    }

    #[test]
    fn unary_minus_above_colon() {
        assert_eq!(
            parse("-1:3").unwrap(),
            "Block([Binary(Colon, Unary(Neg, Dbl(1.0)), Dbl(3.0))])"
        );
    }

    #[test]
    fn assign_right_assoc() {
        assert_eq!(
            parse("a <- b <- 1").unwrap(),
            "Block([Assign { target: Ident(\"a\"), value: Assign { target: Ident(\"b\"), value: Dbl(1.0) } }])"
        );
    }

    #[test]
    fn pipe_inserts_lhs() {
        assert_eq!(
            parse("x |> f(y)").unwrap(),
            "Block([Call { func: \"f\", args: [(None, Ident(\"x\")), (None, Ident(\"y\"))] }])"
        );
    }

    #[test]
    fn pipe_placeholder_named() {
        assert_eq!(
            parse("x |> f(y, z = _)").unwrap(),
            "Block([Call { func: \"f\", args: [(None, Ident(\"y\")), (Some(\"z\"), Ident(\"x\"))] }])"
        );
    }

    #[test]
    fn pipe_non_call_err() {
        assert_eq!(
            parse("x |> 1").unwrap_err(),
            "Right-hand side of '|>' pipe must be a function call"
        );
    }

    #[test]
    fn in_desugars_to_call() {
        assert_eq!(
            parse("x %in% t").unwrap(),
            "Block([Call { func: \"%in%\", args: [(None, Ident(\"x\")), (None, Ident(\"t\"))] }])"
        );
    }
}

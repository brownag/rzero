//! Abstract syntax tree.

use crate::value::Lgl;

// ============================================================================
// 3. AST & PRATT PARSER WITH PIPE (`|>`) DESUGARING
// ============================================================================
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum BinOp {
    Add,
    Sub,
    Mul,
    Div,
    Pow,
    Mod,
    IntDiv,
    Colon,
    InOp,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    And,
    And2,
    Or,
    Or2,
}
#[derive(Clone, Debug)]
pub enum UnaryOp {
    Neg,
    Pos,
    Not,
}
#[derive(Clone, Debug)]
pub enum Expr {
    Dbl(f64),
    Int(i32),
    Lgl(Lgl),
    Str(String),
    ChrNA,
    Null,
    Ident(String),
    Placeholder,
    Unary(UnaryOp, Box<Expr>),
    Binary(BinOp, Box<Expr>, Box<Expr>),
    Assign {
        target: Box<Expr>,
        value: Box<Expr>,
    },
    Index1 {
        target: Box<Expr>,
        idx: Box<Expr>,
    },
    Index2 {
        target: Box<Expr>,
        i: Option<Box<Expr>>,
        j: Option<Box<Expr>>,
    },
    IndexScalar {
        target: Box<Expr>,
        idx: Box<Expr>,
    },
    Field {
        target: Box<Expr>,
        name: String,
    },
    Call {
        func: Box<Expr>,
        args: Vec<(Option<String>, Expr)>,
    },
    FnDef {
        params: Vec<(String, Option<Expr>)>,
        body: Box<Expr>,
    },
    If {
        cond: Box<Expr>,
        then_br: Box<Expr>,
        else_br: Option<Box<Expr>>,
    },
    For {
        var: String,
        iter: Box<Expr>,
        body: Box<Expr>,
    },
    While {
        cond: Box<Expr>,
        body: Box<Expr>,
    },
    Return(Box<Expr>),
    Break,
    Next,
    Block(Vec<Expr>),
}

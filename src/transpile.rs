//! Token-level transpiler from R0 source to GNU R source.

use crate::lexer::{tokenize, Tok};
use crate::value::{is_na_real, LGL_FALSE, LGL_TRUE, NA_INT};

fn num(x: f64) -> String {
    if is_na_real(x) {
        "NA_real_".into()
    } else if x.is_nan() {
        "NaN".into()
    } else if x.is_infinite() {
        if x > 0.0 { "Inf" } else { "-Inf" }.into()
    } else if x.fract() == 0.0 && x.abs() < 1e15 {
        format!("{}", x as i64)
    } else {
        format!("{:?}", x)
    }
}

fn text(t: &Tok, in_call: bool) -> String {
    match t {
        Tok::DblLit(x) => num(*x),
        Tok::IntLit(i) if *i == NA_INT => "NA_integer_".into(),
        Tok::IntLit(i) => format!("{}L", i),
        Tok::LglLit(v) if *v == LGL_TRUE => "TRUE".into(),
        Tok::LglLit(v) if *v == LGL_FALSE => "FALSE".into(),
        Tok::LglLit(_) => "NA".into(),
        Tok::StrLit(s) => format!("\"{}\"", s.replace('\\', "\\\\").replace('"', "\\\"")),
        Tok::NaChr => "NA_character_".into(),
        Tok::Ident(s) => s.clone(),
        Tok::Fn => "function".into(),
        Tok::If => "if".into(),
        Tok::Else => "else".into(),
        Tok::For => "for".into(),
        Tok::In => "in".into(),
        Tok::While => "while".into(),
        Tok::Return => "return".into(),
        Tok::Break => "break".into(),
        Tok::Next => "next".into(),
        Tok::Assign => if in_call { "=" } else { "<-" }.into(),
        Tok::Arrow => "->".into(),
        // ponytail: `%>%` also lexes to Pipe, so it is emitted as `|>`
        Tok::Pipe => "|>".into(),
        Tok::InOp => "%in%".into(),
        Tok::Mod => "%%".into(),
        Tok::IntDiv => "%/%".into(),
        Tok::Colon => ":".into(),
        Tok::Plus => "+".into(),
        Tok::Minus => "-".into(),
        Tok::Star => "*".into(),
        Tok::Slash => "/".into(),
        Tok::Caret => "^".into(),
        Tok::Eq => "==".into(),
        Tok::Ne => "!=".into(),
        Tok::Lt => "<".into(),
        Tok::Le => "<=".into(),
        Tok::Gt => ">".into(),
        Tok::Ge => ">=".into(),
        Tok::Bang => "!".into(),
        Tok::And => "&".into(),
        Tok::And2 => "&&".into(),
        Tok::Or => "|".into(),
        Tok::Or2 => "||".into(),
        Tok::Dollar => "$".into(),
        Tok::Placeholder => "_".into(),
        Tok::LParen => "(".into(),
        Tok::RParen => ")".into(),
        Tok::LBrack => "[".into(),
        Tok::RBrack => "]".into(),
        Tok::LBrack2 => "[[".into(),
        Tok::RBrack2 => "]]".into(),
        Tok::LBrace => "{".into(),
        Tok::RBrace => "}".into(),
        Tok::Comma => ",".into(),
        Tok::Term | Tok::Eof => "\n".into(),
    }
}

/// Rewrite R0 source (`fn`, `: type`, `-> type`) into GNU R source.
pub fn transpile_to_r(src: &str) -> Result<String, String> {
    let toks = tokenize(src)?;
    let mut out = String::new();
    // Open delimiters: '(' '[' '{'
    let mut stack: Vec<char> = Vec::new();
    // Stack depths of open `fn` parameter lists
    let mut headers: Vec<usize> = Vec::new();
    let mut pending_fn = false;
    let mut name_pos = false;
    let mut just_named = false;
    let mut prev: Option<&Tok> = None;
    let mut i = 0;
    while i < toks.len() {
        let t = &toks[i];
        i += 1;
        let at_header = headers.last() == Some(&stack.len());
        let named = std::mem::take(&mut just_named);
        match t {
            Tok::Colon if at_header && named => {
                i += 1; // drop the type token
                continue;
            }
            Tok::Ident(_) if at_header && name_pos => just_named = true,
            _ => {}
        }
        if !matches!(t, Tok::Comma) {
            name_pos = false;
        }
        match t {
            Tok::Fn => pending_fn = true,
            Tok::LParen | Tok::LBrack | Tok::LBrack2 | Tok::LBrace => {
                stack.push(match t {
                    Tok::LParen => '(',
                    Tok::LBrace => '{',
                    _ => '[',
                });
                if matches!(t, Tok::LParen) && std::mem::take(&mut pending_fn) {
                    headers.push(stack.len());
                    name_pos = true;
                }
            }
            Tok::Comma if at_header => name_pos = true,
            Tok::RParen | Tok::RBrack | Tok::RBrack2 | Tok::RBrace => {
                if matches!(t, Tok::RParen) && at_header {
                    headers.pop();
                    stack.pop();
                    out.push(')');
                    prev = Some(t);
                    if toks.get(i) == Some(&Tok::Arrow) {
                        i += 2; // drop `-> type`
                    }
                    continue;
                }
                stack.pop();
            }
            _ => {}
        }
        let in_call = matches!(stack.last(), Some('(' | '['));
        // `=`/`<-` is emitted after popping/pushing, so use the enclosing bracket
        let s = text(t, in_call);
        let tight_before = matches!(
            t,
            Tok::RParen
                | Tok::RBrack
                | Tok::RBrack2
                | Tok::Comma
                | Tok::Colon
                | Tok::Term
                | Tok::Eof
                | Tok::LBrack
                | Tok::LBrack2
        ) || (matches!(t, Tok::LParen)
            && matches!(
                prev,
                Some(Tok::Fn | Tok::Ident(_) | Tok::RParen | Tok::RBrack | Tok::RBrack2)
            ));
        let tight_after_prev = matches!(
            prev,
            None | Some(
                Tok::LParen | Tok::LBrack | Tok::LBrack2 | Tok::Colon | Tok::Term | Tok::Dollar
            )
        );
        if !(tight_before || tight_after_prev) {
            out.push(' ');
        }
        out.push_str(&s);
        prev = Some(t);
    }
    Ok(out.trim().to_string())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn t(s: &str) -> String {
        transpile_to_r(s).unwrap()
    }

    #[test]
    fn fn_keyword() {
        assert_eq!(t("f <- fn(x) x + 1"), "f <- function(x) x + 1");
        assert_eq!(t("f <- function(x) x"), "f <- function(x) x");
    }

    #[test]
    fn param_types() {
        assert_eq!(t("fn(x: dbl, y: int) x"), "function(x, y) x");
    }

    #[test]
    fn return_type() {
        assert_eq!(t("fn(x) -> dbl x"), "function(x) x");
    }

    #[test]
    fn default_args() {
        assert_eq!(t("fn(x: dbl = 1, y = 2) x"), "function(x = 1, y = 2) x");
        assert_eq!(t("fn(x: int = 1:3) x"), "function(x = 1:3) x");
    }

    #[test]
    fn seq_and_pipe_intact() {
        assert_eq!(t("1:10"), "1:10");
        assert_eq!(t("x |> f()"), "x |> f()");
    }

    #[test]
    fn list_of_fn() {
        assert_eq!(
            t("list(f = fn(x: dbl) -> dbl x)"),
            "list(f = function(x) x)"
        );
    }

    #[test]
    fn newlines_and_assign() {
        assert_eq!(t("x <- 1\ny = 2.5"), "x <- 1\ny <- 2.5");
    }
}

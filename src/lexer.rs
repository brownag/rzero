//! Tokenizer for the R subset.

use crate::value::{na_real, Lgl, LGL_FALSE, LGL_NA, LGL_TRUE, NA_INT};

#[derive(Clone, Debug, PartialEq)]
pub enum Tok {
    DblLit(f64),
    IntLit(i32),
    LglLit(Lgl),
    StrLit(String),
    NaChr,
    Ident(String),
    // Keywords
    Fn,
    If,
    Else,
    For,
    In,
    While,
    Return,
    Break,
    Next,
    // Operators
    Assign, // <- or =
    Arrow,  // ->
    Pipe,   // |> or %>%
    InOp,   // %in%
    Mod,    // %%
    IntDiv, // %/%
    Colon,  // :
    Plus,
    Minus,
    Star,
    Slash,
    Caret,
    Eq,
    Ne,
    Lt,
    Le,
    Gt,
    Ge,
    Bang,
    And,
    And2,
    Or,
    Or2,
    Dollar,
    Placeholder, // _
    // Delimiters
    LParen,
    RParen,
    LBrack,
    RBrack,
    LBrack2,
    RBrack2,
    LBrace,
    RBrace,
    Comma,
    Term, // ; or significant \n
    Eof,
}

impl Tok {
    /// In R, a newline after a binary operator, comma, or open delimiter is ignored
    fn suppresses_newline(&self) -> bool {
        matches!(
            self,
            Tok::Assign
                | Tok::Arrow
                | Tok::Pipe
                | Tok::InOp
                | Tok::Mod
                | Tok::IntDiv
                | Tok::Colon
                | Tok::Plus
                | Tok::Minus
                | Tok::Star
                | Tok::Slash
                | Tok::Caret
                | Tok::Eq
                | Tok::Ne
                | Tok::Lt
                | Tok::Le
                | Tok::Gt
                | Tok::Ge
                | Tok::Bang
                | Tok::And
                | Tok::And2
                | Tok::Or
                | Tok::Or2
                | Tok::Dollar
                | Tok::LParen
                | Tok::LBrack
                | Tok::LBrack2
                | Tok::LBrace
                | Tok::Comma
                | Tok::Else
        )
    }
}

/// An open delimiter on the bracket stack. `Header` is a `(` opened right after
/// `if`, `while`, `for`, or `function`.
#[derive(Clone, Copy, PartialEq)]
enum Open {
    Paren,
    Header,
    Brack,
    Brack2,
}

pub fn tokenize(src: &str) -> Result<Vec<Tok>, String> {
    let bytes = src.as_bytes();
    let mut i = 0;
    let mut tokens: Vec<Tok> = Vec::new();
    // Newlines are suppressed while any `(`, `[`, or `[[` is open
    let mut stack: Vec<Open> = Vec::new();
    // Index of the `)` that closes an if/while/for/function header
    let mut header_close: Option<usize> = None;

    while i < bytes.len() {
        let c = bytes[i] as char;
        if c == ' ' || c == '\t' || c == '\r' {
            i += 1;
            continue;
        }
        if c == '#' {
            while i < bytes.len() && bytes[i] != b'\n' {
                i += 1;
            }
            continue;
        }
        if c == '\n' || c == ';' {
            i += 1;
            let after_header =
                header_close.is_some() && header_close == tokens.len().checked_sub(1);
            let suppress = c == '\n'
                && (!stack.is_empty()
                    || after_header
                    || tokens
                        .last()
                        .is_none_or(|t| t.suppresses_newline() || *t == Tok::Term));
            if !suppress {
                tokens.push(Tok::Term);
            }
            continue;
        }
        // Numbers
        if c.is_ascii_digit()
            || (c == '.' && i + 1 < bytes.len() && (bytes[i + 1] as char).is_ascii_digit())
        {
            let start = i;
            let mut is_float = c == '.';
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_digit() {
                    i += 1;
                } else if ch == '.' && !is_float {
                    is_float = true;
                    i += 1;
                } else if (ch == 'e' || ch == 'E') && i + 1 < bytes.len() {
                    is_float = true;
                    i += 1;
                    if bytes[i] == b'+' || bytes[i] == b'-' {
                        i += 1;
                    }
                } else {
                    break;
                }
            }
            let num_str = &src[start..i];
            if i < bytes.len() && bytes[i] == b'L' && !is_float {
                i += 1;
                let val: i32 = num_str.parse().map_err(|e| format!("Invalid int: {}", e))?;
                tokens.push(Tok::IntLit(val));
            } else {
                if is_float && i < bytes.len() && bytes[i] == b'L' {
                    return Err("ParseError: float literal cannot have L suffix".into());
                }
                let val: f64 = num_str
                    .parse()
                    .map_err(|e| format!("Invalid float: {}", e))?;
                tokens.push(Tok::DblLit(val));
            }
            continue;
        }
        // Strings
        if c == '"' || c == '\'' {
            let quote = bytes[i];
            i += 1;
            let mut value = Vec::new();
            while i < bytes.len() && bytes[i] != quote {
                if bytes[i] == b'\\' && i + 1 < bytes.len() {
                    let escaped = bytes[i + 1];
                    if escaped == quote || escaped == b'"' || escaped == b'\'' || escaped == b'\\' {
                        value.push(escaped);
                        i += 2;
                        continue;
                    }
                }
                value.push(bytes[i]);
                i += 1;
            }
            if i >= bytes.len() {
                return Err("Unterminated string literal".into());
            }
            tokens.push(Tok::StrLit(
                String::from_utf8(value).map_err(|e| e.to_string())?,
            ));
            i += 1;
            continue;
        }
        // Special %...% operators
        if c == '%' {
            let start = i;
            i += 1;
            while i < bytes.len() && bytes[i] != b'%' {
                i += 1;
            }
            if i >= bytes.len() {
                return Err("Unterminated % operator".into());
            }
            i += 1;
            match &src[start..i] {
                "%%" => tokens.push(Tok::Mod),
                "%/%" => tokens.push(Tok::IntDiv),
                "%in%" => tokens.push(Tok::InOp),
                "%>%" => tokens.push(Tok::Pipe),
                other => return Err(format!("Unsupported infix operator: {}", other)),
            }
            continue;
        }
        // Identifiers & Keywords (R allows '.' in identifiers)
        if c.is_ascii_alphabetic() || c == '_' || c == '.' {
            let start = i;
            i += 1;
            while i < bytes.len() {
                let ch = bytes[i] as char;
                if ch.is_ascii_alphanumeric() || ch == '_' || ch == '.' {
                    i += 1;
                } else {
                    break;
                }
            }
            let ident = &src[start..i];
            let tok = match ident {
                "function" | "fn" => Tok::Fn,
                "if" => Tok::If,
                "else" => Tok::Else,
                "for" => Tok::For,
                "in" => Tok::In,
                "while" => Tok::While,
                "return" => Tok::Return,
                "break" => Tok::Break,
                "next" => Tok::Next,
                "TRUE" => Tok::LglLit(LGL_TRUE),
                "FALSE" => Tok::LglLit(LGL_FALSE),
                "NA" => Tok::LglLit(LGL_NA),
                "NA_real_" => Tok::DblLit(na_real()),
                "NaN" => Tok::DblLit(f64::NAN),
                "Inf" => Tok::DblLit(f64::INFINITY),
                "NA_integer_" => Tok::IntLit(NA_INT),
                "NA_character_" => Tok::NaChr,
                "_" => Tok::Placeholder,
                _ => Tok::Ident(ident.to_string()),
            };
            tokens.push(tok);
            continue;
        }
        // Multi-char & single-char punctuation
        let rem = &src[i..];
        if rem.starts_with("<-") {
            tokens.push(Tok::Assign);
            i += 2;
        } else if rem.starts_with("->") {
            tokens.push(Tok::Arrow);
            i += 2;
        } else if rem.starts_with("|>") {
            tokens.push(Tok::Pipe);
            i += 2;
        } else if rem.starts_with("==") {
            tokens.push(Tok::Eq);
            i += 2;
        } else if rem.starts_with("!=") {
            tokens.push(Tok::Ne);
            i += 2;
        } else if rem.starts_with("<=") {
            tokens.push(Tok::Le);
            i += 2;
        } else if rem.starts_with(">=") {
            tokens.push(Tok::Ge);
            i += 2;
        } else if rem.starts_with("&&") {
            tokens.push(Tok::And2);
            i += 2;
        } else if rem.starts_with("||") {
            tokens.push(Tok::Or2);
            i += 2;
        } else if rem.starts_with("[[") {
            tokens.push(Tok::LBrack2);
            stack.push(Open::Brack2);
            i += 2;
        } else if rem.starts_with("]]") && stack.last() == Some(&Open::Brack2) {
            // `]]` closes `[[` only when `[[` is the innermost open bracket
            tokens.push(Tok::RBrack2);
            stack.pop();
            i += 2;
        } else {
            let ch = rem.chars().next().unwrap();
            i += 1;
            match c {
                '=' => tokens.push(Tok::Assign),
                ':' => tokens.push(Tok::Colon),
                '+' => tokens.push(Tok::Plus),
                '-' => tokens.push(Tok::Minus),
                '*' => tokens.push(Tok::Star),
                '/' => tokens.push(Tok::Slash),
                '^' => tokens.push(Tok::Caret),
                '<' => tokens.push(Tok::Lt),
                '>' => tokens.push(Tok::Gt),
                '!' => tokens.push(Tok::Bang),
                '&' => tokens.push(Tok::And),
                '|' => tokens.push(Tok::Or),
                '$' => tokens.push(Tok::Dollar),
                ',' => tokens.push(Tok::Comma),
                '(' => {
                    let header = matches!(
                        tokens.last(),
                        Some(Tok::If | Tok::While | Tok::For | Tok::Fn)
                    );
                    stack.push(if header { Open::Header } else { Open::Paren });
                    tokens.push(Tok::LParen);
                }
                ')' => {
                    tokens.push(Tok::RParen);
                    match stack.pop() {
                        Some(Open::Header) => header_close = Some(tokens.len() - 1),
                        Some(Open::Paren) => {}
                        _ => return Err("ParseError: unmatched ')'".into()),
                    }
                }
                '[' => {
                    tokens.push(Tok::LBrack);
                    stack.push(Open::Brack);
                }
                ']' => {
                    tokens.push(Tok::RBrack);
                    if stack.pop() != Some(Open::Brack) {
                        return Err("ParseError: unmatched ']'".into());
                    }
                }
                '\\' if src[i..].trim_start_matches([' ', '\t']).starts_with('(') => {
                    tokens.push(Tok::Fn)
                }
                '{' => tokens.push(Tok::LBrace),
                '}' => tokens.push(Tok::RBrace),
                _ => return Err(format!("Unexpected character: '{}'", ch)),
            }
        }
    }
    if let Some(open) = stack.last() {
        let delim = match open {
            Open::Paren | Open::Header => "(",
            Open::Brack => "[",
            Open::Brack2 => "[[",
        };
        return Err(format!("ParseError: unclosed '{}'", delim));
    }
    tokens.push(Tok::Eof);
    Ok(tokens)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn toks(src: &str) -> Vec<Tok> {
        tokenize(src).unwrap()
    }

    #[test]
    fn newline_after_binary_op_is_suppressed() {
        assert!(!toks("1 +\n2").contains(&Tok::Term));
    }

    #[test]
    fn newline_inside_parens_is_suppressed() {
        assert!(!toks("(1\n+2)").contains(&Tok::Term));
    }

    #[test]
    fn statement_newline_emits_one_term() {
        assert_eq!(
            toks("x <- 1\ny")
                .iter()
                .filter(|t| **t == Tok::Term)
                .count(),
            1
        );
    }

    #[test]
    fn keywords_and_aliases() {
        assert_eq!(toks("fn function"), vec![Tok::Fn, Tok::Fn, Tok::Eof]);
        assert!(matches!(toks("NaN")[0], Tok::DblLit(x) if x.is_nan()));
        assert!(matches!(toks("NA_real_")[0], Tok::DblLit(x) if x.is_nan()));
        assert_eq!(toks("Inf"), vec![Tok::DblLit(f64::INFINITY), Tok::Eof]);
        assert_eq!(toks("NA_integer_"), vec![Tok::IntLit(NA_INT), Tok::Eof]);
    }

    #[test]
    fn nested_index_brackets() {
        assert_eq!(
            toks("a[b[1]]"),
            vec![
                Tok::Ident("a".into()),
                Tok::LBrack,
                Tok::Ident("b".into()),
                Tok::LBrack,
                Tok::DblLit(1.0),
                Tok::RBrack,
                Tok::RBrack,
                Tok::Eof
            ]
        );
        assert_eq!(
            toks("x[[y[1]]]"),
            vec![
                Tok::Ident("x".into()),
                Tok::LBrack2,
                Tok::Ident("y".into()),
                Tok::LBrack,
                Tok::DblLit(1.0),
                Tok::RBrack,
                Tok::RBrack2,
                Tok::Eof
            ]
        );
    }

    #[test]
    fn newline_after_if_header() {
        assert!(!toks("if (x)\n1").contains(&Tok::Term));
        assert!(toks("f(x)\n1").contains(&Tok::Term));
    }

    #[test]
    fn backslash_lambda_is_fn() {
        assert_eq!(toks("\\(x) x + 1"), toks("fn(x) x + 1"));
        assert_eq!(toks("\\ (x) x"), toks("fn (x) x"));
    }

    #[test]
    fn bare_backslash_errors() {
        for s in ["\\", "\\x", "\\\n(x)"] {
            assert_eq!(tokenize(s).unwrap_err(), "Unexpected character: '\\'");
        }
    }

    #[test]
    fn unexpected_char_message() {
        assert_eq!(tokenize("1 + é").unwrap_err(), "Unexpected character: 'é'");
    }

    #[test]
    fn float_with_l_suffix_is_parse_error() {
        assert!(tokenize("1.5L").unwrap_err().starts_with("ParseError:"));
        assert!(tokenize("1e3L").unwrap_err().starts_with("ParseError:"));
    }

    #[test]
    fn mismatched_and_unclosed_delimiters_are_parse_errors() {
        for src in ["f(1]", "x[1)", "(1", "x[[1]", ")"] {
            let err = tokenize(src).unwrap_err();
            assert!(err.starts_with("ParseError:"), "{src}: {err}");
        }
    }

    #[test]
    fn nested_matching_delimiters_still_lex() {
        assert!(tokenize("x[[a[1]]]").is_ok());
        assert!(tokenize("f(g(1))[2]").is_ok());
    }

    #[test]
    fn int_with_l_suffix_still_lexes() {
        assert_eq!(toks("1L"), vec![Tok::IntLit(1), Tok::Eof]);
        assert_eq!(toks("2147483647L"), vec![Tok::IntLit(2147483647), Tok::Eof]);
    }
}

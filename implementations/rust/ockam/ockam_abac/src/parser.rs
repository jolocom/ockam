use crate::error::ParseError;
use crate::expr::Expr;
use core::str;
use ockam_core::compat::boxed::Box;
use ockam_core::compat::format;
use ockam_core::compat::string::ToString;
use ockam_core::compat::vec::Vec;
use once_cell::race::OnceBox;
use regex::Regex;
use wast::lexer::{FloatVal, Lexer, Token};

/// Allowed identifier patterns.
fn ident_pattern() -> &'static Regex {
    static INSTANCE: OnceBox<Regex> = OnceBox::new();
    INSTANCE.get_or_init(|| {
        Box::new(Regex::new("^[a-zA-Z!$%&*/:<=>?~_^][a-zA-Z0-9!$%&*/:<=>?~_^.+-@]*$").unwrap())
    })
}

#[rustfmt::skip]
pub fn parse(s: &str) -> Result<Option<Expr>, ParseError> {
    // Control stack element
    enum E {
        Ex(Expr), // expression
        Nx,       // parse next expression
        La,       // start of list
        Le,       // end of list
        Sa,       // start of sequence
        Se,       // end of sequence
    }

    let mut lx = Lexer::new(s);
    let mut xs = Vec::new(); // expressions
    let mut st = Vec::new(); // stack
    st.push(E::Nx);

    while let Some(e) = st.pop() {
        match e {
            E::Nx => match next(&mut lx)? {
                None => continue,
                Some(Token::Whitespace(_) | Token::LineComment(_) | Token::BlockComment(_)) =>
                    st.push(E::Nx),
                Some(Token::Integer(i)) => {
                    let (s, r) = i.val();
                    let x = i64::from_str_radix(s, r)?;
                    st.push(E::Ex(Expr::Int(x)));
                    st.push(E::Nx)
                }
                Some(Token::Float(v)) => {
                    match v.val() {
                        FloatVal::Inf { negative: true } =>
                            st.push(E::Ex(Expr::Float(f64::NEG_INFINITY))),
                        FloatVal::Inf { negative: false } =>
                            st.push(E::Ex(Expr::Float(f64::INFINITY))),
                        FloatVal::Nan { .. } =>
                            st.push(E::Ex(Expr::Float(f64::NAN))),
                        FloatVal::Val { .. } => {
                            let x: f64 = v.src().parse()?;
                            st.push(E::Ex(Expr::Float(x)))
                        }
                    }
                    st.push(E::Nx)
                }
                Some(Token::String(s)) => {
                    st.push(E::Ex(Expr::Str(str::from_utf8(s.val())?.to_string())));
                    st.push(E::Nx)
                }
                Some(Token::LParen(_)) => {
                    st.push(E::La);
                    st.push(E::Nx)
                }
                Some(Token::RParen(_)) => {
                    st.push(E::Le)
                }
                Some(Token::Reserved("]")) => {
                    st.push(E::Se)
                }
                Some(Token::Reserved("[")) => {
                    st.push(E::Sa);
                    st.push(E::Nx)
                }
                Some(Token::Keyword("true")) => {
                    st.push(E::Ex(Expr::Bool(true)));
                    st.push(E::Nx)
                }
                Some(Token::Keyword("false")) => {
                    st.push(E::Ex(Expr::Bool(false)));
                    st.push(E::Nx)
                }
                Some(Token::Id(v)) => {
                    st.push(E::Ex(Expr::Ident(v.to_string())));
                    st.push(E::Nx)
                }
                Some(Token::Keyword(v) | Token::Reserved(v)) => {
                    if ident_pattern().is_match(v) {
                        st.push(E::Ex(Expr::Ident(v.to_string())));
                        st.push(E::Nx)
                    } else {
                        return Err(ParseError::message(format!("invalid token '{v}'")))
                    }
                }
            }
            E::Ex(x) => xs.push(x),
            E::Le => {
                let mut v = Vec::new();
                while let Some(x) = st.pop() {
                    match x {
                        E::La    => break,
                        E::Ex(x) => v.push(x),
                        E::Le    => return Err(ParseError::message("')' without matching '('")),
                        E::Sa    => return Err(ParseError::message("'[' without matching ']'")),
                        E::Se    => return Err(ParseError::message("']' without matching '['")),
                        E::Nx    => unreachable!("consecutive E::Nx are impossible")
                    }
                }
                v.reverse();
                st.push(E::Ex(Expr::List(v)));
                st.push(E::Nx)
            }
            E::Se => {
                let mut v = Vec::new();
                while let Some(x) = st.pop() {
                    match x {
                        E::Sa    => break,
                        E::Ex(x) => v.push(x),
                        E::Le    => return Err(ParseError::message("')' without matching '('")),
                        E::La    => return Err(ParseError::message("'(' without matching ')'")),
                        E::Se    => return Err(ParseError::message("']' without matching '['")),
                        E::Nx    => unreachable!("consecutive E::Nx are impossible")
                    }
                }
                v.reverse();
                st.push(E::Ex(Expr::Seq(v)));
                st.push(E::Nx)
            }
            E::La => return Err(ParseError::message("unclosed '('")),
            E::Sa => return Err(ParseError::message("unclosed '['"))
        }
    }

    match xs.len() {
        0 => Ok(None),
        1 => Ok(Some(xs.remove(0))),
        _ => {
            xs.reverse();
            Ok(Some(Expr::List(xs)))
        }
    }
}

fn next<'a>(lx: &mut Lexer<'a>) -> Result<Option<Token<'a>>, ParseError> {
    while let Some(tk) = lx.parse()? {
        match tk {
            Token::Whitespace(_) | Token::LineComment(_) | Token::BlockComment(_) => continue,
            other => return Ok(Some(other)),
        }
    }
    Ok(None)
}

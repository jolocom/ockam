use crate::env::Env;
use crate::error::EvalError;
use crate::expr::{unit, Expr};
use ockam_core::compat::string::ToString;

#[rustfmt::skip]
pub fn eval(expr: &Expr, env: &Env) -> Result<Expr, EvalError> {

    #[derive(Debug)]
    enum Op<'a> {
        Eval(&'a Expr),
        And(usize),
        Or(usize),
        Not,
        If,
        Eq(usize),
        Gt(usize),
        Lt(usize),
        Member,
        Seq(usize),
    }

    let mut ctrl = Vec::new();
    let mut args = Vec::new();
    ctrl.push(Op::Eval(expr));

    while let Some(x) = ctrl.pop() {
        match x {
            Op::Eval(Expr::Ident(id)) => ctrl.push(Op::Eval(env.get(id)?)),
            Op::Eval(Expr::List(es))  => match &es[..] {
                []                    => args.push(unit()),
                [Expr::Ident(id), ..] => {
                    match id.as_str() {
                        "and" => ctrl.push(Op::And(es.len() - 1)),
                        "or"  => ctrl.push(Op::Or(es.len() - 1)),
                        "not" => ctrl.push(Op::Not),
                        "if"  => ctrl.push(Op::If),
                        "<"   => ctrl.push(Op::Lt(es.len() - 1)),
                        ">"   => ctrl.push(Op::Gt(es.len() - 1)),
                        "="   => ctrl.push(Op::Eq(es.len() - 1)),
                        "!="  => {
                            ctrl.push(Op::Not);
                            ctrl.push(Op::Eq(es.len() - 1))
                        }
                        "member?" => ctrl.push(Op::Member),
                        "exists?" => {
                            let mut b = true;
                            for e in &es[1..] {
                                match e {
                                    Expr::Ident(id) => if !env.contains(id) {
                                        b = false;
                                        break
                                    }
                                    other => {
                                        let msg = "'exists?' expects identifiers";
                                        return Err(EvalError::InvalidType(other.clone(), msg))
                                    }
                                }
                            }
                            args.push(Expr::Bool(b));
                            continue
                        }
                        _  => return Err(EvalError::Unknown(id.to_string()))
                    }
                    for e in es[1..].iter().rev() {
                        ctrl.push(Op::Eval(e))
                    }
                }
                [other, ..] => {
                    let msg = "expected (op ...)";
                    return Err(EvalError::InvalidType(other.clone(), msg))
                }
            }
            Op::Eval(Expr::Seq(es)) => {
                ctrl.push(Op::Seq(es.len()));
                for e in es.iter().rev() {
                    ctrl.push(Op::Eval(e))
                }
            }
            Op::Eval(expr) => args.push(expr.clone()),
            Op::And(n) => {
                let mut b = true;
                for e in args.drain(args.len() - n ..) {
                    match e {
                        Expr::Bool(true)  => continue,
                        Expr::Bool(false) => { b = false; break }
                        other             => return Err(EvalError::InvalidType(other, "'and' expected bool"))
                    }
                }
                args.push(Expr::Bool(b))
            }
            Op::Or(n) => {
                let mut b = false ;
                for e in args.drain(args.len() - n ..) {
                    match e {
                        Expr::Bool(true)  => { b = true; break }
                        Expr::Bool(false) => continue,
                        other             => return Err(EvalError::InvalidType(other, "'or' expected bool"))
                    }
                }
                args.push(Expr::Bool(b))
            }
            Op::Not => {
                match args.pop() {
                    Some(Expr::Bool(b)) => args.push(Expr::Bool(!b)),
                    Some(other)         => return Err(EvalError::InvalidType(other, "'not' expected bool")),
                    None                => return Err(EvalError::malformed("'not' requires exactly one argument"))
                }
            }
            Op::If => {
                if args.len() < 3 {
                    return Err(EvalError::malformed("'if' requires three arguments"))
                }
                let f = args.pop().expect("args.len() >= 3");
                let t = args.pop().expect("args.len() >= 2");
                let x = args.pop().expect("args.len() >= 1");
                match x {
                    Expr::Bool(true)  => args.push(t),
                    Expr::Bool(false) => args.push(f),
                    other             => return Err(EvalError::InvalidType(other, "'if' expected bool"))
                }
            }
            Op::Eq(n) => {
                if args.len() < 2 {
                    return Err(EvalError::malformed("'=' requires at least two arguments"))
                }
                let mut b = true;
                let x = &args[args.len() - n];
                for y in &args[args.len() - (n - 1) ..] {
                    if x != y {
                        b = false;
                        break
                    }
                }
                args.truncate(args.len() - n);
                args.push(Expr::Bool(b))
            }
            Op::Lt(n) => {
                if args.len() < 2 {
                    return Err(EvalError::malformed("'<' requires at least two arguments"))
                }
                let mut b = true;
                let mut x = &args[args.len() - n];
                for y in &args[args.len() - (n - 1) ..] {
                    if x >= y {
                        b = false;
                        break
                    }
                    x = y
                }
                args.truncate(args.len() - n);
                args.push(Expr::Bool(b))
            }
            Op::Gt(n) => {
                if args.len() < 2 {
                    return Err(EvalError::malformed("'>' requires at least two arguments"))
                }
                let mut b = true;
                let mut x = &args[args.len() - n];
                for y in &args[args.len() - (n - 1) ..] {
                    if x <= y {
                        b = false;
                        break
                    }
                    x = y
                }
                args.truncate(args.len() - n);
                args.push(Expr::Bool(b))
            }
            Op::Member => {
                if args.len() < 2 {
                    return Err(EvalError::malformed("'member?' requires two arguments"))
                }
                let s = args.pop().expect("args.len() >= 2");
                let x = args.pop().expect("args.len() >= 1");
                match s {
                    Expr::Seq(xs) => args.push(Expr::Bool(xs.contains(&x))),
                    other => {
                        let msg = "'member?' expects sequence as second argument";
                        return Err(EvalError::InvalidType(other, msg))
                    }
                }
            }
            Op::Seq(n) => {
                let s = args.split_off(args.len() - n);
                args.push(Expr::Seq(s))
            }
        }
    }

    debug_assert_eq!(1, args.len());
    Ok(args.pop().unwrap())
}

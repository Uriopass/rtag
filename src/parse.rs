use crate::{qry::Expr, TagName};
use std::str::Chars;

#[derive(Copy, Clone, Eq, PartialEq, Debug)]
#[repr(u8)]
enum Oper {
    Intersect,
    Union,
    Neg,
}

fn precedence(op: Oper) -> u8 {
    match op {
        Oper::Neg => 3,
        Oper::Intersect => 2,
        Oper::Union => 1,
    }
}

#[derive(Eq, PartialEq, Debug)]
enum Token {
    Ident(String),
    ParLeft,
    ParRight,
    Op(Oper),
}

fn lexer(v: &str) -> Vec<Token> {
    use Oper::*;
    use Token::*;
    fn parse_ident(first: char, c: &mut Chars) -> (Option<char>, String) {
        let mut chars = vec![first];
        for v in c {
            if v.is_alphanumeric() {
                chars.push(v);
            } else {
                return (Some(v), chars.into_iter().collect());
            }
        }
        (None, chars.into_iter().collect())
    }

    let mut tokens = vec![];
    let mut c = v.chars();
    let mut nexttok = c.next();
    while nexttok.is_some() {
        let tok = match nexttok {
            Some('(') => ParLeft,
            Some(')') => ParRight,
            Some('-') => Op(Neg),
            Some('!') => Op(Neg),
            Some('~') => Op(Neg),
            Some('&') => Op(Intersect),
            Some('|') => Op(Union),
            Some('+') => Op(Union),
            Some(t) if t.is_alphanumeric() => {
                let (nextok, ident) = parse_ident(t, &mut c);
                nexttok = nextok;
                tokens.push(Ident(ident));
                continue;
            }
            Some(_) => {
                nexttok = c.next();
                continue;
            }
            None => break,
        };
        nexttok = c.next();
        tokens.push(tok);
    }
    tokens
}

fn shunting_yard(toks: Vec<Token>) -> Vec<Token> {
    use Token::*;
    let mut opstack: Vec<Token> = vec![];
    let mut out = Vec::with_capacity(toks.len());
    let mut lastwasident = false;
    for t in toks {
        match t {
            Op(Oper::Union) | Op(Oper::Intersect) | ParRight => {}
            Ident(_) | ParLeft | Op(Oper::Neg) => {
                if lastwasident {
                    opstack.push(Token::Op(Oper::Intersect));
                }
            }
        }

        lastwasident = false;
        match t {
            Ident(_) => {
                out.push(t);
                lastwasident = true;
            }
            ParLeft => opstack.push(t),
            ParRight => {
                loop {
                    match opstack.pop() {
                        None => break,
                        Some(ParLeft) => break,
                        Some(v) => out.push(v),
                    }
                }
                lastwasident = true;
            }
            Op(op) => {
                while opstack
                    .last()
                    .and_then(|x| {
                        if let Op(op) = *x {
                            Some(precedence(op))
                        } else {
                            None
                        }
                    })
                    .unwrap_or(u8::MIN)
                    >= precedence(op)
                {
                    out.push(opstack.pop().unwrap());
                }
                opstack.push(t);
            }
        }
    }
    out.extend(opstack.into_iter().rev().filter(|x| x != &ParLeft));
    out
}

fn rpn_to_expr(tokens: Vec<Token>) -> Option<Expr> {
    let mut stack = vec![];
    for t in tokens {
        match t {
            Token::Ident(x) => stack.push(Expr::Tag(TagName(x))),
            Token::Op(Oper::Neg) => {
                if let Some(x) = stack.pop() {
                    stack.push(Expr::Not(Box::new(x)));
                } else {
                    panic!("expected expr to negate");
                }
            }
            Token::Op(Oper::Union) => {
                if let (Some(a), Some(b)) = (stack.pop(), stack.pop()) {
                    stack.push(Expr::Or(Box::new(a), Box::new(b)));
                } else {
                    panic!("expected 2 expr to unionize");
                }
            }
            Token::Op(Oper::Intersect) => {
                if let (Some(a), Some(b)) = (stack.pop(), stack.pop()) {
                    stack.push(Expr::And(Box::new(a), Box::new(b)));
                } else {
                    panic!("expected 2 expr to intersect");
                }
            }
            Token::ParLeft | Token::ParRight => {
                unreachable!("parentheses should'vee been removed by now")
            }
        }
    }
    if stack.len() > 1 {
        panic!("multiple expr left in stack");
    }
    stack.pop()
}

pub fn parse_query(v: &str) -> Option<Expr> {
    let mut lexems = lexer(v);
    println!("lexems:  {:?}", lexems);
    lexems = shunting_yard(lexems);
    println!("shunted: {:?}", lexems);
    rpn_to_expr(lexems)
}

use crate::parse::Token::ParLeft;
use crate::Expr;
use std::str::Chars;

enum Oper {
    Intersect,
    Union,
    Neg,
}

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
            match v {
                t @ ('a'..='z' | 'A'..='Z' | '0'..='9') => chars.push(t),
                _ => return (Some(v), chars.into_iter().collect()),
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
            Some('&') => Op(Intersect),
            Some('|') => Op(Union),
            Some(t @ ('a'..='z' | 'A'..='Z' | '0'..='9')) => {
                let (nextok, ident) = parse_ident(t, &mut c);
                nexttok = nextok;
                tokens.push(Ident(ident));
                continue;
            }
            Some(_) => {
                continue;
            }
            None => break,
        };
        nexttok = c.next();
        tokens.push(tok);
    }
    tokens
}

fn shunting_yard(x: Vec<Token>) -> Vec<Token> {
    todo!()
}

fn rpn_to_expr(x: Vec<Token>) -> Expr {
    todo!()
}

fn parse_query(v: &str) -> Expr {
    let mut lexems = lexer(v);
    lexems = shunting_yard(lexems);
    rpn_to_expr(lexems)
}

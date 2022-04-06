mod parse;

use std::collections::BTreeMap;
use std::path::PathBuf;

#[derive(Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
struct ID(u32);

struct Value(PathBuf);

#[derive(Clone)]
struct TagName(String);

struct TagList {
    name: String,
    kind: TagListKind,
}

enum TagListKind {
    List(Vec<(ID, Value)>),
    Map(BTreeMap<ID, Value>),
}

#[derive(Clone)]
enum Expr {
    Tag(TagName),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

/// Conjunctive normal form
/// List of ors of ands of tags with boolean = true or false
struct CNF(Vec<Vec<(TagName, bool)>>);

fn to_cnf(expr: Expr) -> CNF {
    use Expr::*;
    fn lower_negs(expr: Expr) -> Expr {
        match expr {
            Tag(_) => expr,
            Not(v) => match *v {
                Tag(ref l) => Not(v),
                Not(x) => *x,
                And(l, r) => Or(Box::new(lower_negs(Not(l))), Box::new(lower_negs(Not(r)))),
                Or(l, r) => And(Box::new(lower_negs(Not(l))), Box::new(lower_negs(Not(r)))),
            },
            And(l, r) => And(Box::new(lower_negs(*l)), Box::new(lower_negs(*r))),
            Or(l, r) => Or(Box::new(lower_negs(*l)), Box::new(lower_negs(*r))),
        }
    }

    fn to_cnf_inner(expr: Expr) -> Expr {
        match expr {
            e @ Tag(_) => e,
            Not(v) => match *v {
                Tag(_) => Not(v),
                _ => unreachable!("NOTs should be lowered"),
            },
            Or(l, r) => Or(Box::new(to_cnf_inner(*l)), Box::new(to_cnf_inner(*r))),
            And(l, r) => match (*l, *r) {
                (Or(l1, l2), v) | (v, Or(l1, l2)) => Or(
                    Box::new(to_cnf_inner(And(l1, Box::new(v.clone())))),
                    Box::new(to_cnf_inner(And(l2, Box::new(v)))),
                ),
                (a, b) => And(Box::new(a), Box::new(b)),
            },
        }
    }

    fn cnf_flatten(expr: Expr) -> CNF {
        fn collect_ands(expr: Expr, v: &mut Vec<(TagName, bool)>) {
            match expr {
                Tag(x) => v.push((x, true)),
                Not(t) => match *t {
                    Tag(x) => v.push((x, false)),
                    _ => unreachable!("NOTs should be lowered"),
                },
                And(l, r) => {
                    collect_ands(*l, v);
                    collect_ands(*r, v);
                }
                Or(_, _) => unreachable!("CNF means no ORs under ANDs"),
            }
        }

        fn collect_ors(expr: Expr, cnf: &mut CNF) {
            match expr {
                Or(l, r) => {
                    collect_ors(*l, cnf);
                    collect_ors(*r, cnf);
                }
                e => {
                    let mut v = vec![];
                    collect_ands(e, &mut v);
                    cnf.0.push(v);
                }
            }
        }

        let mut emptycnf = CNF(vec![]);
        collect_ors(expr, &mut emptycnf);
        emptycnf
    }

    let lowered = lower_negs(expr);
    let cnf_expr = to_cnf_inner(lowered);
    cnf_flatten(cnf_expr)
}

fn execute(cnf: &CNF, limit: usize) -> impl Iterator<Item = ID> {
    todo!();
    let v: Option<ID> = None;
    v.into_iter()
}

fn main() {
    let qry_expr = parse_query("a & b");
    let qry_cnf = to_cnf(qry_expr);
    for id in execute(&qry_cnf, 100) {
        println!("{}", id.0);
    }
}

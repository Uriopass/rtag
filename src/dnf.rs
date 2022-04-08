use crate::{qry::Expr, qry::DNF, TagName};

pub fn to_dnf(expr: Expr) -> DNF {
    use Expr::*;
    fn lower_negs(expr: Expr) -> Expr {
        match expr {
            Tag(_) => expr,
            Not(v) => match *v {
                Tag(_) => Not(v),
                Not(x) => *x,
                And(l, r) => Or(Box::new(lower_negs(Not(l))), Box::new(lower_negs(Not(r)))),
                Or(l, r) => And(Box::new(lower_negs(Not(l))), Box::new(lower_negs(Not(r)))),
            },
            And(l, r) => And(Box::new(lower_negs(*l)), Box::new(lower_negs(*r))),
            Or(l, r) => Or(Box::new(lower_negs(*l)), Box::new(lower_negs(*r))),
        }
    }

    fn to_dnf_inner(expr: Expr) -> Expr {
        match expr {
            e @ Tag(_) => e,
            Not(v) => match *v {
                Tag(_) => Not(v),
                _ => unreachable!("NOTs should be lowered"),
            },
            Or(l, r) => Or(Box::new(to_dnf_inner(*l)), Box::new(to_dnf_inner(*r))),
            And(l, r) => match (*l, *r) {
                (Or(l1, l2), v) | (v, Or(l1, l2)) => Or(
                    Box::new(to_dnf_inner(And(l1, Box::new(v.clone())))),
                    Box::new(to_dnf_inner(And(l2, Box::new(v)))),
                ),
                (a, b) => And(Box::new(a), Box::new(b)),
            },
        }
    }

    fn dnf_flatten(expr: Expr) -> DNF {
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

        fn collect_ors(expr: Expr, cnf: &mut DNF) {
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

        let mut emptycnf = DNF(vec![]);
        collect_ors(expr, &mut emptycnf);
        emptycnf
    }

    let lowered = lower_negs(expr);
    if std::env::var("DEBUG").is_ok() {
        eprintln!("lowered: {:?}", lowered);
    }
    let dnf_expr = to_dnf_inner(lowered);
    if std::env::var("DEBUG").is_ok() {
        eprintln!("dnf_expr: {:?}", dnf_expr);
    }
    dnf_flatten(dnf_expr)
}

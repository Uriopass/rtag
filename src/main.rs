use std::path::PathBuf;

mod cnf;
mod parse;
mod qry;

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct ID(u32);

pub struct Value(PathBuf);

#[derive(PartialOrd, Ord, Clone, Debug, Eq, PartialEq)]
pub struct TagName(String);

fn main() {
    // (-(a (b) c ) | (d & e))
    for id in qry::parse_and_execute("p p", 100) {
        println!("{}", id.0);
    }
}

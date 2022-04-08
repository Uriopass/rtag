use crate::write::{data, get_allmap, get_datamap, getroot, value_from_off};
use crate::{cnf, parse, TagName, Value, ID};
use memmap2::Mmap;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;

pub struct TagCtx {
    mapped_tags: BTreeMap<TagName, Mmap>,
    allmap: Mmap,
    datamap: Mmap,
}

#[derive(Clone, Debug)]
pub enum Expr {
    Tag(TagName),
    And(Box<Expr>, Box<Expr>),
    Or(Box<Expr>, Box<Expr>),
    Not(Box<Expr>),
}

/// Conjunctive normal form
/// List of ors of ands of tags with boolean = true or false
#[derive(Debug)]
pub struct CNF(pub Vec<Vec<(TagName, bool)>>);

fn iter_data<'a>(datamap: &'a [u8]) -> impl Iterator<Item = Value> + 'a {
    (0..datamap.len() / 256).map(move |off| value_from_off(datamap, off))
}

fn iter_tagmap<'a>(map: &'a [u8]) -> impl Iterator<Item = ID> + 'a {
    (0..map.len() / 4).map(move |off| ID(read_int(map, off)))
}

fn iter_tagmap_negall<'a>(allmap: &'a [u8], map: &'a [u8]) -> impl Iterator<Item = ID> + 'a {
    iter_tagmap(allmap).filter(|x| !find(map, *x))
}

pub fn find(map: &[u8], needle: ID) -> bool {
    let mut left = 0;
    let mut right = map.len() / 4;
    while left < right {
        let middle = (left + right) / 2;
        let v = read_int(map, middle);
        if v == needle.0 {
            return true;
        }
        if v > needle.0 {
            right = middle;
        } else {
            left = middle + 1;
        }
    }
    false
}

fn execute_and(ctx: &TagCtx, mut needs: Vec<(TagName, bool)>, limit: usize) -> Vec<ID> {
    needs.sort();
    needs.dedup();

    // detect p & !p
    for w in needs.windows(2) {
        if w[0].0 == w[1].0 && w[0].1 != w[1].1 {
            return vec![];
        }
    }

    let mut maps = Vec::with_capacity(needs.len());
    for (tag, val) in needs {
        let map = ctx.mapped_tags.get(&tag);
        match map {
            None => {
                if val {
                    return vec![];
                } else {
                    continue;
                }
            }
            Some(m) => maps.push((m, val)),
        }
    }

    if maps.is_empty() {
        return vec![];
    }

    let allsize = ctx.allmap.len() / 4;
    let (i, _) = maps
        .iter()
        .enumerate()
        .min_by_key(move |(_, (x, pos))| {
            let mut l = x.len() / 4;
            if !*pos {
                l += allsize;
            }
            l
        })
        .unwrap();

    let (minm, pos) = maps.swap_remove(i);

    let mut out = vec![];
    if pos {
        for val in iter_tagmap(minm) {
            if out.len() >= limit {
                return out;
            }

            if maps.iter().all(|&(m, pos)| find(m, val) == pos) {
                out.push(val);
            }
        }
    } else {
        for val in iter_tagmap_negall(&ctx.allmap, minm) {
            if out.len() >= limit {
                return out;
            }

            if maps.iter().all(|&(m, pos)| find(m, val) == pos) {
                out.push(val);
            }
        }
    }

    out
}

pub fn parse_and_execute(qry: &str, limit: usize) -> Vec<Value> {
    let qry_expr = parse::parse_query(qry);
    let qry_expr = match qry_expr {
        None => {
            let ctx = prepare_tags(&CNF(vec![]));
            return iter_data(&ctx.datamap).take(limit).collect();
        }
        Some(x) => x,
    };
    if std::env::var("DEBUG").is_ok() {
        eprintln!("expr:    {:?}", qry_expr);
    }
    let qry_cnf = cnf::to_cnf(qry_expr);
    if std::env::var("DEBUG").is_ok() {
        eprintln!("cnf: {:?}", qry_cnf);
    }
    execute(qry_cnf, limit).collect()
}

pub fn execute(cnf: CNF, limit: usize) -> impl Iterator<Item = Value> {
    let ctx = prepare_tags(&cnf);

    let mut ids: Vec<ID> = vec![];
    for andqry in cnf.0 {
        if ids.len() >= limit {
            break;
        }
        ids.extend(execute_and(&ctx, andqry, limit - ids.len()).into_iter());
    }
    ids.into_iter().flat_map(move |id| data(&ctx.datamap, id))
}

pub fn read_int(map: &[u8], off: usize) -> u32 {
    unsafe {
        u32::from_le_bytes([
            *map.get_unchecked(off * 4),
            *map.get_unchecked(off * 4 + 1),
            *map.get_unchecked(off * 4 + 2),
            *map.get_unchecked(off * 4 + 3),
        ])
    }
}

pub fn write_int(map: &mut [u8], off: usize, v: u32) {
    unsafe {
        let bytes = u32::to_le_bytes(v);
        *map.get_unchecked_mut(off * 4 + 0) = *bytes.get_unchecked(0);
        *map.get_unchecked_mut(off * 4 + 1) = *bytes.get_unchecked(1);
        *map.get_unchecked_mut(off * 4 + 2) = *bytes.get_unchecked(2);
        *map.get_unchecked_mut(off * 4 + 3) = *bytes.get_unchecked(3);
    }
}

fn prepare_tags(cnf: &CNF) -> TagCtx {
    let mut uniq_tags = BTreeSet::new();
    for or in &cnf.0 {
        for and in or {
            uniq_tags.insert(and.0.clone());
        }
    }

    let mut rootpath = getroot();
    let allmap = get_allmap(&mut rootpath);
    let (datamap, _) = get_datamap(&mut rootpath);

    let mut ctx = TagCtx {
        mapped_tags: BTreeMap::new(),
        allmap,
        datamap,
    };

    for tag in uniq_tags {
        rootpath.push(tag.0.clone());
        let file = File::options().read(true).open(&rootpath);
        let file = match file {
            Ok(x) => x,
            Err(_) => continue,
        };
        rootpath.pop();
        let mmap = unsafe { memmap2::Mmap::map(&file).expect("could not memmap file") };
        ctx.mapped_tags.insert(tag, mmap);
    }

    ctx
}

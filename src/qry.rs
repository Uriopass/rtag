use crate::{cnf, parse, TagName, ID};
use memmap2::Mmap;
use std::collections::{BTreeMap, BTreeSet};
use std::fs::File;
use std::io::{Seek, SeekFrom, Write};
use std::path::PathBuf;

pub struct TagCtx {
    mapped_tags: BTreeMap<TagName, Mmap>,
    allmap: Mmap,
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

fn iter_tagmap<'a>(map: &'a [u8]) -> impl Iterator<Item = ID> + 'a {
    (0..map.len() / 4).map(move |off| ID(read_int(map, off)))
}

fn iter_tagmap_negall<'a>(allmap: &'a [u8], map: &'a [u8]) -> impl Iterator<Item = ID> + 'a {
    iter_tagmap(allmap).filter(|x| !find(map, *x))
}

fn find(map: &[u8], needle: ID) -> bool {
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

    let maps: Option<Vec<_>> = needs
        .into_iter()
        .map(|(tag, val)| ctx.mapped_tags.get(&tag).zip(Some(val)))
        .collect();

    if maps.is_none() {
        return vec![];
    }
    let mut maps = maps.unwrap();

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

pub fn parse_and_execute(qry: &str, limit: usize) -> Vec<ID> {
    let qry_expr = parse::parse_query(qry);
    let qry_expr = match qry_expr {
        None => {
            let ctx = prepare_tags(&CNF(vec![]));
            return iter_tagmap(&ctx.allmap).take(limit).collect();
        }
        Some(x) => x,
    };
    eprintln!("expr:    {:?}", qry_expr);
    let qry_cnf = cnf::to_cnf(qry_expr);
    eprintln!("cnf: {:?}", qry_cnf);
    execute(qry_cnf, limit)
}

pub fn execute(cnf: CNF, limit: usize) -> Vec<ID> {
    let ctx = prepare_tags(&cnf);

    eprintln!("total tag size: {}", ctx.allmap.len() / 4);

    let mut ids = vec![];
    for andqry in cnf.0 {
        if ids.len() >= limit {
            return ids;
        }
        ids.extend(execute_and(&ctx, andqry, limit - ids.len()).into_iter());
    }
    ids
}

fn read_int(map: &[u8], off: usize) -> u32 {
    unsafe {
        u32::from_le_bytes([
            *map.get_unchecked(off * 4),
            *map.get_unchecked(off * 4 + 1),
            *map.get_unchecked(off * 4 + 2),
            *map.get_unchecked(off * 4 + 3),
        ])
    }
}

fn write_int(map: &mut [u8], off: usize, v: u32) {
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

    let home = std::env::var("HOME").expect("HOME is not defined in env");

    let mut rootpath = PathBuf::from(home);
    rootpath.push(".rtag/");

    std::fs::create_dir_all(&rootpath)
        .expect(&*format!("failed creating rtag dir at {:?}", &rootpath));

    rootpath.push("all");
    let mut allfile = File::options()
        .create(true)
        .write(true)
        .read(true)
        .open(&rootpath)
        .expect("could not open allfile");
    rootpath.pop();

    if allfile
        .metadata()
        .expect("cannot get metadata for allfile")
        .len()
        < 4
    {
        allfile.seek(SeekFrom::Start(0)).unwrap();
        allfile.write_all(&[0, 0, 0, 0]).unwrap();
    }

    let allmmap = unsafe { memmap2::Mmap::map(&allfile).expect("could not memmap all file") };

    let mut ctx = TagCtx {
        mapped_tags: BTreeMap::new(),
        allmap: allmmap,
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

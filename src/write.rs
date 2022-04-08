use crate::qry::{find, read_int, write_int};
use crate::{TagName, Value, ID};
use memmap2::Mmap;
use std::cmp::Ordering;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub fn getroot() -> PathBuf {
    let home = std::env::var("HOME").expect("HOME is not defined in env");

    let mut rootpath = PathBuf::from(home);
    rootpath.push(".rtag/");

    std::fs::create_dir_all(&rootpath)
        .expect(&*format!("failed creating rtag dir at {:?}", &rootpath));
    rootpath
}

pub fn get_allmap(root: &mut PathBuf) -> Mmap {
    root.push("all");
    let allfile = File::options()
        .create(true)
        .write(true)
        .read(true)
        .open(&root)
        .expect("could not open allfile");
    root.pop();

    unsafe { memmap2::Mmap::map(&allfile).expect("could not memmap all file") }
}

pub fn search_data(data: &[u8], needle: &[u8]) -> Option<usize> {
    let mut left = 0;
    let mut right = data.len() / 256;
    while left < right {
        let middle = (left + right) / 2;
        let v = &data[middle * 256..middle * 256 + 252];
        match v.cmp(needle) {
            Ordering::Equal => return Some(middle),
            Ordering::Greater => right = middle,
            Ordering::Less => left = middle + 1,
        }
    }
    None
}

pub fn insert_data(root: &mut PathBuf, value: Value) -> (ID, bool) {
    root.push("data");
    let mut datafile = File::options()
        .create(true)
        .write(true)
        .read(true)
        .open(&root)
        .expect("could not open allfile");
    root.pop();

    let data = unsafe {
        memmap2::Mmap::map(&datafile)
            .expect("could not memmap all file")
            .make_mut()
            .expect("cannot make mut map")
    };

    if data.len() % 256 != 0 {
        panic!("data is corrupted")
    }

    let mut bytes = value.0.into_bytes();
    if bytes.len() > 252 {
        panic!("value too long: max is 252 bytes")
    }
    bytes.extend((0..252 - bytes.len()).map(|_| 0));
    if let Some(off) = search_data(&data, &bytes) {
        return (ID(read_int(&data, off * 64 + 63)), false);
    }

    let newid = if data.len() == 0 {
        1
    } else {
        read_int(&data, data.len() / 4 - 1) + 1
    };
    drop(data);

    datafile
        .seek(SeekFrom::End(0))
        .expect("failed seeking to end");
    bytes.extend(u32::to_le_bytes(newid));
    assert_eq!(bytes.len(), 256);
    datafile
        .write_all(&bytes)
        .expect("failed adding new id to data");

    return (ID(newid), true);
}

pub fn insert_tag_in_map(root: &mut PathBuf, name: &str, id: ID) {
    root.push(name);
    let mut file = File::options()
        .create(true)
        .write(true)
        .read(true)
        .open(&root)
        .expect("could not open allfile");
    root.pop();

    {
        let map = unsafe { memmap2::Mmap::map(&file).expect("could not memmap all file") };

        if find(&map, id) {
            return;
        }
    }

    let mut data = vec![];
    file.read_to_end(&mut data).expect("could not read file");
    let mut buf = id.0;
    for i in 0..data.len() / 4 {
        let v = read_int(&data, i);
        if v < id.0 {
            continue;
        }
        write_int(&mut data, i, buf);
        buf = v;
    }
    data.extend_from_slice(&u32::to_le_bytes(buf));

    file.seek(SeekFrom::Start(0)).expect("could not seek");
    file.write_all(&data).expect("could not write");
}

pub fn add_tag(tag: &TagName, value: Value) {
    let mut root = getroot();

    let (dataid, inserted) = insert_data(&mut root, value);

    if inserted {
        insert_tag_in_map(&mut root, "all", dataid);
    }
    insert_tag_in_map(&mut root, &tag.0, dataid);
}

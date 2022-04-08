use crate::qry::{find, read_int, write_int};
use crate::{TagName, Value, ID};
use memmap2::Mmap;
use std::fs::File;
use std::io::{Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub const MAX_VALUE_LENGTH: usize = 251;

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

pub fn value_from_off(datamap: &[u8], off: usize) -> Value {
    let bytes = &datamap[off * 256..off * 256 + MAX_VALUE_LENGTH + 1];
    let len = bytes[0] as usize;
    Value(
        String::from_utf8(bytes[1..len + 1].iter().copied().collect()).expect("data is corrupted"),
    )
}

// Finds the ID with a corresponding data if it exists
pub fn data(datamap: &[u8], needle: ID) -> Option<Value> {
    let mut left = 0;
    let mut right = datamap.len() / 256;
    while left < right {
        let middle = (left + right) / 2;
        let v = read_int(datamap, middle * 64 + 63);
        if v == needle.0 {
            return Some(value_from_off(datamap, middle));
        }
        if v > needle.0 {
            right = middle;
        } else {
            left = middle + 1;
        }
    }
    None
}

// Finds the ID with a corresponding data if it exists
pub fn search_data(data: &[u8], needle: &[u8]) -> Option<ID> {
    assert_eq!(needle.len(), MAX_VALUE_LENGTH + 1);

    for i in 0..data.len() / 256 {
        let v = &data[i * 256..i * 256 + MAX_VALUE_LENGTH + 1];
        if v == needle {
            return Some(ID(read_int(data, i * 64 + 63)));
        }
    }
    None
}

pub fn get_datamap(root: &mut PathBuf) -> (Mmap, File) {
    root.push("data");
    let datafile = File::options()
        .create(true)
        .write(true)
        .read(true)
        .open(&root)
        .expect("could not open datafile");
    root.pop();

    (
        unsafe { memmap2::Mmap::map(&datafile).expect("could not memmap data file") },
        datafile,
    )
}

fn prepare_data_needle(value: Value) -> Vec<u8> {
    let mut bytes = vec![value.0.len() as u8];
    bytes.extend(value.0.bytes());
    bytes.extend((0..MAX_VALUE_LENGTH + 1 - bytes.len()).map(|_| 0));
    bytes
}

pub fn insert_data(root: &mut PathBuf, value: Value) -> (ID, bool) {
    let (data, mut datafile) = get_datamap(root);
    let data = data.make_mut().expect("could not make mat mup");

    if data.len() % 256 != 0 {
        panic!("data is corrupted")
    }
    if value.0.len() > MAX_VALUE_LENGTH {
        panic!("value too long: max is {} bytes", MAX_VALUE_LENGTH)
    }
    let mut bytes = prepare_data_needle(value);
    if let Some(id) = search_data(&data, &bytes) {
        return (id, false);
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

pub fn remove_tag_from_map(root: &mut PathBuf, name: &str, id: ID) {
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

        if !find(&map, id) {
            return;
        }
    }

    let mut data = vec![];
    file.read_to_end(&mut data).expect("could not read file");
    let mut i = 0;
    while i < data.len() / 4 {
        let v = read_int(&data, i);
        if v < id.0 {
            i += 1;
            continue;
        }
        if v == id.0 {
            break;
        }
    }
    while i < data.len() / 4 {
        let v = read_int(&data, i + 1);
        write_int(&mut data, i, v);
        i += 1;
    }
    if data.len() <= 4 {
        root.push(name);
        std::fs::remove_file(&root).expect("could not delete tagmap");
        root.pop();
        return;
    }
    file.set_len((data.len() - 4) as u64)
        .expect("could not truncate tagmap");
}

pub fn add_tag(tag: &TagName, value: Value) {
    let mut root = getroot();

    let (dataid, inserted) = insert_data(&mut root, value);

    if inserted {
        insert_tag_in_map(&mut root, "all", dataid);
    }
    insert_tag_in_map(&mut root, &tag.0, dataid);
}

pub fn del_tag(tag: &TagName, value: Value) {
    if value.0.len() > MAX_VALUE_LENGTH {
        panic!("value too big");
    }
    let mut root = getroot();

    let (datamap, _) = get_datamap(&mut root);
    let needle = prepare_data_needle(value);

    let id = search_data(&datamap, &needle);
    let id = if let Some(x) = id { x } else { return };

    remove_tag_from_map(&mut root, &tag.0, id);
}

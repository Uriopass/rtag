mod dnf;
mod parse;
mod qry;
mod write;

use crate::write::{add_tag, del_tag, getroot};
use clap::{Parser, Subcommand};
use std::collections::hash_map::DefaultHasher;
use std::hash::{Hash, Hasher};

#[derive(Copy, Clone, Ord, PartialOrd, Eq, PartialEq)]
#[repr(transparent)]
pub struct ID(u32);

pub struct Value(String);

#[derive(PartialOrd, Ord, Clone, Debug, Eq, PartialEq)]
pub struct TagName(String);

#[derive(Parser)]
#[clap(name = "rtag")]
#[clap(author = "Pâris D. <paris.douady@hotmail.fr>")]
#[clap(version = "0.1")]
#[clap(about = "General Tagging CLI", long_about = None)]
#[clap(propagate_version = true)]
struct Cli {
    #[clap(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Queries the values from a tag query
    Qry { qry: String },
    /// Sets tag to values
    Set { tag: String, values: Vec<String> },
    /// Remove tag from values
    Del { tag: String, values: Vec<String> },
    /// Clean remove all tags and everything
    Clean {},
    /// Generate test data
    GenTestData { dataset: Option<u32> },
    /// List all tags
    Tags {},
}

fn cli() -> Cli {
    Cli::parse()
}

fn main() {
    let cli = cli();

    match cli.command {
        Commands::Qry { qry } => {
            for val in qry::parse_and_execute(&qry, 100) {
                println!("{}", val.0);
            }
        }
        Commands::Set { tag, values } => {
            let tag = TagName(tag);
            for val in values {
                add_tag(&tag, &Value(val));
            }
        }
        Commands::Del { tag, values } => {
            let tag = TagName(tag);
            for val in values {
                del_tag(&tag, &Value(val));
            }
        }
        Commands::Clean {} => {
            let root = getroot();
            std::fs::remove_dir_all(root).expect("failed cleaning");
        }
        Commands::Tags {} => {
            let root = getroot();
            for file in std::fs::read_dir(&root)
                .expect("cannot read root")
                .flat_map(|x| x.ok())
            {
                let fname = file.file_name();
                let name = fname.to_string_lossy();
                if name == "__all" || name == "__data" {
                    continue;
                }
                println!("{}", name);
            }
        }
        Commands::GenTestData { dataset } => match dataset {
            Some(2) => {
                println!("generating dataset: 50k items  10 tags  [1-10] items per tag");
                for musique in 0..50000 {
                    let mustag = Value(format!("m_{}", musique));
                    let mut st = DefaultHasher::new();
                    musique.hash(&mut st);
                    let n_items = st.finish() % 10;
                    for i in 0..n_items {
                        let mut st = DefaultHasher::new();
                        musique.hash(&mut st);
                        (i + 1).hash(&mut st);
                        let artiste = st.finish() % 10;

                        let artag = format!("a_{}", artiste);
                        add_tag(&TagName(artag), &mustag);
                    }
                }
            }
            _ => {
                println!("generating dataset: 50k items 2000 tags 1 item per tag");
                for musique in 0..50000 {
                    let mut st = DefaultHasher::new();
                    musique.hash(&mut st);
                    let artiste = st.finish() % 2000;

                    let mustag = format!("m_{}", musique);
                    let artag = format!("a_{}", artiste);
                    add_tag(&TagName(artag), &Value(mustag));
                }
            }
        },
    }
}

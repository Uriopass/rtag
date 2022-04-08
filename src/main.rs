mod cnf;
mod parse;
mod qry;
mod write;

use crate::write::add_tag;
use clap::{Parser, Subcommand};

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
}

fn cli() -> Cli {
    Cli::parse()
}

fn main() {
    let cli = cli();

    match cli.command {
        Commands::Qry { qry } => {
            for id in qry::parse_and_execute(&qry, 100) {
                println!("{}", id.0);
            }
        }
        Commands::Set { tag, values } => {
            let tag = TagName(tag);
            for val in values {
                add_tag(&tag, Value(val));
            }
        }
    }
}

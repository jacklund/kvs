extern crate structopt;

use std::process::exit;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct KvsOptions {
    #[structopt(subcommand)]
    command: KvsCommands,
}

#[derive(Debug, StructOpt)]
enum KvsCommands {
    #[structopt(name = "get")]
    Get { key: String },

    #[structopt(name = "rm")]
    Remove { key: String },

    #[structopt(name = "set")]
    Set { key: String, value: String },
}

fn main() {
    KvsOptions::from_args();

    eprintln!("unimplemented");
    exit(1);
}

extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate structopt;

use kvs::{KvsCommands, Result};

use std::io::{BufReader, BufWriter, Read, Write};
use std::net::TcpStream;
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct KvsOptions {
    #[structopt(subcommand)]
    command: KvsCommands,

    #[structopt(long = "addr", default_value = "127.0.0.1:4000")]
    address: String,
}

fn main() -> Result<()> {
    env_logger::init();
    let opts = KvsOptions::from_args();

    let stream = TcpStream::connect(opts.address)?;

    let mut reader = BufReader::new(&stream);
    let mut writer = BufWriter::new(&stream);

    match opts.command {
        KvsCommands::Get { key } => {
            serde_json::to_writer(&mut writer, &KvsCommands::Get { key })?;
        }
        KvsCommands::Set { key, value } => {
            serde_json::to_writer(
                &mut writer,
                &KvsCommands::Set {
                    key: key.clone(),
                    value: value.clone(),
                },
            )?;
        }
        KvsCommands::Remove { key } => {
            serde_json::to_writer(&mut writer, &KvsCommands::Remove { key })?;
        }
    }
    writer.flush()?;
    let mut buffer = String::new();
    reader.read_to_string(&mut buffer)?;
    if !buffer.is_empty() {
        if buffer.starts_with("Server error:") {
            eprintln!("{}", buffer);
            std::process::exit(1);
        } else {
            println!("{}", buffer);
        }
    }

    Ok(())
}

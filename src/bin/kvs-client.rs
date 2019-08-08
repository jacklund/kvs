extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate structopt;

use kvs::{KvsCommands, Result};

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

    info!("Connecting to {}", opts.address);
    let mut stream = TcpStream::connect(opts.address)?;
    debug!("Connected to {:?}", stream);

    match opts.command {
        KvsCommands::Get { ref key } => {
            serde_json::to_writer(&mut stream, &KvsCommands::Get { key: key.clone() })?;
        }
        KvsCommands::Set { key, value } => {
            serde_json::to_writer(
                &mut stream,
                &KvsCommands::Set {
                    key: key.clone(),
                    value: value.clone(),
                },
            )?;
        }
        KvsCommands::Remove { key } => {
            serde_json::to_writer(&mut stream, &KvsCommands::Remove { key: key })?;
        }
    }

    Ok(())
}

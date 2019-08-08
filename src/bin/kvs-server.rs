#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate structopt;

use kvs::{KvsCommands, Result};

use std::net::{TcpListener, TcpStream};
use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct KvsOptions {
    #[structopt(long = "addr", default_value = "127.0.0.1:4000")]
    address: String,

    #[structopt(long = "engine")]
    engine: Option<String>,
}

fn main() -> Result<()> {
    env_logger::init();
    let opts = KvsOptions::from_args();

    info!("kvs-server {}", crate_version!());

    let listener = TcpListener::bind(opts.address.clone())?;
    info!("listening on {}", opts.address);

    for stream in listener.incoming() {
        handle_client(stream?)?;
    }

    Ok(())
}

fn handle_client(stream: TcpStream) -> Result<()> {
    debug!("Got connection: {:#?}", stream);

    let mut command_stream =
        serde_json::Deserializer::from_reader(stream).into_iter::<KvsCommands>();

    while let Some(command) = command_stream.next() {
        info!("Got command: {:?}", command?);
    }

    Ok(())
}

#[macro_use]
extern crate clap;
extern crate env_logger;
#[macro_use]
extern crate log;
extern crate serde_json;
extern crate structopt;

use kvs::{KvStore, KvsCommands, KvsEngine, Result, SledKvsEngine};

use env_logger::Builder;
use log::LevelFilter;
use std::env::{current_dir, var_os};
use std::fs::read_to_string;
use std::io::{BufReader, BufWriter, Write};
use std::net::{Shutdown, SocketAddr, TcpListener, TcpStream};
use structopt::StructOpt;

const DEFAULT_LISTENING_ADDRESS: &str = "127.0.0.1:4000";
const DEFAULT_ENGINE: EngineName = EngineName::kvs;

#[derive(StructOpt, Debug)]
struct KvsOptions {
    #[structopt(
        long,
        help = "Sets the listening address",
        value_name = "IP:PORT",
        raw(default_value = "DEFAULT_LISTENING_ADDRESS"),
        parse(try_from_str)
    )]
    addr: SocketAddr,

    #[structopt(
        long,
        help = "Storage engine name",
        value_name = "ENGINE-NAME",
        raw(possible_values = "&EngineName::variants()")
    )]
    engine: Option<EngineName>,
}

arg_enum! {
    #[allow(non_camel_case_types)]
    #[derive(Debug, Copy, Clone, PartialEq, Eq)]
    enum EngineName {
        kvs,
        sled,
    }
}

struct Server<E: KvsEngine> {
    engine: E,
}

impl<E: KvsEngine> Server<E> {
    fn new(engine: E) -> Server<E> {
        Server { engine }
    }

    fn start(&mut self, address: &SocketAddr) -> Result<()> {
        let listener = TcpListener::bind(address.clone())?;
        info!("listening on {}", address);
        debug!("Current directory is {:?}", current_dir()?);

        for stream in listener.incoming() {
            self.handle_client(stream?)?;
        }

        Ok(())
    }

    fn handle_client(&mut self, stream: TcpStream) -> Result<()> {
        debug!("Got connection: {:#?}", stream);

        let reader = BufReader::new(&stream);
        let mut writer = BufWriter::new(&stream);

        let command_stream =
            serde_json::Deserializer::from_reader(reader).into_iter::<KvsCommands>();

        for command_result in command_stream {
            let command = command_result?;
            debug!("Got command: {:?}", command);
            match command {
                KvsCommands::Get { key } => {
                    debug!("Get, key = {}", key);
                    match self.engine.get(key) {
                        Ok(response) => {
                            match response {
                                None => {
                                    debug!("Got None");
                                    writer.write(b"Key not found")?;
                                }
                                Some(value) => {
                                    debug!("Got {}", value);
                                    writer.write(value.as_bytes())?;
                                }
                            };
                        }
                        Err(e) => {
                            writer.write(format!("Server error: {}", e).as_bytes())?;
                        }
                    };
                }
                KvsCommands::Set { key, value } => {
                    if let Err(e) = self.engine.set(key, value) {
                        writer.write(format!("Server error: {}", e).as_bytes())?;
                    }
                }
                KvsCommands::Remove { key } => {
                    if let Err(e) = self.engine.remove(key) {
                        debug!("Got error: {}", e);
                        writer.write(format!("Server error: {}", e).as_bytes())?;
                    }
                }
            }
            writer.flush()?;
            stream.shutdown(Shutdown::Both)?;
        }

        debug!("Exiting command loop");

        Ok(())
    }
}

fn main() -> Result<()> {
    // Default log level is "info"
    match var_os("RUST_LOG") {
        None => {
            let mut builder = Builder::from_default_env();
            builder.filter_level(LevelFilter::Info).init();
        }
        Some(_) => env_logger::init(),
    };

    let opts = KvsOptions::from_args();

    info!("kvs {}", crate_version!());

    let arg_engine = opts.engine.unwrap_or(DEFAULT_ENGINE);
    debug!("Engine {} from command line args", arg_engine);
    let current_engine = read_current_engine_name()?;
    if let Some(current_engine_name) = current_engine {
        if arg_engine != current_engine_name {
            error!("Engine requested doesn't match engine last used");
            std::process::exit(1);
        }
    }

    debug!("Using {} engine", arg_engine);

    write_current_engine_name(arg_engine)?;

    match arg_engine {
        EngineName::kvs => Server::<KvStore>::new(KvStore::open(".")?).start(&opts.addr),
        EngineName::sled => {
            Server::<SledKvsEngine>::new(SledKvsEngine::open(".")?).start(&opts.addr)
        }
    }?;

    Ok(())
}

fn write_current_engine_name(engine: EngineName) -> Result<()> {
    let engine_file = current_dir()?.join("engine");
    debug!("writing engine name '{}' to {:?}", engine, engine_file);
    std::fs::write(engine_file, engine.to_string())?;
    Ok(())
}

fn read_current_engine_name() -> Result<Option<EngineName>> {
    let engine_file = current_dir()?.join("engine");
    if engine_file.exists() {
        match read_to_string(engine_file)?.parse() {
            Ok(engine) => Ok(Some(engine)),
            Err(_) => {
                warn!("Unparseable engine file");
                Ok(None)
            }
        }
    } else {
        Ok(None)
    }
}

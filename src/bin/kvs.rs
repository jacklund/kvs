extern crate serde_json;
extern crate structopt;

use kvs::{KvStore, KvsCommands, KvsError, Result};

use structopt::StructOpt;

#[derive(Debug, StructOpt)]
struct KvsOptions {
    #[structopt(subcommand)]
    command: KvsCommands,
}

fn main() -> Result<()> {
    let opts = KvsOptions::from_args();

    let mut store = KvStore::open(".")?;

    match opts.command {
        KvsCommands::Get { ref key } => match store.get(key.to_string())? {
            Some(value) => println!("{}", value),
            None => println!("Key not found"),
        },
        KvsCommands::Set { key, value } => store.set(key, value)?,
        KvsCommands::Remove { key } => match store.remove(key) {
            Ok(_) => (),
            Err(KvsError::KeyNotFound) => {
                println!("Key not found");
                std::process::exit(1);
            }
            Err(error) => return Err(error),
        },
    }

    Ok(())
}

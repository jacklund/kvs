extern crate serde;
extern crate structopt;

pub mod error;
pub mod kvsengine;
pub mod kvstore;
pub mod sledkvsengine;

pub use error::{KvsError, Result};
pub use kvsengine::KvsEngine;
pub use kvstore::KvStore;

use serde::{Deserialize, Serialize};
use structopt::StructOpt;

#[derive(Debug, Deserialize, Serialize, StructOpt)]
pub enum KvsCommands {
    #[structopt(name = "get")]
    Get { key: String },

    #[structopt(name = "rm")]
    Remove { key: String },

    #[structopt(name = "set")]
    Set { key: String, value: String },
}

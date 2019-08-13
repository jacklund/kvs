use crate::{KvsEngine, KvsError, Result};

use sled::Db;
use std::path::PathBuf;

pub struct SledKvsEngine {
    db: Db,
}

impl SledKvsEngine {
    pub fn open(pathbuf: impl Into<PathBuf>) -> Result<SledKvsEngine> {
        Ok(SledKvsEngine {
            db: Db::start_default(&pathbuf.into())?,
        })
    }
}

impl KvsEngine for SledKvsEngine {
    #[logfn(Trace)]
    fn get(&mut self, key: String) -> Result<Option<String>> {
        Ok(self
            .db
            .get(key)?
            .map(|i_vec| AsRef::<[u8]>::as_ref(&i_vec).to_vec())
            .map(String::from_utf8)
            .transpose()?)
    }

    #[logfn(Trace)]
    fn set(&mut self, key: String, value: String) -> Result<()> {
        self.db.set(key.clone(), value.as_bytes())?;
        self.db.flush()?;
        Ok(())
    }

    #[logfn(Trace)]
    fn remove(&mut self, key: String) -> Result<()> {
        let ret = match self.db.del(key)? {
            Some(_) => {
                debug!("remove, found previous value");
                Ok(())
            }
            None => {
                debug!("remove, no previous value found");
                Err(KvsError::KeyNotFound)
            }
        };
        self.db.flush()?;

        ret
    }
}

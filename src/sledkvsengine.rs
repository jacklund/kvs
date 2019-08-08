use crate::KvsEngine;
use crate::Result;

pub struct SledKvsEngine {}

impl KvsEngine for SledKvsEngine {
    fn get(&mut self, _key: String) -> Result<Option<String>> {
        unimplemented!()
    }

    fn set(&mut self, _key: String, _value: String) -> Result<()> {
        unimplemented!()
    }

    fn remove(&mut self, _key: String) -> Result<()> {
        unimplemented!()
    }
}

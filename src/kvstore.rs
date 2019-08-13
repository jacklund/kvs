use crate::KvsEngine;
use crate::{KvsCommands, KvsError, Result};
use serde_json;
use std::collections::HashMap;
use std::fs::{create_dir_all, read_dir, remove_file, File, OpenOptions};
use std::io::{self, BufReader, BufWriter, Read, Seek, SeekFrom, Write};
use std::path::PathBuf;

pub struct KvStore {
    store: HashMap<String, FileLocation>,
    writer: KvWriter<File>,
    readers: HashMap<u64, BufReader<File>>,
    gen: u64,
    compactible: u64,
    path: PathBuf,
}

#[derive(Clone, Debug)]
pub struct FileLocation {
    gen: u64,
    offset: u64,
    length: u64,
}

impl FileLocation {
    pub fn new(gen: u64, offset: u64, length: u64) -> Self {
        FileLocation {
            gen,
            offset,
            length,
        }
    }
}

pub struct KvWriter<W: Write + Seek> {
    writer: BufWriter<W>,
    offset: u64,
}

impl<W: Write + Seek> KvWriter<W> {
    pub fn new(mut writer: W) -> Result<Self> {
        let offset = writer.seek(SeekFrom::End(0))?;
        Ok(KvWriter {
            writer: BufWriter::new(writer),
            offset,
        })
    }
}

impl<W: Write + Seek> Write for KvWriter<W> {
    fn write(&mut self, buf: &[u8]) -> io::Result<usize> {
        let len = self.writer.write(buf)?;
        self.offset += len as u64;
        Ok(buf.len())
    }

    fn flush(&mut self) -> io::Result<()> {
        self.writer.flush()
    }
}

const COMPACTION_THRESHOLD: u64 = 1024 * 1024;

impl KvStore {
    pub fn open(path: impl Into<PathBuf>) -> Result<KvStore> {
        let path = path.into();
        create_dir_all(&path)?;
        let gen_list = gen_list(&path)?;
        let mut store: HashMap<String, FileLocation> = HashMap::new();
        let mut readers: HashMap<u64, BufReader<File>> = HashMap::new();
        let mut compactible: u64 = 0;
        for gen in &gen_list {
            let mut reader = get_reader(&path, *gen)?;
            compactible += load(*gen, &mut reader, &mut store)?;
            readers.insert(*gen, reader);
        }
        let latest_gen = *gen_list.last().unwrap_or(&(1 as u64));
        let writer = KvWriter::new(open_log_file(&path, latest_gen, false)?)?;
        if latest_gen == 1 && readers.is_empty() {
            readers.insert(latest_gen, get_reader(&path, latest_gen)?);
        }
        debug!(
            "KvStore::open, gen = {}, compactible = {}, path = {:?}",
            latest_gen, compactible, path
        );
        Ok(KvStore {
            store,
            writer,
            readers,
            gen: latest_gen,
            compactible,
            path,
        })
    }

    fn compact(&mut self) -> Result<()> {
        debug!("Compacting, compactible = {}", self.compactible);
        // Get list of files we need to delete
        let list = gen_list(&self.path)?;

        // Increment the generation counter
        self.gen += 1;
        debug!("Compacting, new gen = {}", self.gen);

        // New writer, and clear out the store
        self.writer = KvWriter::new(open_log_file(&self.path, self.gen, false)?)?;
        let store = self.store.clone();
        self.store.clear();

        // Read from the old files and write the new
        for (key, location) in store.iter() {
            let reader = self
                .readers
                .get_mut(&location.gen)
                .expect("Cannot find log reader");
            reader.seek(SeekFrom::Start(location.offset))?;
            let command = serde_json::from_reader(reader.take(location.length as u64))?;
            if let KvsCommands::Set { value, .. } = command {
                self.set(key.to_string(), value)?;
            } else {
                return Err(KvsError::UnexpectedCommandType);
            }
        }

        // Delete all the old files
        for gen in list {
            let path = log_file(&self.path, gen)?;
            debug!("Compacting, deleting gen file {:?}", path);
            remove_file(path)?;
        }

        // Generate new readers
        self.readers.clear();
        for gen in gen_list(&self.path)? {
            let reader = get_reader(&self.path, gen)?;
            self.readers.insert(gen, reader);
        }

        Ok(())
    }
}

impl KvsEngine for KvStore {
    fn get(&mut self, key: String) -> Result<Option<String>> {
        debug!("KvStore::get({})", key);
        match self.store.get(&key) {
            None => Ok(None),
            Some(location) => {
                let reader = self
                    .readers
                    .get_mut(&location.gen)
                    .expect("Cannot find log reader");
                reader.seek(SeekFrom::Start(location.offset))?;
                let command = serde_json::from_reader(reader.take(location.length as u64))?;
                if let KvsCommands::Set { value, .. } = command {
                    Ok(Some(value))
                } else {
                    Err(KvsError::UnexpectedCommandType)
                }
            }
        }
    }

    fn set(&mut self, key: String, value: String) -> Result<()> {
        debug!("KvStore::set({}, {})", key, value);
        let original_offset = self.writer.offset;
        serde_json::to_writer(
            &mut self.writer,
            &KvsCommands::Set {
                key: key.clone(),
                value: value.clone(),
            },
        )?;
        self.writer.flush()?;
        let old_location = self.store.insert(
            key,
            FileLocation::new(
                self.gen,
                original_offset,
                self.writer.offset - original_offset,
            ),
        );
        if let Some(location) = old_location {
            self.compactible += location.length;
            if self.compactible >= COMPACTION_THRESHOLD {
                self.compact()?;
            }
        }
        Ok(())
    }

    fn remove(&mut self, key: String) -> Result<()> {
        debug!("KvStore::remove({})", key);
        match self.store.remove(&key) {
            Some(location) => {
                let orig_offset = self.writer.offset;
                serde_json::to_writer(&mut self.writer, &KvsCommands::Remove { key })?;
                self.writer.flush()?;
                let command_size = self.writer.offset - orig_offset;
                self.compactible += location.length + command_size;
                if self.compactible >= COMPACTION_THRESHOLD {
                    self.compact()?;
                }
                Ok(())
            }
            None => Err(KvsError::KeyNotFound),
        }
    }
}

fn gen_list(path: &PathBuf) -> Result<Vec<u64>> {
    let pathbufs: Vec<PathBuf> = read_dir(path)?.flatten().map(|d| d.path()).collect();
    let mut numbers: Vec<u64> = pathbufs
        .iter()
        .map(|p| p.file_stem())
        .flatten()
        .map(|s| s.to_str())
        .flatten()
        .map(|s| s.parse::<u64>())
        .flatten()
        .collect::<Vec<u64>>();
    numbers.sort();
    Ok(numbers)
}

fn log_file(path: &PathBuf, gen: u64) -> Result<PathBuf> {
    Ok(path.join(format!("{}.log", gen.to_string())))
}

fn open_log_file(path: &PathBuf, gen: u64, readonly: bool) -> Result<File> {
    let log_file_path = log_file(path, gen)?;

    if readonly {
        Ok(OpenOptions::new().read(true).open(log_file_path)?)
    } else {
        Ok(OpenOptions::new()
            .read(true)
            .append(true)
            .create(true)
            .open(log_file_path)?)
    }
}

fn get_reader(path: &PathBuf, gen: u64) -> Result<BufReader<File>> {
    Ok(BufReader::new(open_log_file(path, gen, true)?))
}

fn load(
    gen: u64,
    reader: &mut BufReader<File>,
    store: &mut HashMap<String, FileLocation>,
) -> Result<u64> {
    let mut offset = reader.seek(SeekFrom::Start(0))?;
    let mut stream = serde_json::Deserializer::from_reader(reader).into_iter::<KvsCommands>();
    let mut compactible = 0 as u64;
    while let Some(command) = stream.next() {
        let new_offset = stream.byte_offset() as u64;
        match command? {
            KvsCommands::Set { key, .. } => {
                if let Some(old_location) = store.insert(
                    key,
                    FileLocation {
                        gen,
                        offset,
                        length: new_offset - offset,
                    },
                ) {
                    compactible += old_location.length;
                }
            }
            KvsCommands::Remove { key } => {
                if let Some(old_location) = store.remove(&key) {
                    compactible += old_location.length;
                }
                compactible += new_offset - offset;
            }
            KvsCommands::Get { .. } => return Err(KvsError::UnexpectedCommandType),
        }
        offset = new_offset;
    }
    Ok(compactible)
}

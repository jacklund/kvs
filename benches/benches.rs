#[macro_use]
extern crate criterion;
extern crate kvs;
#[macro_use]
extern crate lazy_static;
extern crate rand;

use crate::kvs::{KvStore, KvsEngine, Result, SledKvsEngine};
use crate::rand::Rng;
use criterion::Criterion;

fn generate_random_length_string(len: usize) -> String {
    let mut rng = rand::thread_rng();
    (0..rng.gen_range(0, len)).map(|_| "X").collect::<String>()
}

fn generate_random_string_pairs(num: usize, len: usize) -> Vec<(String, String)> {
    (0..num)
        .map(|_| {
            (
                generate_random_length_string(len),
                generate_random_length_string(len),
            )
        })
        .collect::<Vec<(String, String)>>()
}

lazy_static! {
    static ref PAIRS: Vec<(String, String)> = generate_random_string_pairs(100, 100000);
}

fn kvs_write(c: &mut Criterion) -> Result<()> {
    let mut kvstore = KvStore::open(".")?;
    c.bench_function("kvs_write", move |b| {
        let pairs = PAIRS.clone();
        b.iter(|| {
            for (key, value) in pairs.clone() {
                kvstore.set(key, value);
            }
        });
    });

    Ok(())
}

fn kvs_read(c: &mut Criterion) -> Result<()> {
    let mut kvstore = KvStore::open(".")?;
    c.bench_function("kvs_read", move |b| {
        let pairs = PAIRS.clone();
        b.iter(|| {
            for (key, _) in pairs.clone() {
                kvstore.get(key);
            }
        });
    });

    Ok(())
}

fn sled_write(c: &mut Criterion) -> Result<()> {
    let mut sled = SledKvsEngine::open(".")?;
    c.bench_function("sled_write", move |b| {
        let pairs = PAIRS.clone();
        b.iter(|| {
            for (key, value) in pairs.clone() {
                sled.set(key, value);
            }
        });
    });

    Ok(())
}

fn sled_read(c: &mut Criterion) -> Result<()> {
    let mut sled = SledKvsEngine::open(".")?;
    c.bench_function("sled_read", move |b| {
        let pairs = PAIRS.clone();
        b.iter(|| {
            for (key, _) in pairs.clone() {
                sled.get(key);
            }
        });
    });

    Ok(())
}

criterion_group!(benches, kvs_write, kvs_read, sled_write, sled_read);
criterion_main!(benches);

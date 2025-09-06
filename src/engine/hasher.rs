use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
};
use rayon::{ThreadPool, ThreadPoolBuilder};
use once_cell::sync::Lazy;
use sha2::{Sha256, Digest};
use blake3::Hasher as Blake3Inner;

use crate::errors::Result;

static RAYON_POOL: Lazy<Arc<ThreadPool>> = Lazy::new(|| {
    Arc::new(
        ThreadPoolBuilder::new()
            .num_threads(num_cpus::get()) // max out CPU
            .build()
            .expect("Failed to build Rayon pool"),
    )
});

const BUFFER_SIZE: usize = 8192; // 8KB

/// Generic interface for all hashers
#[async_trait]
pub trait FileHasher: Send + Sync {
    async fn hash_file(&self, path: &Path) -> Result<Vec<u8>>;
}

pub struct Sha256Hasher;

#[async_trait]
impl FileHasher for Sha256Hasher {
    async fn hash_file(&self, path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path).await?;
        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
        let mut buf = vec![0u8; BUFFER_SIZE];

        let mut hasher = Sha256::new();

        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }

            // Send CPU-heavy hashing into Rayon pool
            RAYON_POOL.install(|| {
                hasher.update(&buf[..n]);
            });
        }

        Ok(hasher.finalize().to_vec())
    }
}

pub struct Blake3Hasher;

#[async_trait]
impl FileHasher for Blake3Hasher {
    async fn hash_file(&self, path: &Path) -> Result<Vec<u8>> {
        let file = File::open(path).await?;
        let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
        let mut buf = vec![0u8; BUFFER_SIZE];

        let mut hasher = Blake3Inner::new();

        loop {
            let n = reader.read(&mut buf).await?;
            if n == 0 {
                break;
            }

            // BLAKE3 is parallel internally, but keep consistency
            RAYON_POOL.install(|| {
                hasher.update(&buf[..n]);
            });
        }

        Ok(hasher.finalize().as_bytes().to_vec())
    }
}

pub enum HashAlgo {
    Sha256,
    Blake3,
}

pub fn create_hasher(algo: HashAlgo) -> Arc<dyn FileHasher> {
    match algo {
        HashAlgo::Sha256 => Arc::new(Sha256Hasher),
        HashAlgo::Blake3 => Arc::new(Blake3Hasher),
    }
}


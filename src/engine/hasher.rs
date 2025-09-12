use async_trait::async_trait;
use std::{path::Path, sync::Arc};
use tokio::{
    fs::File,
    io::{AsyncReadExt, BufReader},
    task,
};
use sha2::{Sha256, Digest};
use blake3::Hasher as Blake3Inner;

use crate::errors::Result;

const BUFFER_SIZE: usize = 8192; // 8KB
const BLOCKING_THRESHOLD: u64 = 50 * 1024 * 1024; // 50MB

/// Generic interface for all hashers
#[async_trait]
pub trait FileHasher: Send + Sync {
    async fn hash_file(&self, path: &Path) -> Result<Vec<u8>>;
}

/// ---------------- Shared Helper ----------------
async fn hash_file_with<H, U, F>(
    path: &Path,
    init: impl Fn() -> H + Send + 'static,
    mut update: U,
    finalize: F,
) -> Result<Vec<u8>>
where 
    H: Send + 'static,
    U: FnMut(&mut H, &[u8]) + Send + 'static,
    F: FnOnce(H) -> Vec<u8> + Send + 'static,
{
    let metadata = tokio::fs::metadata(path).await?;

    // Large file -> offload to blocking thread
    if metadata.len() > BLOCKING_THRESHOLD {
        let path = path.to_owned();
        let digest = task::spawn_blocking(move || -> Result<Vec<u8>> {
            use std::{fs::File, io::{Read, BufReader}};
            let mut file = BufReader::with_capacity(BUFFER_SIZE, File::open(path)?);
            let mut buf = vec![0u8; BUFFER_SIZE];
            let mut hasher = init();

            loop {
                let n = file.read(&mut buf)?;
                if n == 0 { break; }
                update(&mut hasher, &buf[..n]);
            }

            Ok(finalize(hasher))
        })
        .await??;

        return Ok(digest);
    }

    // Smalll file -> inline async loop
    let file = File::open(path).await?;
    let mut reader = BufReader::with_capacity(BUFFER_SIZE, file);
    let mut buf = vec![0u8; BUFFER_SIZE];
    let mut hasher = init();

    loop {
        let n = reader.read(&mut buf).await?;
        if n == 0 { break; }
        update(&mut hasher, &buf[..n]);
    }

    Ok(finalize(hasher))
}

/// ---------------- SHA256 ----------------
pub struct Sha256Hasher;

#[async_trait]
impl FileHasher for Sha256Hasher {
    async fn hash_file(&self, path: &Path) -> Result<Vec<u8>> {
        hash_file_with(
            path, 
            Sha256::new, 
            |h, chunk| h.update(chunk), 
            |h| h.finalize().to_vec(),
        )
        .await
    }
}

/// ---------------- BLAKE3 ----------------
pub struct Blake3Hasher;

#[async_trait]
impl FileHasher for Blake3Hasher {
    async fn hash_file(&self, path: &Path) -> Result<Vec<u8>> {
        hash_file_with(
            path,
            Blake3Inner::new,
            |h, chunk| { h.update(chunk); },
            |h| h.finalize().as_bytes().to_vec(),
        )
        .await
    }
}

/// ---------------- Factory ----------------
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


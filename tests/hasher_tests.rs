#[cfg(test)]
mod tests {
    use std::io::Write;

    use stash::hasher::{Blake3Hasher, FileHasher, Sha256Hasher};
    use tokio::io::AsyncWriteExt;
    use tempfile::NamedTempFile;
    use sha2::Digest;
    use blake3;

    const BLOCKING_THRESHOLD: u64 = 50 * 1024 * 1024; // 50MB

    /// Helper: write bytes into a fresh temp file and return its PathBuf
    async fn write_temp_file(content: &[u8]) -> std::path::PathBuf {
        let mut tmp = NamedTempFile::new().unwrap();
        tmp.write_all(content).unwrap();
        let path = tmp.into_temp_path().to_path_buf();

        // Make sure it's flushed for async readers
        let mut f = tokio::fs::File::create(&path).await.unwrap();
        f.write_all(content).await.unwrap();
        f.flush().await.unwrap();

        path
    }

    async fn check_hasher<H: FileHasher>(
        hasher: H,
        content: &[u8],
        expected: Vec<u8>,
    ) {
        let path = write_temp_file(content).await;
        let digest = hasher.hash_file(&path).await.unwrap();
        assert_eq!(digest, expected);
        tokio::fs::remove_file(path).await.unwrap();
    }

    #[tokio::test]
    async fn sha256_small_file() {
        let content = b"hello world";
        let mut ref_hasher = sha2::Sha256::new();
        ref_hasher.update(content);
        let expected = ref_hasher.finalize().to_vec();

        check_hasher(Sha256Hasher, content, expected).await;
    }

    #[tokio::test]
    async fn sha256_large_file() {
        let content = vec![b'a'; (BLOCKING_THRESHOLD as usize) + 1024];
        let mut ref_hasher = sha2::Sha256::new();
        ref_hasher.update(&content);
        let expected = ref_hasher.finalize().to_vec();

        check_hasher(Sha256Hasher, &content, expected).await;
    }

    #[tokio::test]
    async fn blake3_small_file() {
        let content = b"hello world";
        let expected = blake3::hash(content).as_bytes().to_vec();

        check_hasher(Blake3Hasher, content, expected).await;
    }

    #[tokio::test]
    async fn blake3_large_file() {
        let content = vec![b'b'; (BLOCKING_THRESHOLD as usize) + 2048];
        let expected = blake3::hash(&content).as_bytes().to_vec();

        check_hasher(Blake3Hasher, &content, expected).await;
    }
}
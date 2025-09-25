use std::{collections::HashSet, path::Path, sync::Arc};
use tokio::{
    fs, io::{self, AsyncWriteExt}, sync::RwLock, task
};
use tracing::{debug, instrument};

use crate::errors::Result;

#[derive(Debug, Clone)]
pub struct FileMover {
    created_dirs: Arc<RwLock<HashSet<String>>>,
}

impl FileMover {
    pub fn new() -> Self {
        Self {
            created_dirs: Arc::new(RwLock::new(HashSet::new())),
        }
    }

    /// Ensure parent dir exists (creates once, cache result)
    pub async fn ensure_parent_dir(&self, dest: &Path) -> Result<()> {
        if let Some(parent) = dest.parent() {
            let dir_str = parent.to_string_lossy().to_string();

            {
                let cache = self.created_dirs.read().await;
                if cache.contains(&dir_str) {
                    return Ok(())
                }
            }

            fs::create_dir_all(parent).await?;

            let mut cache = self.created_dirs.write().await;
            cache.insert(dir_str);
        }
        Ok(())
    }

    #[cfg(unix)]
    fn is_cross_device_error(e: &io::Error) -> bool {
        e.kind() == io::ErrorKind::CrossesDevices
    }

    #[cfg(windows)]
    fn is_cross_device_error(e: &io::Error) -> bool {
        // Windows returns ERROR_NOT_SAME_DEVICE (17) for cross-device moves
        e.raw_os_error() == Some(17)
    }

    #[cfg(not(any(unix, windows)))]
    fn is_cross_device_error(_e: &io::Error) -> bool {
        false
    }

    /// Move file, falling back to copy+delete if across devices
    #[instrument(skip(self), level = "debug")]
    pub async fn move_file(&self, src: &Path, dest: &Path) -> Result<()> {
        self.ensure_parent_dir(dest).await?;

        match fs::rename(src, dest).await {
            Ok(_) => {
                debug!(?src, ?dest, "File moved with rename");
                Ok(())
            }
            Err(e) if Self::is_cross_device_error(&e) => {
                tracing::debug!(?src, ?dest, "Cross-device move, falling back to copy+delete");
                self.copy_file(src, dest).await?;
                fs::remove_file(src).await?;
                Ok(())
            }
            Err(e) => Err(e.into()),
        }
    }

    /// Copy file efficiently (platform-specific fast path, buffered fallback)
    #[instrument(skip(self), level = "debug")]
    pub async fn copy_file(&self, src: &Path, dest: &Path) -> Result<()> {
        self.ensure_parent_dir(dest).await?;
        
        #[cfg(target_os = "linux")]
        {
            if let Err(e) = self.copy_file_unix(src, dest).await {
                debug!(error = ?e, "sendfile failed, falling back to buffered copy");
                self.buffered_copy(src, dest).await
            } else {
                Ok(())
            }
        }

        #[cfg(target_os = "macos")]
        {
            // macOS doesn’t support rustix::sendfile
            self.buffered_copy(src, dest).await
        }

        #[cfg(windows)]
        {
            if let Err(e) = self.copy_file_windows(src, dest).await {
                debug!(error = ?e, "CopyFileExW failed, falling back to buffered copy");
                self.buffered_copy(src, dest).await
            } else {
                Ok(())
            }
        }

        // Fallback for other platforms (e.g., WASM, embedded)
        #[cfg(not(any(target_os = "linux", target_os = "macos", windows)))]
        {
            self.buffered_copy(src, dest).await
        }
    }

    /// Buffered async copy fallback
    async fn buffered_copy(&self, src: &Path, dest: &Path) -> Result<()> {
        let mut src_file = fs::File::open(src).await?;
        let mut dest_file = fs::File::create(dest).await?;
        tokio::io::copy(&mut src_file, &mut dest_file).await?;
        dest_file.flush().await?;

        let metadata = fs::metadata(src).await?;
        fs::set_permissions(dest, metadata.permissions()).await?;
        Ok(())
    }

    /// Get file size
    #[instrument(skip(self), level = "debug")]
    pub async fn get_file_size(&self, path: &Path) -> Result<u64> {
        let metadata = fs::metadata(path).await?;
        Ok(metadata.len())
    }

    // ----------- Platform-specific fast paths -----------

    /// Unix: use rustix::fs::sendfile
    #[cfg(target_os = "linux")]
    async fn copy_file_unix(&self, src: &Path, dest: &Path) -> Result<()> {
        use std::os::fd::AsFd;
        use rustix::fs::sendfile;

        // Take ownership of paths (so they can move into the 'static closure)
        let src = src.to_path_buf();
        let dest = dest.to_path_buf();

        let src_file = fs::File::open(&src).await?;
        let dest_file = fs::File::create(&dest).await?;

        let std_src = src_file.into_std().await;
        let std_dest = dest_file.into_std().await;

        task::spawn_blocking(move || {
            let len = std::fs::metadata(&src)?.len();
            let mut offset: u64 = 0;
            let mut remaining = len;

            while remaining > 0 {
                let written = sendfile(
                    std_dest.as_fd(),
                    std_src.as_fd(),
                    Some(&mut offset),
                    remaining as usize,
                )?;
                if written == 0 {
                    if remaining > 0 {
                        return Err(std::io::Error::new(
                            std::io::ErrorKind::UnexpectedEof,
                            "sendfile returned 0 before copying all data"
                        ));
                    }
                    break;
                }
                remaining -= written as u64;
            }
            Ok::<_, std::io::Error>(())
        })
        .await??; // join + IO error propagation

        Ok(())
    }


    /// Windows: use CopyFileExW
    #[cfg(windows)]
    async fn copy_file_windows(&self, src: &Path, dest: &Path) -> Result<()> {
        use std::os::windows::ffi::OsStrExt;
        use windows_sys::Win32::Storage::FileSystem::CopyFileExW;

        let src_w: Vec<u16> = src.as_os_str().encode_wide().chain(Some(0)).collect();
        let dest_w: Vec<u16> = dest.as_os_str().encode_wide().chain(Some(0)).collect();

        task::spawn_blocking(move || unsafe {
            let success = CopyFileExW(
                src_w.as_ptr(),
                dest_w.as_ptr(),
                None,
                std::ptr::null_mut(),
                std::ptr::null_mut(),
                0,
            );
            if success == 0 {
                Err(std::io::Error::last_os_error())
            } else {
                Ok(())
            }
        })
        .await??;

        debug!(?src, ?dest, "Copied with CopyFileExW");
        Ok(())
    }
}

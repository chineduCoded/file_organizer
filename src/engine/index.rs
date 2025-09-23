use std::{path::{Path, PathBuf}, sync::Arc, time::SystemTime};

use chrono::{DateTime, Local};
use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite, Transaction};
use tokio::{fs, sync::Semaphore};

use crate::{errors::{FileOrganizerError, Result}, scanner::RawFileMetadata, utils::{from_unix, to_unix}};


#[derive(Clone)]
pub struct Db {
    pool: Pool<Sqlite>,
    write_limit: Arc<Semaphore>,
}

impl Db {
    pub async fn new(db_path: &Path) -> Result<Self> {
        println!("DB path: {:?}", db_path);

        // Ensure parent directory exists for file-based DBs
        if db_path.to_string_lossy() != ":memory:" {
            if let Some(parent) = db_path.parent() {
                tokio::fs::create_dir_all(parent).await.map_err(|e| {
                    FileOrganizerError::Io(std::io::Error::new(
                        std::io::ErrorKind::PermissionDenied,
                        format!("Failed to create database directory {:?}: {}", parent, e),
                    ))
                })?;

                #[cfg(unix)]
                {
                    use std::os::unix::fs::PermissionsExt;
                    if let Ok(metadata) = fs::metadata(parent).await {
                        let mut perms = metadata.permissions();
                        if (perms.mode() & 0o700) != 0o700 {
                            perms.set_mode(perms.mode() | 0o700);
                            if let Err(e) = fs::set_permissions(parent, perms).await {
                                tracing::warn!("Failed to set permissions on {:?}: {}", parent, e);
                            }
                        }
                    } 
                }
            }
        }

        let url = if db_path.to_string_lossy() == ":memory:" {
            "sqlite::memory:".to_string()
        } else {
            format!("sqlite://{}", db_path.display())
        };
        let pool = SqlitePoolOptions::new()
            .max_connections(5)
            .acquire_timeout(std::time::Duration::from_secs(10))
            .connect(&url)
            .await
            .map_err(|e| {
                FileOrganizerError::Io(std::io::Error::new(
                    std::io::ErrorKind::ConnectionRefused,
                    format!("Failed to connect to database at {:?}: {}", db_path, e),
                ))
            })?;

        // --- Pragmas recommended for concurrent access ---
        sqlx::query("PRAGMA journal_mode=WAL;").execute(&pool).await?;
        sqlx::query("PRAGMA synchronous=NORMAL;").execute(&pool).await?;
        sqlx::query("PRAGMA foreign_keys=ON;").execute(&pool).await?;
        sqlx::query("PRAGMA temp_store=MEMORY;").execute(&pool).await?;
        sqlx::query("PRAGMA mmap_size=30000000000;").execute(&pool).await?;
        sqlx::query("PRAGMA busy_timeout=5000;").execute(&pool).await?;

        // Add auto-checkpointing every ~1000 pages (~4MB with default 4KB page size)
        sqlx::query("PRAGMA wal_autocheckpoint=1000;").execute(&pool).await?;

        sqlx::query(
            r#"
            CREATE TABLE IF NOT EXISTS files (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                path TEXT NOT NULL UNIQUE,
                size INTEGER NOT NULL,
                created INTEGER,
                modified INTEGER,
                accessed INTEGER,
                hash TEXT,
                category TEXT,
                dest_path TEXT NOT NULL,
                updated_at INTEGER NOT NULL DEFAULT (strftime('%s','now'))
            );
            "#,
        )
        .execute(&pool)
        .await?;

        sqlx::query(
            r#"CREATE INDEX IF NOT EXISTS idx_files_updated_at ON files(updated_at);"#,
        )
        .execute(&pool)
        .await?;

        Ok(Self { 
            pool,
            write_limit: Arc::new(Semaphore::new(1)),
        })
    }

    /// Begin a transaction
    pub async fn begin(&self) -> Result<Transaction<'_, Sqlite>> {
        Ok(self.pool.begin().await?)
    }

    /// Acquire a write permit from the semaphore
    async fn acquire_write_permit(&self) -> Result<tokio::sync::OwnedSemaphorePermit> {
        Ok(self.write_limit.clone().acquire_owned().await?)
    }

    /// Insert/update a single file record (delegates to batch method).
    pub async fn update_file(
        &self,
        meta: &RawFileMetadata,
        category: &str,
        dest: &Path,
        hash: &str,
    ) -> Result<()> {
        self.update_files_batch(&[(meta.clone(), category.to_string(), dest.to_path_buf(), hash.to_string())]).await
    }


    pub async fn update_files_batch(
        &self,
        entries: &[(RawFileMetadata, String, std::path::PathBuf, String)],
    ) -> Result<()> {
        if entries.is_empty() {
            return Ok(());
        }

        let _permit = self.acquire_write_permit().await?;

        // Each row uses 8 bind params (path, size, created, modified, accessed, category, dest_path, hash)
        const PARAMS_PER_ROW: usize = 8;
        const MAX_BIND_VARS: usize = 999; // default SQLite limit
        let max_rows_per_chunk = MAX_BIND_VARS / PARAMS_PER_ROW;
        let chunk_size = std::cmp::min(max_rows_per_chunk, 100); // cap to 100 for safety

        for chunk in entries.chunks(chunk_size) {
            // Build SQL with N "(?,...,strftime('%s','now'))" groups
            let mut sql = String::from(
                "INSERT INTO files (path, size, created, modified, accessed, category, dest_path, hash, updated_at) VALUES ",
            );
            let mut binds: Vec<(
                String,
                i64,
                Option<i64>,
                Option<i64>,
                Option<i64>,
                String,
                String,
                String,
            )> = Vec::with_capacity(chunk.len());

            for (i, (meta, category, dest, hash)) in chunk.iter().enumerate() {
                if i > 0 {
                    sql.push_str(", ");
                }
                sql.push_str("(?, ?, ?, ?, ?, ?, ?, ?, strftime('%s','now'))");

                let created = to_unix(meta.created);
                let modified = to_unix(meta.modified);
                let accessed = to_unix(meta.accessed);

                binds.push((
                    meta.path.to_string_lossy().to_string(),
                    meta.size as i64,
                    created,
                    modified,
                    accessed,
                    category.clone(),
                    dest.to_string_lossy().to_string(),
                    hash.clone(),
                ));
            }

            sql.push_str(
                " ON CONFLICT(path) DO UPDATE SET
                    size=excluded.size,
                    created=excluded.created,
                    modified=excluded.modified,
                    accessed=excluded.accessed,
                    category=excluded.category,
                    dest_path=excluded.dest_path,
                    hash=excluded.hash,
                    updated_at=strftime('%s','now');",
            );

            // Bind all params in order
            let mut q = sqlx::query(&sql);
            for (path, size, created, modified, accessed, category, dest_path, hash) in binds {
                q = q
                    .bind(path)
                    .bind(size)
                    .bind(created) 
                    .bind(modified)  
                    .bind(accessed)
                    .bind(category)
                    .bind(dest_path)
                    .bind(hash);
            }

            // Execute chunk in a transaction
            let mut tx = self.pool.begin().await?;
            q.execute(&mut *tx).await?;
            tx.commit().await?;
        }

        Ok(())
    }

    pub async fn lookup(&self, path: &Path) -> Result<Option<RawFileMetadata>> {
        let row = sqlx::query(
            "SELECT size, created, modified, accessed FROM files WHERE path = ?",
        )
        .bind(path.to_string_lossy().to_string())
        .fetch_optional(&self.pool)
        .await?;

        if let Some(r) = row {
            let created  = from_unix(r.try_get::<Option<i64>, _>("created")?);
            let modified = from_unix(r.try_get::<Option<i64>, _>("modified")?);
            let accessed = from_unix(r.try_get::<Option<i64>, _>("accessed")?);

            let fs_meta = tokio::fs::symlink_metadata(path).await?;
            let ft = fs_meta.file_type();

            Ok(Some(RawFileMetadata {
                path: path.to_path_buf(),
                size: r.try_get::<i64, _>("size")? as u64,
                created,
                modified,
                accessed,
                permissions: fs_meta.permissions(),
                is_file: ft.is_file(),
                is_dir: ft.is_dir(),
                is_symlink: ft.is_symlink(),
            }))
        } else {
            Ok(None)
        }
    }


    /// Convert a sqlx::Row into a DbFileEntry
    fn row_to_entry(row: &sqlx::sqlite::SqliteRow) -> Result<DbFileEntry> {
        use sqlx::Row;
        let modified: Option<i64> = row.try_get("modified")?;
        let path: String = row.try_get("path")?;
        let dest_path: String = row.try_get("dest_path")?;

        Ok(DbFileEntry {
            path: PathBuf::from(path),
            size: row.try_get::<i64, _>("size")? as u64,
            modified: from_unix(modified),
            hash: row.try_get("hash")?,
            category: row.try_get("category")?,
            dest_path: PathBuf::from(dest_path),
        })
    }

    /// Lookup single file entry by original path
    pub async fn lookup_full(&self, path: &Path) -> Result<Option<DbFileEntry>> {
        let row = sqlx::query(
            r#"
            SELECT path, size, modified, hash, category, dest_path
            FROM files
            WHERE path = ?
            "#,
        )
        .bind(path.to_string_lossy().to_string())
        .fetch_optional(&self.pool)
        .await?;

        Ok(row.map(|r| Self::row_to_entry(&r)).transpose()?)
    }

    /// Get all file entries
    pub async fn get_all_files(&self) -> Result<Vec<DbFileEntry>> {
        let rows = sqlx::query(
            r#"
            SELECT path, size, modified, hash, category, dest_path
            FROM files
            ORDER BY updated_at DESC
            "#,
        )
        .fetch_all(&self.pool)
        .await?;

        rows.iter()
            .map(|r| Self::row_to_entry(r))
            .collect::<Result<Vec<_>>>()
    }

    /// Update a file entry in the database (non-transactional).
    pub async fn update_file_entry(&self, entry: &DbFileEntry) -> Result<()> {
        let _permit = self.acquire_write_permit().await?;
        
        sqlx::query(
            r#"
            INSERT INTO files (path, size, modified, category, dest_path, hash, updated_at)
            VALUES (?1, ?2, ?3, ?4, ?5, ?6, strftime('%s','now'))
            ON CONFLICT(path) DO UPDATE SET
                size=excluded.size,
                modified=excluded.modified,
                category=excluded.category,
                dest_path=excluded.dest_path,
                hash=excluded.hash,
                updated_at=strftime('%s','now');
            "#
        )
        .bind(entry.path.to_string_lossy().to_string())
        .bind(entry.size as i64)
        .bind(to_unix(entry.modified))
        .bind(&entry.category)
        .bind(entry.dest_path.to_string_lossy().to_string())
        .bind(&entry.hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    /// Update only dest_path + updated_at for an entry inside a transaction
    pub async fn update_dest_path_tx<'a>(
        &self,
        tx: &mut sqlx::Transaction<'a, Sqlite>,
        path: &std::path::Path,
        new_dest: &std::path::Path,
    ) -> Result<()> {
        sqlx::query(
            r#"
            UPDATE files
            SET dest_path = ?1,
                updated_at = strftime('%s','now')
            WHERE path = ?2
            "#
        )
        .bind(new_dest.to_string_lossy().to_string())
        .bind(path.to_string_lossy().to_string())
        .execute(&mut **tx)
        .await?;

        Ok(())
    }

    /// Print database information (file path, size, counts).
    pub async fn status(db_path: &Path) -> Result<()> {
        if !fs::try_exists(db_path).await? {
            println!("❌ Database does not exist at {:?}", db_path);
            return Ok(());
        }

        // Get file metadata
        let metadata = fs::metadata(db_path).await?;
        let size_kb = metadata.len() as f64 / 1024.0;

        // Last modified
        let modified_time = metadata.modified()?;
        let datetime: DateTime<Local> = modified_time.into();
        let modified_str = datetime.format("%Y-%m-%d %H:%M:%S").to_string();

        // Connect temporarily to query counts
        let db = Db::new(db_path).await?;
        let files_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM files;")
            .fetch_one(&db.pool)
            .await
            .unwrap_or((0,));
        let actions_count: (i64,) = sqlx::query_as("SELECT COUNT(*) FROM actions;")
            .fetch_one(&db.pool)
            .await
            .unwrap_or((0,));

        println!("📂 Database path : {:?}", db_path);
        println!("📏 File size     : {:.2} KB", size_kb);
        println!("🕒 Last modified   : {}", modified_str);
        println!("📊 Files tracked : {}", files_count.0);
        println!("📊 Actions saved : {}", actions_count.0);

        Ok(())
    }

    /// Run VACUUM + ANALYZE to optimize.
    pub async fn vacuum(&self) -> Result<()> {
        sqlx::query("VACUUM;").execute(&self.pool).await?;
        sqlx::query("ANALYZE;").execute(&self.pool).await?;
        Ok(())
    }

    pub async fn save(&self) -> Result<()> {
        let _permit = self.acquire_write_permit().await;

        // No more forced checkpoint → SQLite handles it incrementally
        sqlx::query("ANALYZE;")
        .execute(&self.pool)
        .await?;

        Ok(())
    }
}

#[derive(Debug, Clone, Default)]
pub struct DbFileEntry {
    pub path: PathBuf,
    pub size: u64,
    pub modified: Option<SystemTime>,
    pub hash: Option<String>,
    pub category: Option<String>,
    pub dest_path: PathBuf,
}
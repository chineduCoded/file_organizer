use std::{path::{Path, PathBuf}, sync::Arc, time::SystemTime};

use sqlx::{sqlite::SqlitePoolOptions, Pool, Row, Sqlite, Transaction};
use tokio::sync::Semaphore;

use crate::{errors::Result, scanner::RawFileMetadata, utils::{from_unix, to_unix}};


#[derive(Clone)]
pub struct Db {
    pool: Pool<Sqlite>,
    write_limit: Arc<Semaphore>,
}

impl Db {
    pub async fn new(db_path: &Path) -> Result<Self> {
        if let Some(parent) = db_path.parent() {
            tokio::fs::create_dir_all(parent).await?;
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
            .await?;

        // --- Pragmas recommended for concurrent access ---
        sqlx::query("PRAGMA journal_mode=WAL;").execute(&pool).await?;
        sqlx::query("PRAGMA synchronous=NORMAL;").execute(&pool).await?;
        sqlx::query("PRAGMA foreign_keys=ON;").execute(&pool).await?;
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

    pub async fn update_file(
        &self,
        meta: &RawFileMetadata,
        category: &str,
        dest: &Path,
        hash: &str,
    ) -> Result<()> {
        let _permit = self.acquire_write_permit().await?;

        let modified = to_unix(meta.modified);
        let created = to_unix(meta.created);
        let accessed = to_unix(meta.accessed);

        sqlx::query(
            r#"
            INSERT INTO files (path, size, created, modified, accessed, category, dest_path, hash, updated_at)
            VALUES (?, ?, ?, ?, ?, ?, ?, ?, strftime('%s','now'))
            ON CONFLICT(path) DO UPDATE SET
                size=excluded.size,
                created=excluded.created,
                modified=excluded.modified,
                accessed=excluded.accessed,
                category=excluded.category,
                dest_path=excluded.dest_path,
                hash=excluded.hash,
                updated_at=strftime('%s','now');
            "#,
        )
        .bind(meta.path.to_string_lossy().to_string())
        .bind(meta.size as i64)
        .bind(created)
        .bind(modified)
        .bind(accessed)
        .bind(category)
        .bind(dest.to_string_lossy().to_string())
        .bind(hash)
        .execute(&self.pool)
        .await?;

        Ok(())
    }

    pub async fn update_files_batch(
        &self,
        entries: &[(RawFileMetadata, String, std::path::PathBuf, String)],
    ) -> Result<()> {
        let _permit = self.acquire_write_permit().await?;

        let mut tx = self.pool.begin().await?;

        for (meta, category, dest, hash) in entries {
            let modified = to_unix(meta.modified);
            let created  = to_unix(meta.created);
            let accessed = to_unix(meta.accessed);

            sqlx::query(
                r#"
                INSERT INTO files (path, size, created, modified, accessed, category, dest_path, hash, updated_at)
                VALUES (?, ?, ?, ?, ?, ?, ?, ?, strftime('%s','now'))
                ON CONFLICT(path) DO UPDATE SET
                    size=excluded.size,
                    created=excluded.created,
                    modified=excluded.modified,
                    accessed=excluded.accessed,
                    category=excluded.category,
                    dest_path=excluded.dest_path,
                    hash=excluded.hash,
                    updated_at=strftime('%s','now');
                "#,
            )
            .bind(meta.path.to_string_lossy().to_string())
            .bind(meta.size as i64)
            .bind(created)
            .bind(modified)
            .bind(accessed)
            .bind(category)
            .bind(dest.to_string_lossy().to_string())
            .bind(hash)
            .execute(&mut *tx)
            .await?;
        }
        tx.commit().await?;

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
        let _permit = self.acquire_write_permit().await;
        
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

    /// Run periodic maintenance: vacuum + analyze
    pub async fn maintenance(&self) -> Result<()> {
        // Rebuild database file to reclaim space
        sqlx::query("VACUUM;").execute(&self.pool).await?;

        // Update statistics so query planner knows to use indexes
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
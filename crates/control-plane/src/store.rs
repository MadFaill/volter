//! Хранилище учётной записи администратора.
//!
//! Трейт `UserStore` (async) имеет три реализации:
//! - [`PgUserStore`] — Postgres (Ш1, основная для деплоя);
//! - [`FileUserStore`] — файловая (dev/фаза 1 без БД);
//! - [`MemoryUserStore`] — для тестов.

use async_trait::async_trait;
use serde::{Deserialize, Serialize};
use sqlx::{Pool, Postgres, Row};
use std::path::PathBuf;
use std::sync::Mutex;

/// Запись администратора.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminRecord {
    pub username: String,
    pub password_hash: String,
}

/// Абстракция хранилища учётки.
#[async_trait]
pub trait UserStore: Send + Sync {
    async fn admin(&self) -> anyhow::Result<Option<AdminRecord>>;
    async fn set_admin(&self, record: AdminRecord) -> anyhow::Result<()>;
}

/// Postgres-хранилище.
pub struct PgUserStore {
    pool: Pool<Postgres>,
}

impl PgUserStore {
    pub fn new(pool: Pool<Postgres>) -> Self {
        Self { pool }
    }
}

#[async_trait]
impl UserStore for PgUserStore {
    async fn admin(&self) -> anyhow::Result<Option<AdminRecord>> {
        let row =
            sqlx::query("SELECT username, password_hash FROM app_user ORDER BY created_at LIMIT 1")
                .fetch_optional(&self.pool)
                .await?;
        Ok(row.map(|r| AdminRecord {
            username: r.get("username"),
            password_hash: r.get("password_hash"),
        }))
    }

    async fn set_admin(&self, record: AdminRecord) -> anyhow::Result<()> {
        sqlx::query("INSERT INTO app_user (username, password_hash) VALUES ($1, $2)")
            .bind(&record.username)
            .bind(&record.password_hash)
            .execute(&self.pool)
            .await?;
        Ok(())
    }
}

/// Файловое хранилище: один JSON-файл с записью администратора.
pub struct FileUserStore {
    path: PathBuf,
    lock: Mutex<()>,
}

impl FileUserStore {
    pub fn new(path: impl Into<PathBuf>) -> Self {
        Self {
            path: path.into(),
            lock: Mutex::new(()),
        }
    }
}

#[async_trait]
impl UserStore for FileUserStore {
    async fn admin(&self) -> anyhow::Result<Option<AdminRecord>> {
        let _guard = self.lock.lock().unwrap();
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    async fn set_admin(&self, record: AdminRecord) -> anyhow::Result<()> {
        let _guard = self.lock.lock().unwrap();
        if let Some(parent) = self.path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let tmp = self.path.with_extension("json.tmp");
        std::fs::write(&tmp, serde_json::to_vec_pretty(&record)?)?;
        std::fs::rename(&tmp, &self.path)?;
        Ok(())
    }
}

/// In-memory хранилище для тестов.
#[derive(Default)]
pub struct MemoryUserStore {
    inner: Mutex<Option<AdminRecord>>,
}

#[async_trait]
impl UserStore for MemoryUserStore {
    async fn admin(&self) -> anyhow::Result<Option<AdminRecord>> {
        Ok(self.inner.lock().unwrap().clone())
    }

    async fn set_admin(&self, record: AdminRecord) -> anyhow::Result<()> {
        *self.inner.lock().unwrap() = Some(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn memory_store_roundtrip() {
        let store = MemoryUserStore::default();
        assert!(store.admin().await.unwrap().is_none());
        let rec = AdminRecord {
            username: "admin".into(),
            password_hash: "phc".into(),
        };
        store.set_admin(rec.clone()).await.unwrap();
        assert_eq!(store.admin().await.unwrap(), Some(rec));
    }

    #[tokio::test]
    async fn file_store_persists_to_disk() {
        let dir = std::env::temp_dir().join(format!("volter-test-{}", std::process::id()));
        let path = dir.join("admin.json");
        let _ = std::fs::remove_file(&path);
        let store = FileUserStore::new(&path);
        assert!(store.admin().await.unwrap().is_none());
        let rec = AdminRecord {
            username: "admin".into(),
            password_hash: "phc".into(),
        };
        store.set_admin(rec.clone()).await.unwrap();
        let store2 = FileUserStore::new(&path);
        assert_eq!(store2.admin().await.unwrap(), Some(rec));
        let _ = std::fs::remove_file(&path);
    }
}

//! Хранилище учётной записи администратора.
//!
//! Ш0а: single-admin, persistence — файловая (JSON). Трейт `UserStore` позволит в Ш1
//! заменить реализацию на Postgres без правок хендлеров.

use serde::{Deserialize, Serialize};
use std::path::PathBuf;
use std::sync::Mutex;

/// Запись администратора.
#[derive(Debug, Clone, Serialize, Deserialize, PartialEq, Eq)]
pub struct AdminRecord {
    pub username: String,
    pub password_hash: String,
}

/// Абстракция хранилища учётки (в Ш1 — Postgres).
pub trait UserStore: Send + Sync {
    fn admin(&self) -> anyhow::Result<Option<AdminRecord>>;
    fn set_admin(&self, record: AdminRecord) -> anyhow::Result<()>;
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

impl UserStore for FileUserStore {
    fn admin(&self) -> anyhow::Result<Option<AdminRecord>> {
        let _guard = self.lock.lock().unwrap();
        match std::fs::read(&self.path) {
            Ok(bytes) => Ok(Some(serde_json::from_slice(&bytes)?)),
            Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(None),
            Err(e) => Err(e.into()),
        }
    }

    fn set_admin(&self, record: AdminRecord) -> anyhow::Result<()> {
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

impl UserStore for MemoryUserStore {
    fn admin(&self) -> anyhow::Result<Option<AdminRecord>> {
        Ok(self.inner.lock().unwrap().clone())
    }

    fn set_admin(&self, record: AdminRecord) -> anyhow::Result<()> {
        *self.inner.lock().unwrap() = Some(record);
        Ok(())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn memory_store_roundtrip() {
        let store = MemoryUserStore::default();
        assert!(store.admin().unwrap().is_none());
        let rec = AdminRecord {
            username: "admin".into(),
            password_hash: "phc".into(),
        };
        store.set_admin(rec.clone()).unwrap();
        assert_eq!(store.admin().unwrap(), Some(rec));
    }

    #[test]
    fn file_store_persists_to_disk() {
        let dir = std::env::temp_dir().join(format!("volter-test-{}", std::process::id()));
        let path = dir.join("admin.json");
        let _ = std::fs::remove_file(&path);
        let store = FileUserStore::new(&path);
        assert!(store.admin().unwrap().is_none());
        let rec = AdminRecord {
            username: "admin".into(),
            password_hash: "phc".into(),
        };
        store.set_admin(rec.clone()).unwrap();
        // новый инстанс читает с диска
        let store2 = FileUserStore::new(&path);
        assert_eq!(store2.admin().unwrap(), Some(rec));
        let _ = std::fs::remove_file(&path);
    }
}

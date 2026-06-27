//! Точка входа control-plane (`volter-api`).
//!
//! Конфигурация через env:
//! - `VOLTER_BIND`       адрес прослушивания (по умолчанию `0.0.0.0:8080`)
//! - `VOLTER_DATA_DIR`   каталог состояния (по умолчанию `./data`)
//! - `VOLTER_JWT_SECRET` секрет подписи сессий (если пуст — генерируется и сохраняется)

use std::path::PathBuf;
use std::sync::Arc;

use argon2::password_hash::rand_core::{OsRng, RngCore};
use volter_control_plane::store::{FileUserStore, PgUserStore, UserStore};
use volter_control_plane::{build_router, AppState};

#[tokio::main]
async fn main() -> anyhow::Result<()> {
    tracing_subscriber::fmt()
        .with_env_filter(
            tracing_subscriber::EnvFilter::try_from_default_env().unwrap_or_else(|_| "info".into()),
        )
        .init();

    let data_dir: PathBuf = std::env::var("VOLTER_DATA_DIR")
        .unwrap_or_else(|_| "./data".into())
        .into();
    std::fs::create_dir_all(&data_dir)?;

    let jwt_secret = load_or_create_secret(&data_dir)?;

    // Postgres (Ш1) при наличии DATABASE_URL, иначе файловый стор (dev/фаза 1 без БД).
    let store: Arc<dyn UserStore> = match std::env::var("DATABASE_URL") {
        Ok(db) if !db.is_empty() => {
            let pool = sqlx::postgres::PgPoolOptions::new()
                .max_connections(5)
                .connect(&db)
                .await?;
            sqlx::migrate!("./migrations").run(&pool).await?;
            tracing::info!("store: postgres (миграции применены)");
            Arc::new(PgUserStore::new(pool))
        }
        _ => {
            tracing::info!("store: file ({}/admin.json)", data_dir.display());
            Arc::new(FileUserStore::new(data_dir.join("admin.json")))
        }
    };
    let state = AppState::new(store, jwt_secret);

    let bind = std::env::var("VOLTER_BIND").unwrap_or_else(|_| "0.0.0.0:8080".into());
    let listener = tokio::net::TcpListener::bind(&bind).await?;
    tracing::info!(%bind, "volter control-plane listening");

    axum::serve(listener, build_router(state))
        .with_graceful_shutdown(shutdown_signal())
        .await?;
    Ok(())
}

/// Читает секрет из env или файла, иначе генерирует 32 случайных байта и сохраняет.
fn load_or_create_secret(data_dir: &std::path::Path) -> anyhow::Result<Vec<u8>> {
    if let Ok(s) = std::env::var("VOLTER_JWT_SECRET") {
        if !s.is_empty() {
            return Ok(s.into_bytes());
        }
    }
    let path = data_dir.join("jwt.secret");
    match std::fs::read(&path) {
        Ok(bytes) if !bytes.is_empty() => Ok(bytes),
        _ => {
            let mut secret = vec![0u8; 32];
            OsRng.fill_bytes(&mut secret);
            std::fs::write(&path, &secret)?;
            Ok(secret)
        }
    }
}

async fn shutdown_signal() {
    let _ = tokio::signal::ctrl_c().await;
    tracing::info!("shutdown signal received");
}

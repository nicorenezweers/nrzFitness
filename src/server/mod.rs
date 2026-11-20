//! Server-only: Datenbank-Pool, Fehler, Open Food Facts.

pub mod ai;
pub mod db;
pub mod off;

use std::sync::OnceLock;

use sqlx::SqlitePool;

static POOL: OnceLock<SqlitePool> = OnceLock::new();

pub fn set_pool(pool: SqlitePool) {
    let _ = POOL.set(pool);
}

pub fn pool() -> &'static SqlitePool {
    POOL.get().expect("Datenbank-Pool nicht initialisiert")
}

#[derive(Debug)]
pub enum AppError {
    NotFound(String),
    BadRequest(String),
    Db(sqlx::Error),
    Http(String),
}

impl std::fmt::Display for AppError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            AppError::NotFound(m) | AppError::BadRequest(m) | AppError::Http(m) => write!(f, "{m}"),
            AppError::Db(e) => write!(f, "Datenbankfehler: {e}"),
        }
    }
}

impl std::error::Error for AppError {}

impl From<sqlx::Error> for AppError {
    fn from(e: sqlx::Error) -> Self {
        AppError::Db(e)
    }
}

pub type AppResult<T> = Result<T, AppError>;

/// Verbindet die SQLite-Datei (legt sie an), führt Migrationen aus und legt Startdaten an.
pub async fn init_db() -> SqlitePool {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqliteJournalMode, SqlitePoolOptions};

    let url = std::env::var("DATABASE_URL").unwrap_or_else(|_| "sqlite://data/nrzfitness.db".into());
    if let Some(path) = url.strip_prefix("sqlite://").or_else(|| url.strip_prefix("sqlite:"))
        && let Some(dir) = std::path::Path::new(path.split('?').next().unwrap_or(path)).parent()
        && !dir.as_os_str().is_empty()
    {
        let _ = tokio::fs::create_dir_all(dir).await;
    }
    let opts = SqliteConnectOptions::from_str(&url)
        .expect("DATABASE_URL ungültig")
        .create_if_missing(true)
        .foreign_keys(true)
        .journal_mode(SqliteJournalMode::Wal);
    let pool = SqlitePoolOptions::new().max_connections(5).connect_with(opts).await.expect("Datenbank nicht erreichbar");
    sqlx::migrate!("./migrations").run(&pool).await.expect("Migrationen fehlgeschlagen");
    if let Err(e) = db::seed::run(&pool).await {
        tracing::warn!("Startdaten: {e}");
    }
    pool
}

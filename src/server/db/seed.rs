use sqlx::SqlitePool;

/// Startdaten (Übungsbibliothek usw.). Muss idempotent sein (läuft bei jedem Start).
pub async fn run(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    super::training::seed_exercises(pool).await?;
    Ok(())
}

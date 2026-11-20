use sqlx::SqlitePool;

/// Startdaten (Basis-Lebensmittel, Übungsbibliothek). Muss idempotent sein (läuft bei jedem Start).
pub async fn run(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    super::foods_seed::seed(pool).await?;
    super::training::seed_exercises(pool).await?;
    Ok(())
}

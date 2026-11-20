use sqlx::SqlitePool;

/// Legt die mitgelieferte Übungsbibliothek an (idempotent).
pub async fn seed_exercises(_pool: &SqlitePool) -> Result<(), sqlx::Error> {
    Ok(())
}

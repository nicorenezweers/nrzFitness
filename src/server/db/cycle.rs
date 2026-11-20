//! Zyklus: Periodentage.

use chrono::NaiveDate;
use sqlx::SqlitePool;

use crate::model::CycleDay;
use crate::server::AppResult;

pub async fn days(pool: &SqlitePool, pid: &str) -> AppResult<Vec<CycleDay>> {
    Ok(sqlx::query_as::<_, CycleDay>("SELECT date, flow FROM cycle_days WHERE profile_id = ? ORDER BY date")
        .bind(pid)
        .fetch_all(pool)
        .await?)
}

/// Stärke setzen (1–3) oder Tag entfernen (`None`).
pub async fn set_day(pool: &SqlitePool, pid: &str, date: NaiveDate, flow: Option<i64>) -> AppResult<()> {
    match flow {
        Some(f) => {
            sqlx::query(
                "INSERT INTO cycle_days (profile_id, date, flow) VALUES (?, ?, ?)
                 ON CONFLICT(profile_id, date) DO UPDATE SET flow = excluded.flow",
            )
            .bind(pid)
            .bind(date)
            .bind(f.clamp(1, 3))
            .execute(pool)
            .await?;
        }
        None => {
            sqlx::query("DELETE FROM cycle_days WHERE profile_id = ? AND date = ?")
                .bind(pid)
                .bind(date)
                .execute(pool)
                .await?;
        }
    }
    Ok(())
}

/// Mehrere Tage auf einmal eintragen (z.B. „Periode begonnen“ mit erwarteter Dauer); vorhandene bleiben.
pub async fn add_range(pool: &SqlitePool, pid: &str, start: NaiveDate, days: i64, flow: i64) -> AppResult<()> {
    let mut tx = pool.begin().await?;
    for i in 0..days.clamp(1, 10) {
        sqlx::query("INSERT OR IGNORE INTO cycle_days (profile_id, date, flow) VALUES (?, ?, ?)")
            .bind(pid)
            .bind(start + chrono::Duration::days(i))
            .bind(flow.clamp(1, 3))
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn cycle_days_roundtrip() {
        let (pool, pid) = super::super::body::test_pool().await;
        let d = NaiveDate::from_ymd_opt(2026, 11, 1).expect("date");
        add_range(&pool, &pid, d, 5, 2).await.expect("range");
        set_day(&pool, &pid, d, Some(3)).await.expect("set");
        set_day(&pool, &pid, d + chrono::Duration::days(4), None).await.expect("remove");
        let all = days(&pool, &pid).await.expect("days");
        assert_eq!(all.len(), 4);
        assert_eq!(all[0].flow, 3);
    }
}

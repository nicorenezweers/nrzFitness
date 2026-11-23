//! Check-ins (Körper): Gewicht, Umfänge, Schritte, Stimmung, Schlaf.

use chrono::NaiveDate;
use sqlx::SqlitePool;

use super::clean;
use crate::model::{BodyOverview, Checkin, WeightPoint};
use crate::server::{AppError, AppResult};

const COLS: &str =
    "date, weight, body_fat, waist, hips, chest, neck, arm, thigh, steps, mood, energy, stress, sleep_hours,
                    sleep_quality, resting_hr, notes";

pub async fn get(pool: &SqlitePool, pid: &str, date: NaiveDate) -> AppResult<Option<Checkin>> {
    Ok(sqlx::query_as::<_, Checkin>(sqlx::AssertSqlSafe(format!("SELECT {COLS} FROM checkins WHERE profile_id = ? AND date = ?")))
        .bind(pid)
        .bind(date)
        .fetch_optional(pool)
        .await?)
}

/// Positive Zahl oder NULL.
fn pos(v: Option<f64>) -> Option<f64> {
    v.filter(|x| x.is_finite() && *x > 0.0)
}

fn scale(v: Option<i64>) -> Option<i64> {
    v.filter(|x| (1..=5).contains(x))
}

/// Check-in eines Tages komplett speichern (leere Felder = NULL). Ganz leer → Eintrag löschen.
pub async fn save(pool: &SqlitePool, pid: &str, c: Checkin) -> AppResult<Option<Checkin>> {
    if c.weight.is_some_and(|w| !(20.0..=400.0).contains(&w)) {
        return Err(AppError::BadRequest("Gewicht muss zwischen 20 und 400 kg liegen".into()));
    }
    if c.body_fat.is_some_and(|w| !(1.0..=75.0).contains(&w)) {
        return Err(AppError::BadRequest("Körperfett muss zwischen 1 und 75 % liegen".into()));
    }
    if c.is_empty() {
        delete(pool, pid, c.date).await?;
        return Ok(None);
    }
    sqlx::query(
        "INSERT INTO checkins (profile_id, date, weight, body_fat, waist, hips, chest, neck, arm, thigh, steps, mood, energy,
                               stress, sleep_hours, sleep_quality, resting_hr, notes)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)
         ON CONFLICT(profile_id, date) DO UPDATE SET
            weight = excluded.weight, body_fat = excluded.body_fat, waist = excluded.waist, hips = excluded.hips,
            chest = excluded.chest, neck = excluded.neck, arm = excluded.arm, thigh = excluded.thigh, steps = excluded.steps,
            mood = excluded.mood, energy = excluded.energy, stress = excluded.stress, sleep_hours = excluded.sleep_hours,
            sleep_quality = excluded.sleep_quality, resting_hr = excluded.resting_hr, notes = excluded.notes",
    )
    .bind(pid)
    .bind(c.date)
    .bind(pos(c.weight))
    .bind(pos(c.body_fat))
    .bind(pos(c.waist))
    .bind(pos(c.hips))
    .bind(pos(c.chest))
    .bind(pos(c.neck))
    .bind(pos(c.arm))
    .bind(pos(c.thigh))
    .bind(c.steps.filter(|s| *s >= 0))
    .bind(scale(c.mood))
    .bind(scale(c.energy))
    .bind(scale(c.stress))
    .bind(c.sleep_hours.filter(|h| h.is_finite() && (0.0..=24.0).contains(h) && *h > 0.0))
    .bind(scale(c.sleep_quality))
    .bind(c.resting_hr.filter(|h| (20..=250).contains(h)))
    .bind(clean(c.notes))
    .execute(pool)
    .await?;
    get(pool, pid, c.date).await
}

/// Nur das Gewicht eines Tages setzen (Schnelleintrag); übrige Felder bleiben.
pub async fn set_weight(pool: &SqlitePool, pid: &str, date: NaiveDate, weight: f64) -> AppResult<()> {
    if !(20.0..=400.0).contains(&weight) {
        return Err(AppError::BadRequest("Gewicht muss zwischen 20 und 400 kg liegen".into()));
    }
    super::profile::set_weight(pool, pid, date, weight).await
}

pub async fn delete(pool: &SqlitePool, pid: &str, date: NaiveDate) -> AppResult<()> {
    sqlx::query("DELETE FROM checkins WHERE profile_id = ? AND date = ?").bind(pid).bind(date).execute(pool).await?;
    Ok(())
}

pub async fn overview(pool: &SqlitePool, pid: &str) -> AppResult<BodyOverview> {
    let weights = sqlx::query_as::<_, WeightPoint>(
        "SELECT date, weight FROM checkins WHERE profile_id = ? AND weight IS NOT NULL ORDER BY date",
    )
    .bind(pid)
    .fetch_all(pool)
    .await?;
    let recent = sqlx::query_as::<_, Checkin>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM checkins WHERE profile_id = ? ORDER BY date DESC LIMIT 30"
    )))
    .bind(pid)
    .fetch_all(pool)
    .await?;
    Ok(BodyOverview { weights, recent })
}

/// Test-Hilfe: In-Memory-DB mit Schema und einem Profil (auch für Fasten/Zyklus/Medikamente).
#[cfg(test)]
pub(crate) async fn test_pool() -> (SqlitePool, String) {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
    let opts = SqliteConnectOptions::from_str("sqlite::memory:").expect("url").foreign_keys(true);
    let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.expect("pool");
    crate::server::migrations::run(&pool).await.expect("migrate");
    let p = super::profile::create(
        &pool,
        crate::model::ProfileInput { name: "Test".into(), sex: "f".into(), cycle_tracking: true, ..Default::default() },
    )
    .await
    .expect("profile");
    (pool, p.id)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn checkin_roundtrip() {
        let (pool, pid) = test_pool().await;
        let d = NaiveDate::from_ymd_opt(2026, 11, 20).expect("date");
        let c = Checkin {
            date: d,
            weight: Some(78.4),
            mood: Some(4),
            steps: Some(9000),
            notes: Some(" gut ".into()),
            ..Default::default()
        };
        let saved = save(&pool, &pid, c).await.expect("save").expect("some");
        assert_eq!(saved.weight, Some(78.4));
        assert_eq!(saved.notes.as_deref(), Some("gut"));
        set_weight(&pool, &pid, d, 78.0).await.expect("weight");
        let got = get(&pool, &pid, d).await.expect("get").expect("some");
        assert_eq!((got.weight, got.mood), (Some(78.0), Some(4)));
        let ov = overview(&pool, &pid).await.expect("overview");
        assert_eq!(ov.weights.len(), 1);
        assert_eq!(ov.recent.len(), 1);
        // Leerer Check-in löscht den Tag.
        assert!(save(&pool, &pid, Checkin::empty(d)).await.expect("save").is_none());
        assert!(get(&pool, &pid, d).await.expect("get").is_none());
    }
}

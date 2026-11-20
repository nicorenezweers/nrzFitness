//! Fastenzeiten.

use chrono::NaiveDateTime;
use sqlx::SqlitePool;

use super::{clean, new_id, ts};
use crate::model::{Fast, FastingData};
use crate::server::{AppError, AppResult};

const COLS: &str = "id, start_at, end_at, target_hours, notes";

pub async fn get(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<Fast> {
    sqlx::query_as::<_, Fast>(&format!("SELECT {COLS} FROM fasts WHERE id = ? AND profile_id = ?"))
        .bind(id)
        .bind(pid)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Fastenzeit nicht gefunden".into()))
}

pub async fn active(pool: &SqlitePool, pid: &str) -> AppResult<Option<Fast>> {
    Ok(sqlx::query_as::<_, Fast>(&format!(
        "SELECT {COLS} FROM fasts WHERE profile_id = ? AND end_at IS NULL ORDER BY start_at DESC LIMIT 1"
    ))
    .bind(pid)
    .fetch_optional(pool)
    .await?)
}

pub async fn data(pool: &SqlitePool, pid: &str) -> AppResult<FastingData> {
    let active = active(pool, pid).await?;
    let history = sqlx::query_as::<_, Fast>(&format!(
        "SELECT {COLS} FROM fasts WHERE profile_id = ? AND end_at IS NOT NULL ORDER BY start_at DESC LIMIT 120"
    ))
    .bind(pid)
    .fetch_all(pool)
    .await?;
    let default_hours = super::profile::goals(pool, pid).await?.fasting_hours;
    Ok(FastingData { active, history, default_hours: if default_hours > 0.0 { default_hours } else { 16.0 } })
}

fn valid_target(h: f64) -> AppResult<f64> {
    if h.is_finite() && (1.0..=168.0).contains(&h) {
        Ok(h)
    } else {
        Err(AppError::BadRequest("Ziel muss zwischen 1 und 168 Stunden liegen".into()))
    }
}

pub async fn start(pool: &SqlitePool, pid: &str, start_at: NaiveDateTime, target_hours: f64) -> AppResult<Fast> {
    let target = valid_target(target_hours)?;
    if active(pool, pid).await?.is_some() {
        return Err(AppError::BadRequest("Es läuft bereits eine Fastenzeit".into()));
    }
    let id = new_id();
    sqlx::query("INSERT INTO fasts (id, profile_id, start_at, end_at, target_hours) VALUES (?, ?, ?, NULL, ?)")
        .bind(&id)
        .bind(pid)
        .bind(ts(start_at))
        .bind(target)
        .execute(pool)
        .await?;
    get(pool, pid, &id).await
}

pub async fn end(pool: &SqlitePool, pid: &str, id: &str, end_at: NaiveDateTime) -> AppResult<Fast> {
    let f = get(pool, pid, id).await?;
    let end_at = end_at.max(f.start_at);
    sqlx::query("UPDATE fasts SET end_at = ? WHERE id = ? AND profile_id = ?")
        .bind(ts(end_at))
        .bind(id)
        .bind(pid)
        .execute(pool)
        .await?;
    get(pool, pid, id).await
}

/// Start/Ende/Ziel/Notiz nachträglich ändern. `end_at` None = läuft noch.
pub async fn update(
    pool: &SqlitePool,
    pid: &str,
    id: &str,
    start_at: NaiveDateTime,
    end_at: Option<NaiveDateTime>,
    target_hours: f64,
    notes: Option<String>,
) -> AppResult<Fast> {
    let target = valid_target(target_hours)?;
    if end_at.is_some_and(|e| e <= start_at) {
        return Err(AppError::BadRequest("Ende muss nach dem Start liegen".into()));
    }
    get(pool, pid, id).await?;
    if end_at.is_none() && active(pool, pid).await?.is_some_and(|a| a.id != id) {
        return Err(AppError::BadRequest("Es läuft bereits eine andere Fastenzeit".into()));
    }
    sqlx::query(
        "UPDATE fasts SET start_at = ?, end_at = ?, target_hours = ?, notes = ? WHERE id = ? AND profile_id = ?",
    )
    .bind(ts(start_at))
    .bind(end_at.map(ts))
    .bind(target)
    .bind(clean(notes))
    .bind(id)
    .bind(pid)
    .execute(pool)
    .await?;
    get(pool, pid, id).await
}

pub async fn delete(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM fasts WHERE id = ? AND profile_id = ?").bind(id).bind(pid).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn fast_lifecycle() {
        let (pool, pid) = super::super::body::test_pool().await;
        let t0 = crate::util::parse_datetime("2026-11-19T20:00").expect("dt");
        let f = start(&pool, &pid, t0, 16.0).await.expect("start");
        assert!(start(&pool, &pid, t0, 16.0).await.is_err(), "nur eine aktive Fastenzeit");
        let d = data(&pool, &pid).await.expect("data");
        assert_eq!(d.active.as_ref().map(|a| a.id.clone()), Some(f.id.clone()));
        assert_eq!(d.default_hours, 16.0);
        let t1 = crate::util::parse_datetime("2026-11-20T12:30").expect("dt");
        let f = end(&pool, &pid, &f.id, t1).await.expect("end");
        assert_eq!(f.end_at, Some(t1));
        let f = update(&pool, &pid, &f.id, t0, Some(t1), 18.0, Some("ok".into())).await.expect("update");
        assert_eq!(f.target_hours, 18.0);
        let d = data(&pool, &pid).await.expect("data");
        assert!(d.active.is_none());
        assert_eq!(d.history.len(), 1);
        delete(&pool, &pid, &f.id).await.expect("delete");
        assert!(data(&pool, &pid).await.expect("data").history.is_empty());
    }
}

//! Medikamente, Einnahme-Protokoll und Symptome.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use sqlx::SqlitePool;

use super::{clean, new_id, now_ts, ts};
use crate::model::{MedLog, Medication, MedicationInput, MedsData, Symptom, SymptomsData, parse_times};
use crate::server::{AppError, AppResult};

const COLS: &str = "id, name, dose, times, active, notes, created_at";

pub async fn get(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<Medication> {
    sqlx::query_as::<_, Medication>(sqlx::AssertSqlSafe(format!("SELECT {COLS} FROM medications WHERE id = ? AND profile_id = ?")))
        .bind(id)
        .bind(pid)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Medikament nicht gefunden".into()))
}

/// Medikamente und Protokoll (letzte 30 Tage bis `today` plus `date`).
pub async fn data(pool: &SqlitePool, pid: &str, date: NaiveDate, today: NaiveDate) -> AppResult<MedsData> {
    let meds = sqlx::query_as::<_, Medication>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM medications WHERE profile_id = ? ORDER BY active DESC, name COLLATE NOCASE"
    )))
    .bind(pid)
    .fetch_all(pool)
    .await?;
    let logs = sqlx::query_as::<_, MedLog>(
        "SELECT medication_id, date, slot, taken_at FROM medication_logs
         WHERE profile_id = ? AND ((date >= ? AND date <= ?) OR date = ?)",
    )
    .bind(pid)
    .bind(today - Duration::days(29))
    .bind(today)
    .bind(date)
    .fetch_all(pool)
    .await?;
    Ok(MedsData { meds, logs })
}

fn validate(input: &MedicationInput) -> AppResult<(String, String)> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name fehlt".into()));
    }
    let times = parse_times(&input.times.join(","));
    if times.is_empty() {
        return Err(AppError::BadRequest("Mindestens eine Einnahmezeit angeben".into()));
    }
    Ok((name, times.join(",")))
}

pub async fn save(pool: &SqlitePool, pid: &str, id: Option<String>, input: MedicationInput) -> AppResult<Medication> {
    let (name, times) = validate(&input)?;
    let id = match id {
        Some(id) => {
            get(pool, pid, &id).await?;
            sqlx::query("UPDATE medications SET name = ?, dose = ?, times = ?, active = ?, notes = ? WHERE id = ? AND profile_id = ?")
                .bind(name)
                .bind(clean(input.dose))
                .bind(times)
                .bind(input.active)
                .bind(clean(input.notes))
                .bind(&id)
                .bind(pid)
                .execute(pool)
                .await?;
            id
        }
        None => {
            let id = new_id();
            sqlx::query(
                "INSERT INTO medications (id, profile_id, name, dose, times, active, notes, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(pid)
            .bind(name)
            .bind(clean(input.dose))
            .bind(times)
            .bind(input.active)
            .bind(clean(input.notes))
            .bind(now_ts())
            .execute(pool)
            .await?;
            id
        }
    };
    get(pool, pid, &id).await
}

pub async fn set_active(pool: &SqlitePool, pid: &str, id: &str, active: bool) -> AppResult<()> {
    sqlx::query("UPDATE medications SET active = ? WHERE id = ? AND profile_id = ?")
        .bind(active)
        .bind(id)
        .bind(pid)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM medications WHERE id = ? AND profile_id = ?").bind(id).bind(pid).execute(pool).await?;
    Ok(())
}

/// Einnahme umschalten: genommen ↔ nicht genommen. Gibt den neuen Zustand zurück.
pub async fn toggle_log(
    pool: &SqlitePool,
    pid: &str,
    med_id: &str,
    date: NaiveDate,
    slot: &str,
    at: NaiveDateTime,
) -> AppResult<bool> {
    get(pool, pid, med_id).await?;
    let r =
        sqlx::query("DELETE FROM medication_logs WHERE medication_id = ? AND profile_id = ? AND date = ? AND slot = ?")
            .bind(med_id)
            .bind(pid)
            .bind(date)
            .bind(slot)
            .execute(pool)
            .await?;
    if r.rows_affected() > 0 {
        return Ok(false);
    }
    sqlx::query(
        "INSERT INTO medication_logs (id, medication_id, profile_id, date, slot, taken_at) VALUES (?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(med_id)
    .bind(pid)
    .bind(date)
    .bind(slot)
    .bind(ts(at))
    .execute(pool)
    .await?;
    Ok(true)
}

// --- Symptome --------------------------------------------------------------

pub async fn symptoms(pool: &SqlitePool, pid: &str) -> AppResult<SymptomsData> {
    let list = sqlx::query_as::<_, Symptom>(
        "SELECT id, date, name, severity, notes, created_at FROM symptoms WHERE profile_id = ?
         ORDER BY date DESC, created_at DESC LIMIT 200",
    )
    .bind(pid)
    .fetch_all(pool)
    .await?;
    let recent_names = sqlx::query_scalar::<_, String>(
        "SELECT name FROM symptoms WHERE profile_id = ? GROUP BY name COLLATE NOCASE ORDER BY MAX(created_at) DESC LIMIT 12",
    )
    .bind(pid)
    .fetch_all(pool)
    .await?;
    Ok(SymptomsData { list, recent_names })
}

pub async fn add_symptom(
    pool: &SqlitePool,
    pid: &str,
    date: NaiveDate,
    name: String,
    severity: i64,
    notes: Option<String>,
) -> AppResult<()> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Symptom fehlt".into()));
    }
    sqlx::query(
        "INSERT INTO symptoms (id, profile_id, date, name, severity, notes, created_at) VALUES (?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(new_id())
    .bind(pid)
    .bind(date)
    .bind(name)
    .bind(severity.clamp(1, 5))
    .bind(clean(notes))
    .bind(now_ts())
    .execute(pool)
    .await?;
    Ok(())
}

pub async fn delete_symptom(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM symptoms WHERE id = ? AND profile_id = ?").bind(id).bind(pid).execute(pool).await?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[tokio::test]
    async fn meds_and_symptoms() {
        let (pool, pid) = super::super::body::test_pool().await;
        let input = MedicationInput {
            name: "Vitamin D".into(),
            dose: Some("1000 IE".into()),
            times: vec!["20:00".into(), "8:00".into()],
            notes: None,
            active: true,
        };
        let m = save(&pool, &pid, None, input.clone()).await.expect("save");
        assert_eq!(m.times, "08:00,20:00");
        let today = NaiveDate::from_ymd_opt(2026, 11, 20).expect("date");
        let at = crate::util::parse_datetime("2026-11-20T08:05").expect("dt");
        assert!(toggle_log(&pool, &pid, &m.id, today, "08:00", at).await.expect("toggle"));
        let d = data(&pool, &pid, today, today).await.expect("data");
        assert_eq!((d.meds.len(), d.logs.len()), (1, 1));
        assert!(!toggle_log(&pool, &pid, &m.id, today, "08:00", at).await.expect("toggle"));
        set_active(&pool, &pid, &m.id, false).await.expect("active");
        let m = save(&pool, &pid, Some(m.id.clone()), MedicationInput { name: "Vit D".into(), ..input })
            .await
            .expect("edit");
        assert_eq!(m.name, "Vit D");
        delete(&pool, &pid, &m.id).await.expect("delete");
        assert!(data(&pool, &pid, today, today).await.expect("data").meds.is_empty());

        add_symptom(&pool, &pid, today, "Kopfschmerzen".into(), 4, None).await.expect("sym");
        add_symptom(&pool, &pid, today, "Müdigkeit".into(), 9, Some("abends".into())).await.expect("sym");
        let s = symptoms(&pool, &pid).await.expect("symptoms");
        assert_eq!(s.list.len(), 2);
        assert_eq!(s.recent_names.len(), 2);
        assert!(s.list.iter().any(|x| x.severity == 5));
        delete_symptom(&pool, &pid, &s.list[0].id).await.expect("del");
        assert_eq!(symptoms(&pool, &pid).await.expect("symptoms").list.len(), 1);
    }
}

use sqlx::SqlitePool;

use super::{clean, new_id, now_ts};
use crate::model::{Goals, Profile, ProfileInput};
use crate::server::{AppError, AppResult};

const COLS: &str = "id, name, color, sex, birth_date, height_cm, activity_level, cycle_tracking";

pub async fn list(pool: &SqlitePool) -> AppResult<Vec<Profile>> {
    Ok(sqlx::query_as::<_, Profile>(&format!("SELECT {COLS} FROM profiles ORDER BY created_at"))
        .fetch_all(pool)
        .await?)
}

pub async fn get(pool: &SqlitePool, id: &str) -> AppResult<Profile> {
    sqlx::query_as::<_, Profile>(&format!("SELECT {COLS} FROM profiles WHERE id = ?"))
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Profil nicht gefunden".into()))
}

fn validate(input: &ProfileInput) -> AppResult<String> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name fehlt".into()));
    }
    Ok(name)
}

pub async fn create(pool: &SqlitePool, input: ProfileInput) -> AppResult<Profile> {
    let name = validate(&input)?;
    let id = new_id();
    let color = if input.color.is_empty() { "emerald".to_string() } else { input.color };
    let sex = if input.sex == "f" { "f" } else { "m" };
    let level = if input.activity_level.is_empty() { "light".to_string() } else { input.activity_level };
    sqlx::query(
        "INSERT INTO profiles (id, name, color, sex, birth_date, height_cm, activity_level, cycle_tracking, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(name)
    .bind(color)
    .bind(sex)
    .bind(input.birth_date)
    .bind(input.height_cm)
    .bind(level)
    .bind(input.cycle_tracking)
    .bind(now_ts())
    .execute(pool)
    .await?;
    sqlx::query("INSERT OR IGNORE INTO goals (profile_id) VALUES (?)").bind(&id).execute(pool).await?;
    get(pool, &id).await
}

pub async fn update(pool: &SqlitePool, id: &str, input: ProfileInput) -> AppResult<Profile> {
    let name = validate(&input)?;
    let sex = if input.sex == "f" { "f" } else { "m" };
    sqlx::query(
        "UPDATE profiles SET name = ?, color = ?, sex = ?, birth_date = ?, height_cm = ?, activity_level = ?, cycle_tracking = ?
         WHERE id = ?",
    )
    .bind(name)
    .bind(clean(Some(input.color)).unwrap_or_else(|| "emerald".into()))
    .bind(sex)
    .bind(input.birth_date)
    .bind(input.height_cm)
    .bind(clean(Some(input.activity_level)).unwrap_or_else(|| "light".into()))
    .bind(input.cycle_tracking)
    .bind(id)
    .execute(pool)
    .await?;
    get(pool, id).await
}

pub async fn delete(pool: &SqlitePool, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM profiles WHERE id = ?").bind(id).execute(pool).await?;
    Ok(())
}

pub async fn goals(pool: &SqlitePool, pid: &str) -> AppResult<Goals> {
    sqlx::query("INSERT OR IGNORE INTO goals (profile_id) VALUES (?)").bind(pid).execute(pool).await?;
    Ok(sqlx::query_as::<_, Goals>(
        "SELECT calories, protein_g, carbs_g, fat_g, fiber_g, sugar_g, water_ml, steps, exercise_min, sleep_hours,
                target_weight, weekly_change, fasting_hours
         FROM goals WHERE profile_id = ?",
    )
    .bind(pid)
    .fetch_one(pool)
    .await?)
}

pub async fn save_goals(pool: &SqlitePool, pid: &str, g: Goals) -> AppResult<Goals> {
    sqlx::query("INSERT OR IGNORE INTO goals (profile_id) VALUES (?)").bind(pid).execute(pool).await?;
    sqlx::query(
        "UPDATE goals SET calories = ?, protein_g = ?, carbs_g = ?, fat_g = ?, fiber_g = ?, sugar_g = ?, water_ml = ?,
                steps = ?, exercise_min = ?, sleep_hours = ?, target_weight = ?, weekly_change = ?, fasting_hours = ?
         WHERE profile_id = ?",
    )
    .bind(g.calories.max(0.0))
    .bind(g.protein_g.max(0.0))
    .bind(g.carbs_g.max(0.0))
    .bind(g.fat_g.max(0.0))
    .bind(g.fiber_g.max(0.0))
    .bind(g.sugar_g.max(0.0))
    .bind(g.water_ml.max(0.0))
    .bind(g.steps.max(0))
    .bind(g.exercise_min.max(0))
    .bind(g.sleep_hours.max(0.0))
    .bind(g.target_weight)
    .bind(g.weekly_change)
    .bind(g.fasting_hours.max(0.0))
    .bind(pid)
    .execute(pool)
    .await?;
    goals(pool, pid).await
}

/// Letztes erfasstes Gewicht (für kcal-Berechnungen).
pub async fn latest_weight(pool: &SqlitePool, pid: &str) -> AppResult<Option<f64>> {
    Ok(sqlx::query_scalar::<_, f64>(
        "SELECT weight FROM checkins WHERE profile_id = ? AND weight IS NOT NULL ORDER BY date DESC LIMIT 1",
    )
    .bind(pid)
    .fetch_optional(pool)
    .await?)
}

/// Gewicht im Check-in eines Tages setzen (legt den Check-in an, falls nötig).
pub async fn set_weight(pool: &SqlitePool, pid: &str, date: chrono::NaiveDate, weight: f64) -> AppResult<()> {
    sqlx::query(
        "INSERT INTO checkins (profile_id, date, weight) VALUES (?, ?, ?)
         ON CONFLICT(profile_id, date) DO UPDATE SET weight = excluded.weight",
    )
    .bind(pid)
    .bind(date)
    .bind(weight)
    .execute(pool)
    .await?;
    Ok(())
}

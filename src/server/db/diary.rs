use chrono::NaiveDate;
use sqlx::SqlitePool;

use super::{foods, new_id, now_ts};
use crate::model::{DayData, Food, FoodEntry, Nutrients, WaterEntry};
use crate::server::{AppError, AppResult};

const COLS: &str = "id, date, meal, food_id, name, brand, unit, amount, kcal, protein, carbs, fat, fiber, sugar, sodium_mg";

fn valid_meal(meal: &str) -> &str {
    match meal {
        "breakfast" | "lunch" | "dinner" | "snack" => meal,
        _ => "snack",
    }
}

pub async fn entries(pool: &SqlitePool, pid: &str, date: NaiveDate) -> AppResult<Vec<FoodEntry>> {
    Ok(sqlx::query_as::<_, FoodEntry>(&format!(
        "SELECT {COLS} FROM food_entries WHERE profile_id = ? AND date = ? ORDER BY created_at"
    ))
    .bind(pid)
    .bind(date)
    .fetch_all(pool)
    .await?)
}

pub async fn get_entry(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<FoodEntry> {
    sqlx::query_as::<_, FoodEntry>(&format!("SELECT {COLS} FROM food_entries WHERE id = ? AND profile_id = ?"))
        .bind(id)
        .bind(pid)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Eintrag nicht gefunden".into()))
}

async fn insert(pool: &SqlitePool, pid: &str, date: NaiveDate, meal: &str, food: &Food, amount: f64) -> AppResult<String> {
    let n = food.scaled(amount);
    let id = new_id();
    sqlx::query(
        "INSERT INTO food_entries (id, profile_id, date, meal, food_id, name, brand, unit, amount, kcal, protein, carbs, fat,
                                   fiber, sugar, sodium_mg, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(pid)
    .bind(date)
    .bind(valid_meal(meal))
    .bind(&food.id)
    .bind(&food.name)
    .bind(&food.brand)
    .bind(&food.unit)
    .bind(amount)
    .bind(n.kcal)
    .bind(n.protein)
    .bind(n.carbs)
    .bind(n.fat)
    .bind(n.fiber)
    .bind(n.sugar)
    .bind(n.sodium_mg)
    .bind(now_ts())
    .execute(pool)
    .await?;
    Ok(id)
}

pub async fn add_food(pool: &SqlitePool, pid: &str, date: NaiveDate, meal: &str, food_id: &str, amount: f64) -> AppResult<FoodEntry> {
    if !(amount > 0.0) {
        return Err(AppError::BadRequest("Menge muss grösser als 0 sein".into()));
    }
    let food = foods::get(pool, pid, food_id).await?;
    let id = insert(pool, pid, date, meal, &food, amount).await?;
    get_entry(pool, pid, &id).await
}

/// Schnelleintrag ohne Lebensmittel (nur kcal und Makros).
pub async fn add_quick(pool: &SqlitePool, pid: &str, date: NaiveDate, meal: &str, name: String, n: Nutrients) -> AppResult<FoodEntry> {
    let name = if name.trim().is_empty() { "Schnelleintrag".to_string() } else { name.trim().to_string() };
    let id = new_id();
    sqlx::query(
        "INSERT INTO food_entries (id, profile_id, date, meal, food_id, name, unit, amount, kcal, protein, carbs, fat,
                                   fiber, sugar, sodium_mg, created_at)
         VALUES (?, ?, ?, ?, NULL, ?, 'g', 0, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(pid)
    .bind(date)
    .bind(valid_meal(meal))
    .bind(name)
    .bind(n.kcal.max(0.0))
    .bind(n.protein.max(0.0))
    .bind(n.carbs.max(0.0))
    .bind(n.fat.max(0.0))
    .bind(n.fiber.max(0.0))
    .bind(n.sugar.max(0.0))
    .bind(n.sodium_mg.max(0.0))
    .bind(now_ts())
    .execute(pool)
    .await?;
    get_entry(pool, pid, &id).await
}

pub async fn add_meal(pool: &SqlitePool, pid: &str, date: NaiveDate, meal: &str, meal_id: &str, factor: f64) -> AppResult<usize> {
    let tpl = foods::meal(pool, pid, meal_id).await?;
    let factor = if factor > 0.0 { factor } else { 1.0 };
    for item in &tpl.items {
        insert(pool, pid, date, meal, &item.food, item.amount * factor).await?;
    }
    Ok(tpl.items.len())
}

/// Menge/Mahlzeit ändern; Nährwerte werden anteilig neu berechnet.
pub async fn update_entry(pool: &SqlitePool, pid: &str, id: &str, meal: &str, amount: f64) -> AppResult<FoodEntry> {
    let e = get_entry(pool, pid, id).await?;
    let f = if e.amount > 0.0 && amount > 0.0 { amount / e.amount } else { 1.0 };
    let amount = if e.amount > 0.0 { amount.max(0.0) } else { 0.0 };
    sqlx::query(
        "UPDATE food_entries SET meal = ?, amount = ?, kcal = kcal * ?, protein = protein * ?, carbs = carbs * ?, fat = fat * ?,
                fiber = fiber * ?, sugar = sugar * ?, sodium_mg = sodium_mg * ?
         WHERE id = ? AND profile_id = ?",
    )
    .bind(valid_meal(meal))
    .bind(amount)
    .bind(f)
    .bind(f)
    .bind(f)
    .bind(f)
    .bind(f)
    .bind(f)
    .bind(f)
    .bind(id)
    .bind(pid)
    .execute(pool)
    .await?;
    get_entry(pool, pid, id).await
}

pub async fn delete_entry(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM food_entries WHERE id = ? AND profile_id = ?").bind(id).bind(pid).execute(pool).await?;
    Ok(())
}

/// Alle Einträge eines Tages in einen anderen Tag kopieren (optional nur eine Mahlzeit).
pub async fn copy_day(pool: &SqlitePool, pid: &str, from: NaiveDate, to: NaiveDate, meal: Option<String>) -> AppResult<u64> {
    let r = sqlx::query(
        "INSERT INTO food_entries (id, profile_id, date, meal, food_id, name, brand, unit, amount, kcal, protein, carbs, fat,
                                   fiber, sugar, sodium_mg, created_at)
         SELECT lower(hex(randomblob(16))), profile_id, ?, meal, food_id, name, brand, unit, amount, kcal, protein, carbs, fat,
                fiber, sugar, sodium_mg, ?
         FROM food_entries WHERE profile_id = ? AND date = ? AND (? IS NULL OR meal = ?)",
    )
    .bind(to)
    .bind(now_ts())
    .bind(pid)
    .bind(from)
    .bind(&meal)
    .bind(&meal)
    .execute(pool)
    .await?;
    Ok(r.rows_affected())
}

// --- Wasser ---------------------------------------------------------------

pub async fn water(pool: &SqlitePool, pid: &str, date: NaiveDate) -> AppResult<Vec<WaterEntry>> {
    Ok(sqlx::query_as::<_, WaterEntry>(
        "SELECT id, ml, created_at FROM water_entries WHERE profile_id = ? AND date = ? ORDER BY created_at",
    )
    .bind(pid)
    .bind(date)
    .fetch_all(pool)
    .await?)
}

pub async fn add_water(pool: &SqlitePool, pid: &str, date: NaiveDate, ml: f64) -> AppResult<()> {
    if ml == 0.0 || !ml.is_finite() {
        return Ok(());
    }
    if ml < 0.0 {
        // Negativ = letzten Eintrag zurücknehmen.
        sqlx::query(
            "DELETE FROM water_entries WHERE id = (SELECT id FROM water_entries WHERE profile_id = ? AND date = ?
             ORDER BY created_at DESC LIMIT 1)",
        )
        .bind(pid)
        .bind(date)
        .execute(pool)
        .await?;
        return Ok(());
    }
    sqlx::query("INSERT INTO water_entries (id, profile_id, date, ml, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(new_id())
        .bind(pid)
        .bind(date)
        .bind(ml)
        .bind(now_ts())
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn delete_water(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM water_entries WHERE id = ? AND profile_id = ?").bind(id).bind(pid).execute(pool).await?;
    Ok(())
}

// --- Tag ------------------------------------------------------------------

pub async fn day(pool: &SqlitePool, pid: &str, date: NaiveDate) -> AppResult<DayData> {
    let entries = entries(pool, pid, date).await?;
    let water = water(pool, pid, date).await?;
    let water_ml = water.iter().map(|w| w.ml).sum();
    let (burned_kcal, exercise_min, exercise_count): (f64, f64, i64) = sqlx::query_as(
        "SELECT IFNULL(SUM(kcal),0.0), IFNULL(SUM(duration_min),0.0), COUNT(*) FROM exercise_entries
         WHERE profile_id = ? AND date = ?",
    )
    .bind(pid)
    .bind(date)
    .fetch_one(pool)
    .await?;
    let check: Option<(Option<i64>, Option<f64>, Option<i64>, Option<f64>)> = sqlx::query_as(
        "SELECT steps, weight, mood, sleep_hours FROM checkins WHERE profile_id = ? AND date = ?",
    )
    .bind(pid)
    .bind(date)
    .fetch_optional(pool)
    .await?;
    let (steps, weight, mood, sleep_hours) = check.unwrap_or_default();
    let fast: Option<(chrono::NaiveDateTime, f64)> = sqlx::query_as(
        "SELECT start_at, target_hours FROM fasts WHERE profile_id = ? AND end_at IS NULL ORDER BY start_at DESC LIMIT 1",
    )
    .bind(pid)
    .fetch_optional(pool)
    .await?;
    Ok(DayData {
        entries,
        water_ml,
        water,
        burned_kcal,
        exercise_min,
        exercise_count,
        steps,
        weight,
        mood,
        sleep_hours,
        fasting_since: fast.map(|f| f.0),
        fasting_target: fast.map(|f| f.1),
    })
}

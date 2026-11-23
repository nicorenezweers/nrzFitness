use sqlx::SqlitePool;

use super::{clean, new_id, now_ts};
use crate::model::{Food, FoodInput, MealItem, MealTemplate};
use crate::server::{AppError, AppResult};

/// Spalten inkl. berechneter `favorite`/`shared` für Profil `?1`.
const COLS: &str = "f.id, f.name, f.brand, f.barcode, f.unit, f.serving_size, f.serving_name, f.kcal, f.protein, f.carbs,
    f.fat, f.fiber, f.sugar, f.sat_fat, f.sodium_mg, f.source, f.image_url,
    EXISTS(SELECT 1 FROM food_favorites ff WHERE ff.food_id = f.id AND ff.profile_id = ?1) AS favorite,
    (f.profile_id IS NULL) AS shared";

const VISIBLE: &str = "(f.profile_id IS NULL OR f.profile_id = ?1)";

pub async fn get(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<Food> {
    sqlx::query_as::<_, Food>(sqlx::AssertSqlSafe(format!("SELECT {COLS} FROM foods f WHERE f.id = ?2")))
        .bind(pid)
        .bind(id)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Lebensmittel nicht gefunden".into()))
}

/// Suche in der eigenen Datenbank (Name, Marke, Barcode). Favoriten zuerst.
pub async fn search(pool: &SqlitePool, pid: &str, q: &str, limit: i64) -> AppResult<Vec<Food>> {
    let like = format!("%{}%", q.trim());
    Ok(sqlx::query_as::<_, Food>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM foods f
         WHERE {VISIBLE} AND (f.name LIKE ?2 OR IFNULL(f.brand,'') LIKE ?2 OR IFNULL(f.barcode,'') = ?3)
         ORDER BY favorite DESC, (f.name LIKE ?4) DESC, f.name COLLATE NOCASE LIMIT ?5"
    )))
    .bind(pid)
    .bind(&like)
    .bind(q.trim())
    .bind(format!("{}%", q.trim()))
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

pub async fn list_all(pool: &SqlitePool, pid: &str) -> AppResult<Vec<Food>> {
    Ok(sqlx::query_as::<_, Food>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM foods f WHERE {VISIBLE} ORDER BY f.name COLLATE NOCASE"
    )))
    .bind(pid)
    .fetch_all(pool)
    .await?)
}

pub async fn favorites(pool: &SqlitePool, pid: &str) -> AppResult<Vec<Food>> {
    Ok(sqlx::query_as::<_, Food>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM foods f JOIN food_favorites fav ON fav.food_id = f.id AND fav.profile_id = ?1
         ORDER BY f.name COLLATE NOCASE"
    )))
    .bind(pid)
    .fetch_all(pool)
    .await?)
}

/// Zuletzt gegessene Lebensmittel (einmalig, neueste zuerst).
pub async fn recent(pool: &SqlitePool, pid: &str, limit: i64) -> AppResult<Vec<Food>> {
    Ok(sqlx::query_as::<_, Food>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM foods f
         JOIN (SELECT food_id, MAX(created_at) AS last FROM food_entries
               WHERE profile_id = ?1 AND food_id IS NOT NULL GROUP BY food_id) r ON r.food_id = f.id
         ORDER BY r.last DESC LIMIT ?2"
    )))
    .bind(pid)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

/// Zuletzt verwendete Menge eines Lebensmittels (für die Mengen-Vorauswahl).
pub async fn last_amount(pool: &SqlitePool, pid: &str, food_id: &str) -> AppResult<Option<f64>> {
    Ok(sqlx::query_scalar::<_, f64>(
        "SELECT amount FROM food_entries WHERE profile_id = ? AND food_id = ? ORDER BY created_at DESC LIMIT 1",
    )
    .bind(pid)
    .bind(food_id)
    .fetch_optional(pool)
    .await?)
}

pub async fn by_barcode(pool: &SqlitePool, pid: &str, code: &str) -> AppResult<Option<Food>> {
    Ok(sqlx::query_as::<_, Food>(sqlx::AssertSqlSafe(format!(
        "SELECT {COLS} FROM foods f WHERE {VISIBLE} AND f.barcode = ?2 ORDER BY (f.profile_id IS NULL) LIMIT 1"
    )))
    .bind(pid)
    .bind(code.trim())
    .fetch_optional(pool)
    .await?)
}

fn sanitize(mut f: FoodInput) -> AppResult<FoodInput> {
    f.name = f.name.trim().to_string();
    if f.name.is_empty() {
        return Err(AppError::BadRequest("Name fehlt".into()));
    }
    if f.unit != "ml" {
        f.unit = "g".into();
    }
    if !(f.serving_size > 0.0) {
        f.serving_size = 100.0;
    }
    if f.source.is_empty() {
        f.source = "custom".into();
    }
    for v in [&mut f.kcal, &mut f.protein, &mut f.carbs, &mut f.fat, &mut f.fiber, &mut f.sugar, &mut f.sat_fat, &mut f.sodium_mg] {
        if !v.is_finite() || *v < 0.0 {
            *v = 0.0;
        }
    }
    f.brand = clean(f.brand);
    f.barcode = clean(f.barcode);
    f.serving_name = clean(f.serving_name);
    f.image_url = clean(f.image_url);
    Ok(f)
}

pub async fn create(pool: &SqlitePool, pid: &str, input: FoodInput) -> AppResult<Food> {
    let f = sanitize(input)?;
    // Gleicher Barcode schon vorhanden → vorhandenes Lebensmittel verwenden.
    if let Some(code) = &f.barcode
        && let Some(existing) = by_barcode(pool, pid, code).await?
    {
        return Ok(existing);
    }
    let id = new_id();
    sqlx::query(
        "INSERT INTO foods (id, profile_id, name, brand, barcode, unit, serving_size, serving_name, kcal, protein, carbs, fat,
                            fiber, sugar, sat_fat, sodium_mg, source, image_url, created_at)
         VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(if f.shared { None } else { Some(pid) })
    .bind(&f.name)
    .bind(&f.brand)
    .bind(&f.barcode)
    .bind(&f.unit)
    .bind(f.serving_size)
    .bind(&f.serving_name)
    .bind(f.kcal)
    .bind(f.protein)
    .bind(f.carbs)
    .bind(f.fat)
    .bind(f.fiber)
    .bind(f.sugar)
    .bind(f.sat_fat)
    .bind(f.sodium_mg)
    .bind(&f.source)
    .bind(&f.image_url)
    .bind(now_ts())
    .execute(pool)
    .await?;
    get(pool, pid, &id).await
}

pub async fn update(pool: &SqlitePool, pid: &str, id: &str, input: FoodInput) -> AppResult<Food> {
    let f = sanitize(input)?;
    sqlx::query(
        "UPDATE foods SET profile_id = ?, name = ?, brand = ?, barcode = ?, unit = ?, serving_size = ?, serving_name = ?,
                kcal = ?, protein = ?, carbs = ?, fat = ?, fiber = ?, sugar = ?, sat_fat = ?, sodium_mg = ?, image_url = ?
         WHERE id = ? AND (profile_id IS NULL OR profile_id = ?)",
    )
    .bind(if f.shared { None } else { Some(pid) })
    .bind(&f.name)
    .bind(&f.brand)
    .bind(&f.barcode)
    .bind(&f.unit)
    .bind(f.serving_size)
    .bind(&f.serving_name)
    .bind(f.kcal)
    .bind(f.protein)
    .bind(f.carbs)
    .bind(f.fat)
    .bind(f.fiber)
    .bind(f.sugar)
    .bind(f.sat_fat)
    .bind(f.sodium_mg)
    .bind(&f.image_url)
    .bind(id)
    .bind(pid)
    .execute(pool)
    .await?;
    get(pool, pid, id).await
}

pub async fn delete(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM foods WHERE id = ? AND (profile_id IS NULL OR profile_id = ?)")
        .bind(id)
        .bind(pid)
        .execute(pool)
        .await?;
    Ok(())
}

pub async fn set_favorite(pool: &SqlitePool, pid: &str, id: &str, fav: bool) -> AppResult<()> {
    if fav {
        sqlx::query("INSERT OR IGNORE INTO food_favorites (profile_id, food_id) VALUES (?, ?)")
    } else {
        sqlx::query("DELETE FROM food_favorites WHERE profile_id = ? AND food_id = ?")
    }
    .bind(pid)
    .bind(id)
    .execute(pool)
    .await?;
    Ok(())
}

// --- Mahlzeiten-Vorlagen ----------------------------------------------------

#[derive(sqlx::FromRow)]
struct MealRow {
    id: String,
    name: String,
    notes: Option<String>,
}

pub async fn meals(pool: &SqlitePool, pid: &str) -> AppResult<Vec<MealTemplate>> {
    let rows = sqlx::query_as::<_, MealRow>("SELECT id, name, notes FROM meals WHERE profile_id = ? ORDER BY name COLLATE NOCASE")
        .bind(pid)
        .fetch_all(pool)
        .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        let items = meal_items(pool, pid, &r.id).await?;
        out.push(MealTemplate { id: r.id, name: r.name, notes: r.notes, items });
    }
    Ok(out)
}

async fn meal_items(pool: &SqlitePool, pid: &str, meal_id: &str) -> AppResult<Vec<MealItem>> {
    #[derive(sqlx::FromRow)]
    struct ItemRow {
        id: String,
        food_id: String,
        amount: f64,
    }
    let rows = sqlx::query_as::<_, ItemRow>("SELECT id, food_id, amount FROM meal_items WHERE meal_id = ? ORDER BY position")
        .bind(meal_id)
        .fetch_all(pool)
        .await?;
    let mut items = Vec::new();
    for r in rows {
        if let Ok(food) = get(pool, pid, &r.food_id).await {
            items.push(MealItem { id: r.id, food, amount: r.amount });
        }
    }
    Ok(items)
}

pub async fn meal(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<MealTemplate> {
    let r = sqlx::query_as::<_, MealRow>("SELECT id, name, notes FROM meals WHERE id = ? AND profile_id = ?")
        .bind(id)
        .bind(pid)
        .fetch_optional(pool)
        .await?
        .ok_or_else(|| AppError::NotFound("Mahlzeit nicht gefunden".into()))?;
    let items = meal_items(pool, pid, &r.id).await?;
    Ok(MealTemplate { id: r.id, name: r.name, notes: r.notes, items })
}

/// Legt eine Vorlage an oder ersetzt sie (id = None → neu). `items`: (food_id, Menge).
pub async fn save_meal(
    pool: &SqlitePool,
    pid: &str,
    id: Option<String>,
    name: String,
    notes: Option<String>,
    items: Vec<(String, f64)>,
) -> AppResult<MealTemplate> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name fehlt".into()));
    }
    let mut tx = pool.begin().await?;
    let id = match id {
        Some(id) => {
            sqlx::query("UPDATE meals SET name = ?, notes = ? WHERE id = ? AND profile_id = ?")
                .bind(&name)
                .bind(clean(notes))
                .bind(&id)
                .bind(pid)
                .execute(&mut *tx)
                .await?;
            sqlx::query("DELETE FROM meal_items WHERE meal_id = ?").bind(&id).execute(&mut *tx).await?;
            id
        }
        None => {
            let id = new_id();
            sqlx::query("INSERT INTO meals (id, profile_id, name, notes, created_at) VALUES (?, ?, ?, ?, ?)")
                .bind(&id)
                .bind(pid)
                .bind(&name)
                .bind(clean(notes))
                .bind(now_ts())
                .execute(&mut *tx)
                .await?;
            id
        }
    };
    for (pos, (food_id, amount)) in items.into_iter().enumerate() {
        sqlx::query("INSERT INTO meal_items (id, meal_id, food_id, amount, position) VALUES (?, ?, ?, ?, ?)")
            .bind(new_id())
            .bind(&id)
            .bind(food_id)
            .bind(amount.max(0.0))
            .bind(pos as i64)
            .execute(&mut *tx)
            .await?;
    }
    tx.commit().await?;
    meal(pool, pid, &id).await
}

pub async fn delete_meal(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM meals WHERE id = ? AND profile_id = ?").bind(id).bind(pid).execute(pool).await?;
    Ok(())
}

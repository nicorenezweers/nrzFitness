//! Berichte (Tageswerte per GROUP BY date), JSON-Backup (generisch über PRAGMA table_info) und CSV-Export.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use serde_json::{Map, Value};
use sqlx::{Sqlite, SqlitePool, Transaction};

use super::{now_ts, profile};
use crate::model::{ActivityDay, DayRow, ImportResult, Report, Summary, TopExercise, TopFood};
use crate::server::{AppError, AppResult};
use crate::util::week_start;

const BACKUP_APP: &str = "nrzFitness";
const BACKUP_FORMAT: i64 = 1;

fn parse_day(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok()
}

// ---------------------------------------------------------------------------------------------
// Berichte
// ---------------------------------------------------------------------------------------------

/// Eine Zeile pro Tag von `from` bis `to` (inklusive), auch für Tage ohne Daten.
pub async fn day_rows(pool: &SqlitePool, pid: &str, from: NaiveDate, to: NaiveDate) -> AppResult<Vec<DayRow>> {
    let n = (to - from).num_days().max(0) + 1;
    let mut rows: Vec<DayRow> = (0..n).map(|i| DayRow::empty(from + Duration::days(i))).collect();
    let idx = |s: &str| parse_day(s).map(|d| (d - from).num_days()).filter(|i| *i >= 0 && *i < n).map(|i| i as usize);

    #[allow(clippy::type_complexity)]
    let food: Vec<(String, i64, f64, f64, f64, f64, f64, f64)> = sqlx::query_as(
        "SELECT date, COUNT(*), TOTAL(kcal), TOTAL(protein), TOTAL(carbs), TOTAL(fat), TOTAL(fiber), TOTAL(sugar)
         FROM food_entries WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3 GROUP BY date",
    )
    .bind(pid)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    for (d, cnt, kcal, p, c, f, fi, su) in food {
        if let Some(i) = idx(&d) {
            let r = &mut rows[i];
            (r.food_count, r.kcal, r.protein, r.carbs, r.fat, r.fiber, r.sugar) = (cnt, kcal, p, c, f, fi, su);
        }
    }

    let water: Vec<(String, f64)> = sqlx::query_as(
        "SELECT date, TOTAL(ml) FROM water_entries WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3 GROUP BY date",
    )
    .bind(pid)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    for (d, ml) in water {
        if let Some(i) = idx(&d) {
            rows[i].water_ml = ml;
        }
    }

    let ex: Vec<(String, i64, f64, f64)> = sqlx::query_as(
        "SELECT date, COUNT(*), TOTAL(duration_min), TOTAL(kcal)
         FROM exercise_entries WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3 GROUP BY date",
    )
    .bind(pid)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    for (d, cnt, min, kcal) in ex {
        if let Some(i) = idx(&d) {
            let r = &mut rows[i];
            (r.workouts, r.exercise_min, r.burned_kcal) = (cnt, min, kcal);
        }
    }

    #[allow(clippy::type_complexity)]
    let ci: Vec<(
        String,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<i64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
        Option<f64>,
    )> = sqlx::query_as(
        "SELECT date, CAST(weight AS REAL), CAST(body_fat AS REAL), CAST(waist AS REAL), CAST(steps AS INTEGER),
                    CAST(mood AS REAL), CAST(energy AS REAL), CAST(sleep_hours AS REAL), CAST(sleep_quality AS REAL)
             FROM checkins WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3",
    )
    .bind(pid)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    for (d, w, bf, wa, st, mo, en, sl, sq) in ci {
        if let Some(i) = idx(&d) {
            let r = &mut rows[i];
            (r.weight, r.body_fat, r.waist, r.steps, r.mood, r.energy, r.sleep_h, r.sleep_quality) =
                (w, bf, wa, st, mo, en, sl, sq);
        }
    }

    let fasts: Vec<(String, i64, f64, i64, f64)> = sqlx::query_as(
        "SELECT substr(end_at, 1, 10) AS d, COUNT(*),
                TOTAL((julianday(end_at) - julianday(start_at)) * 24),
                SUM(CASE WHEN (julianday(end_at) - julianday(start_at)) * 24 + 0.001 >= target_hours THEN 1 ELSE 0 END),
                MAX((julianday(end_at) - julianday(start_at)) * 24)
         FROM fasts
         WHERE profile_id = ?1 AND end_at IS NOT NULL AND substr(end_at, 1, 10) BETWEEN ?2 AND ?3
         GROUP BY d",
    )
    .bind(pid)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    for (d, cnt, hours, reached, max_h) in fasts {
        if let Some(i) = idx(&d) {
            let r = &mut rows[i];
            r.fasts = cnt;
            r.fast_hours = Some(hours.max(0.0));
            r.fasts_reached = reached;
            r.fast_max_h = Some(max_h.max(0.0));
        }
    }
    Ok(rows)
}

/// Tage mit Einträgen (Anzahl Bereiche je Tag) im Zeitraum.
pub async fn activity(pool: &SqlitePool, pid: &str, from: NaiveDate, to: NaiveDate) -> AppResult<Vec<ActivityDay>> {
    let rows: Vec<(String, i64)> = sqlx::query_as(
        "SELECT d, COUNT(DISTINCT k) FROM (
             SELECT date AS d, 1 AS k FROM food_entries WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3
             UNION ALL SELECT date, 2 FROM water_entries WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3
             UNION ALL SELECT date, 3 FROM exercise_entries WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3
             UNION ALL SELECT date, 4 FROM checkins WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3
             UNION ALL SELECT substr(start_at, 1, 10), 5 FROM fasts
                       WHERE profile_id = ?1 AND substr(start_at, 1, 10) BETWEEN ?2 AND ?3
         ) GROUP BY d ORDER BY d",
    )
    .bind(pid)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;
    Ok(rows
        .into_iter()
        .filter_map(|(d, k)| parse_day(&d).map(|date| ActivityDay { date, level: k.clamp(0, 5) as u8 }))
        .collect())
}

/// Aktuelle Serie (bis heute; zählt ab gestern, wenn heute noch nichts erfasst ist) und längste Serie.
pub fn streaks(days: &HashSet<NaiveDate>, today: NaiveDate, since: NaiveDate) -> (i64, i64) {
    let mut d = if days.contains(&today) { today } else { today - Duration::days(1) };
    let mut cur = 0;
    while days.contains(&d) {
        cur += 1;
        d -= Duration::days(1);
    }
    let mut sorted: Vec<NaiveDate> = days.iter().copied().filter(|d| *d >= since && *d <= today).collect();
    sorted.sort();
    let (mut best, mut run, mut prev): (i64, i64, Option<NaiveDate>) = (0, 0, None);
    for d in sorted {
        run = if prev.is_some_and(|p| d - p == Duration::days(1)) { run + 1 } else { 1 };
        best = best.max(run);
        prev = Some(d);
    }
    (cur, best.max(cur))
}

pub async fn report(
    pool: &SqlitePool,
    pid: &str,
    from: NaiveDate,
    to: NaiveDate,
    today: NaiveDate,
) -> AppResult<Report> {
    if to < from {
        return Err(AppError::BadRequest("Ungültiger Zeitraum".into()));
    }
    let len = (to - from).num_days() + 1;
    if len > 800 {
        return Err(AppError::BadRequest("Zeitraum zu lang".into()));
    }
    let goals = profile::goals(pool, pid).await?;
    // Aktueller und vorheriger Zeitraum in einem Durchgang.
    let all = day_rows(pool, pid, from - Duration::days(len), to).await?;
    let (prev_rows, cur_rows) = all.split_at(len as usize);
    let summary = Summary::from_days(cur_rows, goals.steps);
    let prev = Summary::from_days(prev_rows, goals.steps);

    let top_sql = |order: &str| {
        format!(
            "SELECT name, COUNT(*) AS count, TOTAL(kcal) AS kcal FROM food_entries
             WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3 GROUP BY name ORDER BY {order} LIMIT 10"
        )
    };
    let top_foods_count = sqlx::query_as::<_, TopFood>(sqlx::AssertSqlSafe(top_sql("count DESC, kcal DESC")))
        .bind(pid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    let top_foods_kcal = sqlx::query_as::<_, TopFood>(sqlx::AssertSqlSafe(top_sql("kcal DESC, count DESC")))
        .bind(pid)
        .bind(from)
        .bind(to)
        .fetch_all(pool)
        .await?;
    let top_exercises = sqlx::query_as::<_, TopExercise>(
        "SELECT name, COUNT(*) AS count, TOTAL(duration_min) AS minutes, TOTAL(kcal) AS kcal FROM exercise_entries
         WHERE profile_id = ?1 AND date BETWEEN ?2 AND ?3 GROUP BY name ORDER BY count DESC, minutes DESC LIMIT 8",
    )
    .bind(pid)
    .bind(from)
    .bind(to)
    .fetch_all(pool)
    .await?;

    // Konsistenz-Kalender: 18 Wochen, bei langen Zeiträumen ein Jahr (53 Wochen).
    let weeks = if len > 120 { 53 } else { 18 };
    let heat_from = week_start(to) - Duration::days((weeks - 1) * 7);
    let since = today - Duration::days(365);
    let act = activity(pool, pid, heat_from.min(since), to.max(today)).await?;
    let set: HashSet<NaiveDate> = act.iter().map(|a| a.date).collect();
    let (streak, best_streak) = streaks(&set, today, since);
    let activity = act.into_iter().filter(|a| a.date >= heat_from && a.date <= to).collect();

    Ok(Report {
        from,
        to,
        days: cur_rows.to_vec(),
        summary,
        prev,
        goals,
        top_foods_count,
        top_foods_kcal,
        top_exercises,
        activity,
        heat_from,
        streak,
        best_streak,
    })
}

// ---------------------------------------------------------------------------------------------
// JSON-Backup (generisch)
// ---------------------------------------------------------------------------------------------

#[derive(Clone, Copy, PartialEq, Debug)]
enum Kind {
    /// Die Profilzeile selbst (wird beim Import auf das aktive Profil übertragen).
    Profile,
    /// Zeilen mit `profile_id`.
    Owned,
    /// Bibliothek (eigene Zeilen + verwendete gemeinsame Zeilen mit `profile_id` NULL).
    Library,
    /// Kindtabelle ohne `profile_id`: (Elterntabelle, Fremdschlüssel).
    Child(&'static str, &'static str),
}

/// Bekannte Tabellen in Import-Reihenfolge mit WHERE-Bedingung für den Export (?1 = Profil-ID).
const TABLES: &[(&str, Kind, &str)] = &[
    ("profiles", Kind::Profile, "id = ?1"),
    ("goals", Kind::Owned, "profile_id = ?1"),
    (
        "foods",
        Kind::Library,
        "profile_id = ?1
         OR id IN (SELECT food_id FROM food_favorites WHERE profile_id = ?1)
         OR id IN (SELECT food_id FROM food_entries WHERE profile_id = ?1 AND food_id IS NOT NULL)
         OR id IN (SELECT mi.food_id FROM meal_items mi JOIN meals m ON m.id = mi.meal_id WHERE m.profile_id = ?1)",
    ),
    ("food_favorites", Kind::Owned, "profile_id = ?1"),
    ("meals", Kind::Owned, "profile_id = ?1"),
    ("meal_items", Kind::Child("meals", "meal_id"), "meal_id IN (SELECT id FROM meals WHERE profile_id = ?1)"),
    ("food_entries", Kind::Owned, "profile_id = ?1"),
    ("water_entries", Kind::Owned, "profile_id = ?1"),
    (
        "exercises",
        Kind::Library,
        "profile_id = ?1 OR id IN (SELECT exercise_id FROM exercise_entries WHERE profile_id = ?1 AND exercise_id IS NOT NULL)",
    ),
    ("exercise_entries", Kind::Owned, "profile_id = ?1"),
    (
        "exercise_sets",
        Kind::Child("exercise_entries", "entry_id"),
        "entry_id IN (SELECT id FROM exercise_entries WHERE profile_id = ?1)",
    ),
    ("checkins", Kind::Owned, "profile_id = ?1"),
    ("fasts", Kind::Owned, "profile_id = ?1"),
    ("medications", Kind::Owned, "profile_id = ?1"),
    ("medication_logs", Kind::Owned, "profile_id = ?1"),
    ("symptoms", Kind::Owned, "profile_id = ?1"),
    ("cycle_days", Kind::Owned, "profile_id = ?1"),
];

#[derive(Debug, Clone)]
struct Col {
    name: String,
    pk: bool,
}

fn quote(ident: &str) -> String {
    format!("\"{}\"", ident.replace('"', "\"\""))
}

async fn columns<'e, E: sqlx::Executor<'e, Database = Sqlite>>(ex: E, table: &str) -> AppResult<Vec<Col>> {
    let rows: Vec<(String, i64)> =
        sqlx::query_as(sqlx::AssertSqlSafe(format!("SELECT name, pk FROM pragma_table_info({})", quote_str(table)))).fetch_all(ex).await?;
    Ok(rows.into_iter().map(|(name, pk)| Col { name, pk: pk > 0 }).collect())
}

fn quote_str(s: &str) -> String {
    format!("'{}'", s.replace('\'', "''"))
}

async fn table_names<'e, E: sqlx::Executor<'e, Database = Sqlite>>(ex: E) -> AppResult<Vec<String>> {
    Ok(sqlx::query_scalar(
        "SELECT name FROM sqlite_master WHERE type = 'table' AND name NOT LIKE 'sqlite_%' AND name NOT LIKE '_sqlx%' ORDER BY name",
    )
    .fetch_all(ex)
    .await?)
}

/// Bekannte Tabellen + alle weiteren Tabellen mit `profile_id` (spätere Migrationen).
async fn plan(pool: &SqlitePool) -> AppResult<Vec<(String, Kind, String, Vec<Col>)>> {
    let mut out = Vec::new();
    for (t, k, w) in TABLES {
        let cols = columns(pool, t).await?;
        if !cols.is_empty() {
            out.push((t.to_string(), *k, w.to_string(), cols));
        }
    }
    for t in table_names(pool).await? {
        if TABLES.iter().any(|(n, ..)| *n == t) {
            continue;
        }
        let cols = columns(pool, &t).await?;
        if cols.iter().any(|c| c.name == "profile_id") {
            out.push((t, Kind::Owned, "profile_id = ?1".into(), cols));
        }
    }
    Ok(out)
}

async fn json_rows(pool: &SqlitePool, table: &str, cols: &[Col], where_sql: &str, pid: &str) -> AppResult<Vec<Value>> {
    let obj = cols.iter().map(|c| format!("{}, {}", quote_str(&c.name), quote(&c.name))).collect::<Vec<_>>().join(", ");
    let rows: Vec<String> = sqlx::query_scalar(sqlx::AssertSqlSafe(format!(
        "SELECT json_object({obj}) FROM {} WHERE {where_sql} ORDER BY rowid",
        quote(table)
    )))
    .bind(pid)
    .fetch_all(pool)
    .await?;
    rows.iter().map(|r| serde_json::from_str(r).map_err(|e| AppError::Http(format!("JSON: {e}")))).collect()
}

/// Exportiert alle Daten des Profils als JSON-Text.
pub async fn export_json(pool: &SqlitePool, pid: &str) -> AppResult<String> {
    let name: Option<String> =
        sqlx::query_scalar("SELECT name FROM profiles WHERE id = ?").bind(pid).fetch_optional(pool).await?;
    let Some(name) = name else {
        return Err(AppError::NotFound("Profil nicht gefunden".into()));
    };
    let mut tables = Map::new();
    for (t, _, w, cols) in plan(pool).await? {
        tables.insert(t.clone(), Value::Array(json_rows(pool, &t, &cols, &w, pid).await?));
    }
    let doc = serde_json::json!({
        "app": BACKUP_APP,
        "format": BACKUP_FORMAT,
        "exported_at": now_ts(),
        "profile_id": pid,
        "profile_name": name,
        "tables": tables,
    });
    serde_json::to_string(&doc).map_err(|e| AppError::Http(format!("JSON: {e}")))
}

fn bind_value<'q>(
    q: sqlx::query::Query<'q, Sqlite, sqlx::sqlite::SqliteArguments>,
    v: &Value,
) -> sqlx::query::Query<'q, Sqlite, sqlx::sqlite::SqliteArguments> {
    match v {
        Value::Null => q.bind(None::<String>),
        Value::Bool(b) => q.bind(i64::from(*b)),
        Value::Number(n) => match n.as_i64() {
            Some(i) => q.bind(i),
            None => q.bind(n.as_f64().unwrap_or(0.0)),
        },
        Value::String(s) => q.bind(s.clone()),
        other => q.bind(other.to_string()),
    }
}

/// Importiert ein JSON-Backup in das Profil `pid` (eine Transaktion; IDs bleiben, vorhandene Zeilen werden ersetzt).
/// Zeilen, deren ID bereits einem anderen Profil gehört, werden übersprungen.
pub async fn import_json(pool: &SqlitePool, pid: &str, json: &str) -> AppResult<ImportResult> {
    let doc: Value =
        serde_json::from_str(json).map_err(|_| AppError::BadRequest("Datei ist kein gültiges JSON".into()))?;
    if doc.get("app").and_then(Value::as_str) != Some(BACKUP_APP) {
        return Err(AppError::BadRequest("Keine nrzFitness-Sicherung".into()));
    }
    if doc.get("format").and_then(Value::as_i64).unwrap_or(0) > BACKUP_FORMAT {
        return Err(AppError::BadRequest("Sicherung stammt aus einer neueren Version".into()));
    }
    let Some(tables) = doc.get("tables").and_then(Value::as_object) else {
        return Err(AppError::BadRequest("Sicherung enthält keine Tabellen".into()));
    };
    let exists: Option<String> =
        sqlx::query_scalar("SELECT id FROM profiles WHERE id = ?").bind(pid).fetch_optional(pool).await?;
    if exists.is_none() {
        return Err(AppError::NotFound("Profil nicht gefunden".into()));
    }

    let plan = plan(pool).await?;
    let mut tx = pool.begin().await?;
    sqlx::query("PRAGMA defer_foreign_keys = ON").execute(&mut *tx).await?;
    let mut res = ImportResult { done: true, ..Default::default() };
    for (table, kind, _, cols) in &plan {
        let Some(rows) = tables.get(table).and_then(Value::as_array) else { continue };
        res.tables += 1;
        for row in rows {
            let Some(obj) = row.as_object() else {
                res.skipped += 1;
                continue;
            };
            let n = import_row(&mut tx, pid, table, *kind, cols, obj).await?;
            if n > 0 { res.imported += 1 } else { res.skipped += 1 }
        }
    }
    tx.commit().await.map_err(|e| match e {
        sqlx::Error::Database(d) if d.message().contains("FOREIGN KEY") => {
            AppError::BadRequest("Sicherung ist unvollständig (fehlende Verknüpfungen)".into())
        }
        other => other.into(),
    })?;
    Ok(res)
}

async fn import_row(
    tx: &mut Transaction<'_, Sqlite>,
    pid: &str,
    table: &str,
    kind: Kind,
    cols: &[Col],
    obj: &Map<String, Value>,
) -> AppResult<u64> {
    let mut row: BTreeMap<&str, Value> = BTreeMap::new();
    for c in cols {
        if let Some(v) = obj.get(&c.name) {
            row.insert(c.name.as_str(), v.clone());
        }
    }
    let has_pid = cols.iter().any(|c| c.name == "profile_id");
    let shared = kind == Kind::Library && obj.get("profile_id").is_none_or(Value::is_null);
    if has_pid && !shared {
        row.insert("profile_id", Value::String(pid.to_string()));
    }

    if kind == Kind::Profile {
        let sets: Vec<(&str, &Value)> =
            row.iter().filter(|(k, _)| !matches!(**k, "id" | "created_at")).map(|(k, v)| (*k, v)).collect();
        if sets.is_empty() {
            return Ok(0);
        }
        let sql = format!(
            "UPDATE profiles SET {} WHERE id = ?",
            sets.iter().map(|(k, _)| format!("{} = ?", quote(k))).collect::<Vec<_>>().join(", ")
        );
        let mut q = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
        for (_, v) in &sets {
            q = bind_value(q, v);
        }
        return Ok(q.bind(pid).execute(&mut **tx).await?.rows_affected());
    }

    if row.is_empty() {
        return Ok(0);
    }
    let names: Vec<&str> = row.keys().copied().collect();
    let col_list = names.iter().map(|n| quote(n)).collect::<Vec<_>>().join(", ");
    let marks = vec!["?"; names.len()].join(", ");
    let updates: Vec<String> = names
        .iter()
        .filter(|n| !cols.iter().any(|c| c.pk && c.name == **n))
        .map(|n| format!("{0} = excluded.{0}", quote(n)))
        .collect();
    let t = quote(table);
    let on_conflict = if shared || updates.is_empty() {
        "ON CONFLICT DO NOTHING".to_string()
    } else if has_pid {
        format!("ON CONFLICT DO UPDATE SET {} WHERE {t}.profile_id = excluded.profile_id", updates.join(", "))
    } else {
        format!("ON CONFLICT DO UPDATE SET {}", updates.join(", "))
    };

    let sql = match kind {
        Kind::Child(parent, fk) => {
            let Some(parent_id) = row.get(fk).cloned() else { return Ok(0) };
            let sql = format!(
                "INSERT INTO {t} ({col_list}) SELECT {marks} WHERE EXISTS (SELECT 1 FROM {} WHERE id = ? AND profile_id = ?) {on_conflict}",
                quote(parent)
            );
            let mut q = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
            for n in &names {
                q = bind_value(q, &row[n]);
            }
            q = bind_value(q, &parent_id).bind(pid);
            return Ok(q.execute(&mut **tx).await?.rows_affected());
        }
        _ => format!("INSERT INTO {t} ({col_list}) VALUES ({marks}) {on_conflict}"),
    };
    let mut q = sqlx::query(sqlx::AssertSqlSafe(sql.as_str()));
    for n in &names {
        q = bind_value(q, &row[n]);
    }
    Ok(q.execute(&mut **tx).await?.rows_affected())
}

// ---------------------------------------------------------------------------------------------
// CSV
// ---------------------------------------------------------------------------------------------

fn csv_cell(v: &Value) -> String {
    let s = match v {
        Value::Null => String::new(),
        Value::Bool(b) => if *b { "1" } else { "0" }.to_string(),
        Value::Number(n) => match n.as_i64() {
            Some(i) => i.to_string(),
            None => {
                let s = format!("{:.2}", n.as_f64().unwrap_or(0.0));
                let s = s.trim_end_matches('0').trim_end_matches('.').to_string();
                if s == "-0" { "0".into() } else { s }
            }
        },
        Value::String(s) => s.clone(),
        other => other.to_string(),
    };
    if s.contains([';', '"', '\n', '\r']) { format!("\"{}\"", s.replace('"', "\"\"")) } else { s }
}

/// CSV (Semikolon, Punkt als Dezimaltrenner – passt zu Excel in der Schweiz) mit UTF-8-BOM.
fn to_csv(headers: &[(&str, &str)], rows: &[Value]) -> String {
    let mut out = String::from("\u{feff}");
    out.push_str(&headers.iter().map(|(_, h)| *h).collect::<Vec<_>>().join(";"));
    out.push_str("\r\n");
    for r in rows {
        let line =
            headers.iter().map(|(k, _)| csv_cell(r.get(*k).unwrap_or(&Value::Null))).collect::<Vec<_>>().join(";");
        out.push_str(&line);
        out.push_str("\r\n");
    }
    out
}

async fn select_json(
    pool: &SqlitePool,
    sql_cols: &[(&str, &str)],
    from_where: &str,
    pid: &str,
) -> AppResult<Vec<Value>> {
    let obj = sql_cols.iter().map(|(k, expr)| format!("{}, {expr}", quote_str(k))).collect::<Vec<_>>().join(", ");
    let rows: Vec<String> =
        sqlx::query_scalar(sqlx::AssertSqlSafe(format!("SELECT json_object({obj}) {from_where}"))).bind(pid).fetch_all(pool).await?;
    rows.iter().map(|r| serde_json::from_str(r).map_err(|e| AppError::Http(format!("JSON: {e}")))).collect()
}

pub async fn diary_csv(pool: &SqlitePool, pid: &str) -> AppResult<String> {
    let cols = [
        ("date", "date"),
        (
            "meal",
            "CASE meal WHEN 'breakfast' THEN 'Frühstück' WHEN 'lunch' THEN 'Mittagessen' WHEN 'dinner' THEN 'Abendessen' ELSE 'Snacks' END",
        ),
        ("name", "name"),
        ("brand", "brand"),
        ("amount", "amount"),
        ("unit", "unit"),
        ("kcal", "kcal"),
        ("protein", "protein"),
        ("carbs", "carbs"),
        ("fat", "fat"),
        ("fiber", "fiber"),
        ("sugar", "sugar"),
        ("sodium_mg", "sodium_mg"),
    ];
    let rows =
        select_json(pool, &cols, "FROM food_entries WHERE profile_id = ?1 ORDER BY date, created_at", pid).await?;
    let headers = [
        ("date", "Datum"),
        ("meal", "Mahlzeit"),
        ("name", "Lebensmittel"),
        ("brand", "Marke"),
        ("amount", "Menge"),
        ("unit", "Einheit"),
        ("kcal", "kcal"),
        ("protein", "Eiweiss (g)"),
        ("carbs", "Kohlenhydrate (g)"),
        ("fat", "Fett (g)"),
        ("fiber", "Ballaststoffe (g)"),
        ("sugar", "Zucker (g)"),
        ("sodium_mg", "Natrium (mg)"),
    ];
    Ok(to_csv(&headers, &rows))
}

/// Check-ins: alle Spalten (generisch, auch spätere) ausser `profile_id`.
pub async fn checkins_csv(pool: &SqlitePool, pid: &str) -> AppResult<String> {
    let labels: HashMap<&str, &str> = [
        ("date", "Datum"),
        ("weight", "Gewicht (kg)"),
        ("body_fat", "Körperfett (%)"),
        ("waist", "Taille (cm)"),
        ("hips", "Hüfte (cm)"),
        ("chest", "Brust (cm)"),
        ("neck", "Hals (cm)"),
        ("arm", "Arm (cm)"),
        ("thigh", "Oberschenkel (cm)"),
        ("steps", "Schritte"),
        ("mood", "Stimmung (1–5)"),
        ("energy", "Energie (1–5)"),
        ("stress", "Stress (1–5)"),
        ("sleep_hours", "Schlaf (h)"),
        ("sleep_quality", "Schlafqualität (1–5)"),
        ("resting_hr", "Ruhepuls"),
        ("notes", "Notizen"),
    ]
    .into_iter()
    .collect();
    let cols: Vec<Col> = columns(pool, "checkins").await?.into_iter().filter(|c| c.name != "profile_id").collect();
    let names: Vec<String> = cols.iter().map(|c| c.name.clone()).collect();
    let quoted: Vec<String> = names.iter().map(|n| quote(n)).collect();
    let sql_cols: Vec<(&str, &str)> = names.iter().zip(quoted.iter()).map(|(n, q)| (n.as_str(), q.as_str())).collect();
    let rows = select_json(pool, &sql_cols, "FROM checkins WHERE profile_id = ?1 ORDER BY date", pid).await?;
    let headers: Vec<(&str, &str)> =
        names.iter().map(|n| (n.as_str(), labels.get(n.as_str()).copied().unwrap_or(n.as_str()))).collect();
    Ok(to_csv(&headers, &rows))
}

// ---------------------------------------------------------------------------------------------
// Tests
// ---------------------------------------------------------------------------------------------

#[cfg(test)]
mod tests {
    use std::str::FromStr;

    use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};

    use super::*;

    async fn mem_pool() -> SqlitePool {
        let opts = SqliteConnectOptions::from_str("sqlite::memory:").expect("url").foreign_keys(true);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.expect("pool");
        crate::server::migrations::run(&pool).await.expect("migrate");
        pool
    }

    async fn exec(pool: &SqlitePool, sql: &str) {
        sqlx::query(sqlx::AssertSqlSafe(sql)).execute(pool).await.expect(sql);
    }

    async fn count(pool: &SqlitePool, sql: &str) -> i64 {
        sqlx::query_scalar(sqlx::AssertSqlSafe(sql)).fetch_one(pool).await.expect(sql)
    }

    async fn seed(pool: &SqlitePool) {
        for sql in [
            "INSERT INTO profiles (id, name, created_at) VALUES ('a', 'Anna', '2026-01-01T00:00:00')",
            "INSERT INTO profiles (id, name, created_at) VALUES ('b', 'Ben', '2026-01-01T00:00:00')",
            "INSERT INTO goals (profile_id, calories) VALUES ('a', 1800)",
            "INSERT INTO foods (id, profile_id, name, kcal, created_at) VALUES ('f-shared', NULL, 'Apfel', 52, '2026-01-01T00:00:00')",
            "INSERT INTO foods (id, profile_id, name, kcal, created_at) VALUES ('f-own', 'a', 'Müesli \"Spezial\"', 380, '2026-01-01T00:00:00')",
            "INSERT INTO food_favorites (profile_id, food_id) VALUES ('a', 'f-shared')",
            "INSERT INTO meals (id, profile_id, name, created_at) VALUES ('m1', 'a', 'Zmorge', '2026-01-01T00:00:00')",
            "INSERT INTO meal_items (id, meal_id, food_id, amount) VALUES ('mi1', 'm1', 'f-own', 60)",
            "INSERT INTO food_entries (id, profile_id, date, meal, food_id, name, amount, kcal, protein, created_at)
             VALUES ('e1', 'a', '2026-11-18', 'breakfast', 'f-own', 'Müesli', 60, 228, 6.5, '2026-11-18T08:00:00'),
                    ('e2', 'a', '2026-11-19', 'snack', 'f-shared', 'Apfel', 150, 78, 0.4, '2026-11-19T10:00:00'),
                    ('e3', 'a', '2026-11-20', 'lunch', NULL, 'Pasta; gross', 0, 650, 22, '2026-11-20T12:00:00')",
            "INSERT INTO water_entries (id, profile_id, date, ml, created_at) VALUES ('w1', 'a', '2026-11-20', 500, '2026-11-20T09:00:00')",
            "INSERT INTO exercises (id, profile_id, name, created_at) VALUES ('x1', NULL, 'Laufen', '2026-01-01T00:00:00')",
            "INSERT INTO exercise_entries (id, profile_id, date, exercise_id, name, duration_min, kcal, created_at)
             VALUES ('xe1', 'a', '2026-11-19', 'x1', 'Laufen', 30, 300, '2026-11-19T18:00:00')",
            "INSERT INTO exercise_sets (id, entry_id, reps, weight_kg) VALUES ('s1', 'xe1', 10, 20)",
            "INSERT INTO checkins (profile_id, date, weight, steps, mood, sleep_hours) VALUES ('a', '2026-11-18', 80.5, 9000, 4, 7.5),
                                                                                            ('a', '2026-11-20', 79.9, 6000, 3, 6)",
            "INSERT INTO fasts (id, profile_id, start_at, end_at, target_hours) VALUES ('fa1', 'a', '2026-11-19T20:00:00', '2026-11-20T12:00:00', 16)",
            "INSERT INTO medications (id, profile_id, name, created_at) VALUES ('md1', 'a', 'Vitamin D', '2026-01-01T00:00:00')",
            "INSERT INTO medication_logs (id, medication_id, profile_id, date, slot, taken_at) VALUES ('ml1', 'md1', 'a', '2026-11-20', '08:00', '2026-11-20T08:01:00')",
            "INSERT INTO symptoms (id, profile_id, date, name, created_at) VALUES ('sy1', 'a', '2026-11-20', 'Kopfweh', '2026-11-20T10:00:00')",
            "INSERT INTO cycle_days (profile_id, date, flow) VALUES ('a', '2026-11-10', 2)",
            // Daten eines anderen Profils dürfen nicht im Export landen.
            "INSERT INTO food_entries (id, profile_id, date, meal, name, amount, kcal, created_at) VALUES ('eb', 'b', '2026-11-20', 'lunch', 'Fremd', 1, 999, '2026-11-20T12:00:00')",
        ] {
            exec(pool, sql).await;
        }
    }

    #[tokio::test]
    async fn export_import_roundtrip() {
        let pool = mem_pool().await;
        seed(&pool).await;
        let json = export_json(&pool, "a").await.expect("export");
        let doc: Value = serde_json::from_str(&json).expect("json");
        let t = &doc["tables"];
        assert_eq!(t["food_entries"].as_array().map(Vec::len), Some(3));
        assert_eq!(t["foods"].as_array().map(Vec::len), Some(2), "eigene + verwendete gemeinsame Lebensmittel");
        assert_eq!(t["meal_items"].as_array().map(Vec::len), Some(1));
        assert_eq!(t["exercise_sets"].as_array().map(Vec::len), Some(1));
        assert_eq!(t["checkins"].as_array().map(Vec::len), Some(2));

        // In eine frische Datenbank importieren (anderes Profil) – alles kommt mit.
        let pool2 = mem_pool().await;
        exec(&pool2, "INSERT INTO profiles (id, name, created_at) VALUES ('neu', 'Neu', '2026-11-20T00:00:00')").await;
        let r = import_json(&pool2, "neu", &json).await.expect("import");
        assert!(r.done);
        assert_eq!(r.skipped, 0);
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM food_entries WHERE profile_id = 'neu'").await, 3);
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM meal_items").await, 1);
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM exercise_sets").await, 1);
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM checkins WHERE profile_id = 'neu'").await, 2);
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM medication_logs WHERE profile_id = 'neu'").await, 1);
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM foods WHERE profile_id IS NULL").await, 1);
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM foods WHERE profile_id = 'neu'").await, 1);
        let name: String =
            sqlx::query_scalar("SELECT name FROM profiles WHERE id = 'neu'").fetch_one(&pool2).await.expect("name");
        assert_eq!(name, "Anna");
        let cal: f64 = sqlx::query_scalar("SELECT calories FROM goals WHERE profile_id = 'neu'")
            .fetch_one(&pool2)
            .await
            .expect("goals");
        assert_eq!(cal, 1800.0);

        // Erneuter Import: ersetzt (keine Duplikate), Änderungen werden zurückgesetzt.
        exec(&pool2, "UPDATE food_entries SET kcal = 1 WHERE id = 'e1'").await;
        import_json(&pool2, "neu", &json).await.expect("reimport");
        assert_eq!(count(&pool2, "SELECT COUNT(*) FROM food_entries").await, 3);
        let kcal: f64 =
            sqlx::query_scalar("SELECT kcal FROM food_entries WHERE id = 'e1'").fetch_one(&pool2).await.expect("kcal");
        assert_eq!(kcal, 228.0);

        // Import in Profil b derselben DB: Zeilen von a werden nicht übernommen/verschoben.
        let r = import_json(&pool, "b", &json).await.expect("import b");
        assert!(r.skipped > 0);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM food_entries WHERE profile_id = 'a'").await, 3);
        assert_eq!(count(&pool, "SELECT COUNT(*) FROM meal_items").await, 1);

        // Ungültige Datei
        assert!(import_json(&pool2, "neu", "{\"app\":\"x\"}").await.is_err());
    }

    #[tokio::test]
    async fn report_and_csv() {
        let pool = mem_pool().await;
        seed(&pool).await;
        let d = |s: &str| parse_day(s).expect("date");
        let today = d("2026-11-20");
        let r = report(&pool, "a", d("2026-11-14"), today, today).await.expect("report");
        assert_eq!(r.days.len(), 7);
        let last = &r.days[6];
        assert_eq!(last.kcal, 650.0);
        assert_eq!(last.water_ml, 500.0);
        assert_eq!(last.fasts, 1);
        assert_eq!(last.fasts_reached, 1);
        assert!((last.fast_hours.unwrap_or(0.0) - 16.0).abs() < 0.01);
        assert_eq!(r.summary.food_days, 3);
        assert_eq!(r.summary.workouts, 1);
        assert_eq!(r.summary.weight_change().map(|w| (w * 10.0).round() / 10.0), Some(-0.6));
        assert_eq!(r.streak, 3);
        assert_eq!(r.top_foods_kcal[0].name, "Pasta; gross");
        assert_eq!(r.goals.calories, 1800.0);

        let csv = diary_csv(&pool, "a").await.expect("csv");
        assert!(csv.contains("\"Pasta; gross\""));
        assert!(csv.contains("Frühstück"));
        assert!(!csv.contains("Fremd"));
        let csv = checkins_csv(&pool, "a").await.expect("csv");
        assert!(csv.lines().next().unwrap_or("").contains("Gewicht (kg)"));
        assert!(csv.contains("80.5"));
    }
}

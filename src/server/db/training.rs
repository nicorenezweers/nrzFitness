//! Training: Übungsbibliothek, Einträge mit Sätzen, Vorlagen, Verlauf und Rekorde.

use std::collections::{BTreeMap, HashMap, HashSet};

use chrono::{Duration, NaiveDate};
use sqlx::SqlitePool;

use super::{clean, new_id, now_ts};
use crate::model::{
    DEFAULT_WEIGHT, EntryInput, Exercise, ExerciseEntry, ExerciseInput, Library, PersonalRecord, ProgressPoint, SetRow,
    TemplateItem, TemplateItemInput, TrainingHistory, WeekStat, WorkoutTemplate, epley, estimate_kcal, valid_category,
};
use crate::server::{AppError, AppResult};
use crate::util::week_start;

/// Mitgelieferte Übungen: (ID, Name, Kategorie, Muskelgruppe, Gerät, MET).
const SEED: &[(&str, &str, &str, &str, &str, f64)] = &[
    // Brust
    ("ex-bankdruecken", "Bankdrücken", "strength", "Brust", "Langhantel", 5.0),
    ("ex-schraegbankdruecken", "Schrägbankdrücken", "strength", "Brust", "Langhantel", 5.0),
    ("ex-kh-bankdruecken", "Kurzhantel-Bankdrücken", "strength", "Brust", "Kurzhantel", 5.0),
    ("ex-kh-schraegbank", "Kurzhantel-Schrägbankdrücken", "strength", "Brust", "Kurzhantel", 5.0),
    ("ex-fliegende", "Fliegende", "strength", "Brust", "Kurzhantel", 3.5),
    ("ex-butterfly", "Butterfly", "strength", "Brust", "Maschine", 3.5),
    ("ex-brustpresse", "Brustpresse", "strength", "Brust", "Maschine", 3.5),
    ("ex-kabel-crossover", "Kabelzug-Crossover", "strength", "Brust", "Kabelzug", 3.5),
    ("ex-liegestuetze", "Liegestütze", "strength", "Brust", "Körpergewicht", 3.8),
    ("ex-dips", "Dips", "strength", "Brust", "Körpergewicht", 5.0),
    // Rücken
    ("ex-kreuzheben", "Kreuzheben", "strength", "Rücken", "Langhantel", 6.0),
    ("ex-klimmzuege", "Klimmzüge", "strength", "Rücken", "Körpergewicht", 5.0),
    ("ex-latziehen", "Latziehen", "strength", "Rücken", "Kabelzug", 3.5),
    ("ex-lh-rudern", "Langhantelrudern", "strength", "Rücken", "Langhantel", 5.0),
    ("ex-kh-rudern", "Kurzhantelrudern einarmig", "strength", "Rücken", "Kurzhantel", 4.0),
    ("ex-kabelrudern", "Rudern am Kabel sitzend", "strength", "Rücken", "Kabelzug", 3.5),
    ("ex-tbar-rudern", "T-Bar-Rudern", "strength", "Rücken", "Langhantel", 5.0),
    ("ex-rudermaschine", "Rudermaschine (Kraft)", "strength", "Rücken", "Maschine", 3.5),
    ("ex-hyperextensions", "Rückenstrecker (Hyperextensions)", "strength", "Rücken", "Körpergewicht", 3.5),
    ("ex-ueberzuege", "Überzüge", "strength", "Rücken", "Kurzhantel", 3.5),
    // Beine
    ("ex-kniebeugen", "Kniebeugen", "strength", "Beine", "Langhantel", 6.0),
    ("ex-frontkniebeugen", "Frontkniebeugen", "strength", "Beine", "Langhantel", 6.0),
    ("ex-goblet-squat", "Goblet Squat", "strength", "Beine", "Kettlebell", 5.0),
    ("ex-kniebeugen-kg", "Kniebeugen ohne Gewicht", "strength", "Beine", "Körpergewicht", 5.0),
    ("ex-beinpresse", "Beinpresse", "strength", "Beine", "Maschine", 5.0),
    ("ex-ausfallschritte", "Ausfallschritte", "strength", "Beine", "Kurzhantel", 4.0),
    ("ex-bulgarische-kniebeugen", "Bulgarische Kniebeugen", "strength", "Beine", "Kurzhantel", 4.5),
    ("ex-rumaenisches-kreuzheben", "Rumänisches Kreuzheben", "strength", "Beine", "Langhantel", 5.0),
    ("ex-hip-thrust", "Hip Thrust", "strength", "Beine", "Langhantel", 4.5),
    ("ex-beinstrecker", "Beinstrecker", "strength", "Beine", "Maschine", 3.5),
    ("ex-beinbeuger", "Beinbeuger", "strength", "Beine", "Maschine", 3.5),
    ("ex-wadenheben", "Wadenheben", "strength", "Beine", "Maschine", 3.5),
    ("ex-adduktoren", "Adduktorenmaschine", "strength", "Beine", "Maschine", 3.0),
    // Schultern
    ("ex-schulterdruecken", "Schulterdrücken stehend", "strength", "Schultern", "Langhantel", 5.0),
    ("ex-kh-schulterdruecken", "Kurzhantel-Schulterdrücken", "strength", "Schultern", "Kurzhantel", 4.0),
    ("ex-arnold-press", "Arnold Press", "strength", "Schultern", "Kurzhantel", 4.0),
    ("ex-seitheben", "Seitheben", "strength", "Schultern", "Kurzhantel", 3.5),
    ("ex-frontheben", "Frontheben", "strength", "Schultern", "Kurzhantel", 3.5),
    ("ex-reverse-butterfly", "Reverse Butterfly", "strength", "Schultern", "Maschine", 3.5),
    ("ex-face-pulls", "Face Pulls", "strength", "Schultern", "Kabelzug", 3.5),
    ("ex-shrugs", "Shrugs", "strength", "Schultern", "Kurzhantel", 3.5),
    // Arme
    ("ex-lh-curls", "Langhantel-Curls", "strength", "Arme", "Langhantel", 3.5),
    ("ex-kh-curls", "Kurzhantel-Curls", "strength", "Arme", "Kurzhantel", 3.5),
    ("ex-hammer-curls", "Hammer-Curls", "strength", "Arme", "Kurzhantel", 3.5),
    ("ex-sz-curls", "SZ-Curls", "strength", "Arme", "SZ-Stange", 3.5),
    ("ex-konzentrationscurls", "Konzentrationscurls", "strength", "Arme", "Kurzhantel", 3.0),
    ("ex-trizepsdruecken", "Trizepsdrücken am Kabel", "strength", "Arme", "Kabelzug", 3.5),
    ("ex-french-press", "French Press", "strength", "Arme", "SZ-Stange", 3.5),
    ("ex-trizeps-ueberkopf", "Trizepsstrecken über Kopf", "strength", "Arme", "Kurzhantel", 3.5),
    ("ex-bankdips", "Bankdips", "strength", "Arme", "Körpergewicht", 3.8),
    // Bauch
    ("ex-crunches", "Crunches", "strength", "Bauch", "Körpergewicht", 3.8),
    ("ex-plank", "Unterarmstütz (Plank)", "strength", "Bauch", "Körpergewicht", 3.8),
    ("ex-seitstuetz", "Seitstütz", "strength", "Bauch", "Körpergewicht", 3.8),
    ("ex-beinheben", "Beinheben hängend", "strength", "Bauch", "Körpergewicht", 3.8),
    ("ex-russian-twist", "Russian Twist", "strength", "Bauch", "Körpergewicht", 3.8),
    ("ex-ab-roller", "Ab Roller", "strength", "Bauch", "Gerät", 4.0),
    ("ex-kabel-crunch", "Kabel-Crunch", "strength", "Bauch", "Kabelzug", 3.5),
    // Ganzkörper
    ("ex-kettlebell-swings", "Kettlebell Swings", "strength", "Ganzkörper", "Kettlebell", 8.0),
    ("ex-turkish-getup", "Turkish Get-up", "strength", "Ganzkörper", "Kettlebell", 5.0),
    ("ex-burpees", "Burpees", "strength", "Ganzkörper", "Körpergewicht", 8.0),
    ("ex-mountain-climbers", "Mountain Climbers", "strength", "Ganzkörper", "Körpergewicht", 8.0),
    ("ex-thruster", "Thruster", "strength", "Ganzkörper", "Langhantel", 6.0),
    ("ex-umsetzen", "Umsetzen (Power Clean)", "strength", "Ganzkörper", "Langhantel", 6.0),
    ("ex-farmers-walk", "Farmer's Walk", "strength", "Ganzkörper", "Kurzhantel", 6.0),
    ("ex-zirkeltraining", "Zirkeltraining", "strength", "Ganzkörper", "Ohne", 8.0),
    // Cardio
    ("ex-laufen", "Laufen", "cardio", "Cardio", "Ohne", 8.3),
    ("ex-joggen", "Joggen locker", "cardio", "Cardio", "Ohne", 7.0),
    ("ex-laufband", "Laufband", "cardio", "Cardio", "Gerät", 8.0),
    ("ex-gehen", "Spazieren", "cardio", "Cardio", "Ohne", 3.5),
    ("ex-walking", "Walking zügig", "cardio", "Cardio", "Ohne", 4.3),
    ("ex-nordic-walking", "Nordic Walking", "cardio", "Cardio", "Gerät", 4.8),
    ("ex-wandern", "Wandern", "cardio", "Cardio", "Ohne", 6.0),
    ("ex-radfahren", "Radfahren", "cardio", "Cardio", "Gerät", 7.5),
    ("ex-mountainbike", "Mountainbike", "cardio", "Cardio", "Gerät", 8.5),
    ("ex-ergometer", "Fahrradergometer", "cardio", "Cardio", "Gerät", 7.0),
    ("ex-spinning", "Indoor-Cycling (Spinning)", "cardio", "Cardio", "Gerät", 8.5),
    ("ex-schwimmen", "Schwimmen", "cardio", "Cardio", "Ohne", 7.0),
    ("ex-rudergeraet", "Rudergerät", "cardio", "Cardio", "Gerät", 7.0),
    ("ex-crosstrainer", "Crosstrainer", "cardio", "Cardio", "Gerät", 5.0),
    ("ex-stepper", "Stepper / Treppensteigen", "cardio", "Cardio", "Gerät", 9.0),
    ("ex-seilspringen", "Seilspringen", "cardio", "Cardio", "Gerät", 11.0),
    ("ex-hiit", "HIIT", "cardio", "Cardio", "Ohne", 8.0),
    ("ex-inline", "Inline-Skaten", "cardio", "Cardio", "Gerät", 7.5),
    ("ex-langlauf", "Langlauf", "cardio", "Cardio", "Gerät", 9.0),
    // Dehnen / Beweglichkeit
    ("ex-dehnen", "Dehnen", "flexibility", "Ganzkörper", "Ohne", 2.3),
    ("ex-yoga", "Yoga", "flexibility", "Ganzkörper", "Ohne", 2.5),
    ("ex-power-yoga", "Power Yoga", "flexibility", "Ganzkörper", "Ohne", 4.0),
    ("ex-pilates", "Pilates", "flexibility", "Bauch", "Ohne", 3.0),
    ("ex-mobility", "Mobility", "flexibility", "Ganzkörper", "Ohne", 2.5),
    ("ex-faszienrolle", "Faszienrolle", "flexibility", "Ganzkörper", "Gerät", 2.0),
    // Sport
    ("ex-fussball", "Fussball", "sport", "Ganzkörper", "Ohne", 7.0),
    ("ex-unihockey", "Unihockey", "sport", "Ganzkörper", "Gerät", 7.0),
    ("ex-basketball", "Basketball", "sport", "Ganzkörper", "Ohne", 6.5),
    ("ex-handball", "Handball", "sport", "Ganzkörper", "Ohne", 8.0),
    ("ex-volleyball", "Volleyball", "sport", "Ganzkörper", "Ohne", 4.0),
    ("ex-eishockey", "Eishockey", "sport", "Ganzkörper", "Gerät", 8.0),
    ("ex-tennis", "Tennis", "sport", "Ganzkörper", "Gerät", 7.3),
    ("ex-badminton", "Badminton", "sport", "Ganzkörper", "Gerät", 5.5),
    ("ex-squash", "Squash", "sport", "Ganzkörper", "Gerät", 7.3),
    ("ex-tischtennis", "Tischtennis", "sport", "Ganzkörper", "Gerät", 4.0),
    ("ex-bouldern", "Bouldern", "sport", "Ganzkörper", "Ohne", 5.8),
    ("ex-klettern", "Klettern", "sport", "Ganzkörper", "Gerät", 7.5),
    ("ex-boxen", "Boxen (Sandsack)", "sport", "Ganzkörper", "Gerät", 5.5),
    ("ex-kampfsport", "Kampfsport", "sport", "Ganzkörper", "Ohne", 7.0),
    ("ex-tanzen", "Tanzen", "sport", "Ganzkörper", "Ohne", 5.0),
    ("ex-golf", "Golf", "sport", "Ganzkörper", "Gerät", 4.8),
    ("ex-skifahren", "Skifahren", "sport", "Ganzkörper", "Gerät", 5.3),
    ("ex-snowboarden", "Snowboarden", "sport", "Ganzkörper", "Gerät", 5.3),
    ("ex-sup", "Stand-up-Paddling", "sport", "Ganzkörper", "Gerät", 6.0),
];

/// Legt die mitgelieferte Übungsbibliothek an (idempotent).
pub async fn seed_exercises(pool: &SqlitePool) -> Result<(), sqlx::Error> {
    let now = now_ts();
    let mut tx = pool.begin().await?;
    for (id, name, cat, muscle, equipment, met) in SEED {
        sqlx::query(
            "INSERT OR IGNORE INTO exercises (id, profile_id, name, category, muscle, equipment, met, created_at)
             VALUES (?, NULL, ?, ?, ?, ?, ?, ?)",
        )
        .bind(id)
        .bind(name)
        .bind(cat)
        .bind(muscle)
        .bind(equipment)
        .bind(met)
        .bind(&now)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await
}

// --- Übungen ----------------------------------------------------------------

const EX_COLS: &str = "x.id, x.name, x.category, x.muscle, x.equipment, x.met, (x.profile_id IS NOT NULL) AS custom";

pub async fn exercises(pool: &SqlitePool, pid: &str) -> AppResult<Vec<Exercise>> {
    Ok(sqlx::query_as::<_, Exercise>(&format!(
        "SELECT {EX_COLS} FROM exercises x WHERE x.profile_id IS NULL OR x.profile_id = ? ORDER BY x.name COLLATE NOCASE"
    ))
    .bind(pid)
    .fetch_all(pool)
    .await?)
}

/// Zuletzt verwendete Übungen (neueste zuerst).
pub async fn recent(pool: &SqlitePool, pid: &str, limit: i64) -> AppResult<Vec<Exercise>> {
    Ok(sqlx::query_as::<_, Exercise>(&format!(
        "SELECT {EX_COLS} FROM exercises x
         JOIN (SELECT exercise_id, MAX(date || ' ' || created_at) AS last FROM exercise_entries
               WHERE profile_id = ?1 AND exercise_id IS NOT NULL GROUP BY exercise_id) r ON r.exercise_id = x.id
         WHERE x.profile_id IS NULL OR x.profile_id = ?1
         ORDER BY r.last DESC LIMIT ?2"
    ))
    .bind(pid)
    .bind(limit)
    .fetch_all(pool)
    .await?)
}

pub async fn get_exercise(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<Exercise> {
    sqlx::query_as::<_, Exercise>(&format!(
        "SELECT {EX_COLS} FROM exercises x WHERE x.id = ? AND (x.profile_id IS NULL OR x.profile_id = ?)"
    ))
    .bind(id)
    .bind(pid)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Übung nicht gefunden".into()))
}

/// Körpergewicht für die kcal-Schätzung (letztes erfasstes oder Annahme).
pub async fn body_weight(pool: &SqlitePool, pid: &str) -> AppResult<(f64, bool)> {
    Ok(match super::profile::latest_weight(pool, pid).await?.filter(|w| *w > 0.0) {
        Some(w) => (w, true),
        None => (DEFAULT_WEIGHT, false),
    })
}

pub async fn library(pool: &SqlitePool, pid: &str) -> AppResult<Library> {
    let (weight, weight_known) = body_weight(pool, pid).await?;
    Ok(Library { exercises: exercises(pool, pid).await?, recent: recent(pool, pid, 8).await?, weight, weight_known })
}

fn check_input(input: &ExerciseInput) -> AppResult<(String, f64)> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name fehlt".into()));
    }
    let met = if input.met.is_finite() && input.met > 0.0 { input.met.min(25.0) } else { 5.0 };
    Ok((name, met))
}

pub async fn create_exercise(pool: &SqlitePool, pid: &str, input: ExerciseInput) -> AppResult<Exercise> {
    let (name, met) = check_input(&input)?;
    let id = new_id();
    sqlx::query(
        "INSERT INTO exercises (id, profile_id, name, category, muscle, equipment, met, created_at) VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
    )
    .bind(&id)
    .bind(pid)
    .bind(name)
    .bind(valid_category(&input.category))
    .bind(clean(input.muscle))
    .bind(clean(input.equipment))
    .bind(met)
    .bind(now_ts())
    .execute(pool)
    .await?;
    get_exercise(pool, pid, &id).await
}

/// Nur eigene Übungen sind änderbar.
pub async fn update_exercise(pool: &SqlitePool, pid: &str, id: &str, input: ExerciseInput) -> AppResult<Exercise> {
    let (name, met) = check_input(&input)?;
    let r = sqlx::query("UPDATE exercises SET name = ?, category = ?, muscle = ?, equipment = ?, met = ? WHERE id = ? AND profile_id = ?")
        .bind(name)
        .bind(valid_category(&input.category))
        .bind(clean(input.muscle))
        .bind(clean(input.equipment))
        .bind(met)
        .bind(id)
        .bind(pid)
        .execute(pool)
        .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::BadRequest("Nur eigene Übungen können bearbeitet werden".into()));
    }
    get_exercise(pool, pid, id).await
}

/// Eigene Übung löschen. Einträge bleiben (mit Namen) erhalten.
pub async fn delete_exercise(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    let r =
        sqlx::query("DELETE FROM exercises WHERE id = ? AND profile_id = ?").bind(id).bind(pid).execute(pool).await?;
    if r.rows_affected() == 0 {
        return Err(AppError::BadRequest("Nur eigene Übungen können gelöscht werden".into()));
    }
    Ok(())
}

// --- Einträge ---------------------------------------------------------------

const ENTRY_COLS: &str = "id, date, exercise_id, name, category, duration_min, kcal, distance_km, notes";

/// Sätze zu den Einträgen laden (eine Abfrage).
async fn attach_sets(pool: &SqlitePool, entries: &mut [ExerciseEntry]) -> AppResult<()> {
    if entries.is_empty() {
        return Ok(());
    }
    #[derive(sqlx::FromRow)]
    struct Row {
        entry_id: String,
        id: String,
        reps: i64,
        weight_kg: f64,
        done: bool,
    }
    let marks = vec!["?"; entries.len()].join(",");
    let sql = format!(
        "SELECT entry_id, id, reps, weight_kg, done FROM exercise_sets WHERE entry_id IN ({marks}) ORDER BY entry_id, position"
    );
    let mut q = sqlx::query_as::<_, Row>(&sql);
    for e in entries.iter() {
        q = q.bind(&e.id);
    }
    let mut map: HashMap<String, Vec<SetRow>> = HashMap::new();
    for r in q.fetch_all(pool).await? {
        map.entry(r.entry_id).or_default().push(SetRow {
            id: r.id,
            reps: r.reps,
            weight_kg: r.weight_kg,
            done: r.done,
        });
    }
    for e in entries.iter_mut() {
        e.sets = map.remove(&e.id).unwrap_or_default();
    }
    Ok(())
}

pub async fn entries(pool: &SqlitePool, pid: &str, date: NaiveDate) -> AppResult<Vec<ExerciseEntry>> {
    let mut rows = sqlx::query_as::<_, ExerciseEntry>(&format!(
        "SELECT {ENTRY_COLS} FROM exercise_entries WHERE profile_id = ? AND date = ? ORDER BY created_at, id"
    ))
    .bind(pid)
    .bind(date)
    .fetch_all(pool)
    .await?;
    attach_sets(pool, &mut rows).await?;
    Ok(rows)
}

pub async fn get_entry(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<ExerciseEntry> {
    let e = sqlx::query_as::<_, ExerciseEntry>(&format!(
        "SELECT {ENTRY_COLS} FROM exercise_entries WHERE id = ? AND profile_id = ?"
    ))
    .bind(id)
    .bind(pid)
    .fetch_optional(pool)
    .await?
    .ok_or_else(|| AppError::NotFound("Eintrag nicht gefunden".into()))?;
    let mut v = [e];
    attach_sets(pool, &mut v).await?;
    let [e] = v;
    Ok(e)
}

/// Letzter Eintrag einer Übung (für „Letztes Mal“), optional ohne den gerade bearbeiteten.
pub async fn last_entry(
    pool: &SqlitePool,
    pid: &str,
    exercise_id: &str,
    exclude: Option<String>,
) -> AppResult<Option<ExerciseEntry>> {
    let e = sqlx::query_as::<_, ExerciseEntry>(&format!(
        "SELECT {ENTRY_COLS} FROM exercise_entries WHERE profile_id = ? AND exercise_id = ? AND (? IS NULL OR id <> ?)
         ORDER BY date DESC, created_at DESC LIMIT 1"
    ))
    .bind(pid)
    .bind(exercise_id)
    .bind(&exclude)
    .bind(&exclude)
    .fetch_optional(pool)
    .await?;
    let Some(e) = e else { return Ok(None) };
    let mut v = [e];
    attach_sets(pool, &mut v).await?;
    let [e] = v;
    Ok(Some(e))
}

fn clean_sets(sets: Vec<SetRow>) -> Vec<SetRow> {
    sets.into_iter()
        .map(|s| SetRow {
            id: s.id,
            reps: s.reps.clamp(0, 10_000),
            weight_kg: if s.weight_kg.is_finite() { s.weight_kg.clamp(0.0, 2_000.0) } else { 0.0 },
            done: s.done,
        })
        .filter(|s| s.reps > 0 || s.weight_kg > 0.0)
        .collect()
}

fn pos(v: f64) -> f64 {
    if v.is_finite() { v.max(0.0) } else { 0.0 }
}

/// Eintrag anlegen (`id` = None) oder ersetzen; Sätze werden komplett neu geschrieben.
pub async fn save_entry(
    pool: &SqlitePool,
    pid: &str,
    id: Option<String>,
    input: EntryInput,
) -> AppResult<ExerciseEntry> {
    let name = input.name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name fehlt".into()));
    }
    let category = valid_category(&input.category);
    let sets = if category == "strength" { clean_sets(input.sets) } else { Vec::new() };
    let distance = input.distance_km.filter(|d| d.is_finite() && *d > 0.0);
    let mut tx = pool.begin().await?;
    let id = match id {
        Some(id) => {
            let r = sqlx::query(
                "UPDATE exercise_entries SET date = ?, exercise_id = ?, name = ?, category = ?, duration_min = ?, kcal = ?,
                        distance_km = ?, notes = ?
                 WHERE id = ? AND profile_id = ?",
            )
            .bind(input.date)
            .bind(&input.exercise_id)
            .bind(&name)
            .bind(category)
            .bind(pos(input.duration_min))
            .bind(pos(input.kcal))
            .bind(distance)
            .bind(clean(input.notes))
            .bind(&id)
            .bind(pid)
            .execute(&mut *tx)
            .await?;
            if r.rows_affected() == 0 {
                return Err(AppError::NotFound("Eintrag nicht gefunden".into()));
            }
            sqlx::query("DELETE FROM exercise_sets WHERE entry_id = ?").bind(&id).execute(&mut *tx).await?;
            id
        }
        None => {
            let id = new_id();
            sqlx::query(
                "INSERT INTO exercise_entries (id, profile_id, date, exercise_id, name, category, duration_min, kcal, distance_km,
                                               notes, created_at)
                 VALUES (?, ?, ?, ?, ?, ?, ?, ?, ?, ?, ?)",
            )
            .bind(&id)
            .bind(pid)
            .bind(input.date)
            .bind(&input.exercise_id)
            .bind(&name)
            .bind(category)
            .bind(pos(input.duration_min))
            .bind(pos(input.kcal))
            .bind(distance)
            .bind(clean(input.notes))
            .bind(now_ts())
            .execute(&mut *tx)
            .await?;
            id
        }
    };
    for (i, s) in sets.iter().enumerate() {
        sqlx::query(
            "INSERT INTO exercise_sets (id, entry_id, position, reps, weight_kg, done) VALUES (?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(&id)
        .bind(i as i64)
        .bind(s.reps)
        .bind(s.weight_kg)
        .bind(s.done)
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    get_entry(pool, pid, &id).await
}

pub async fn delete_entry(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM exercise_entries WHERE id = ? AND profile_id = ?")
        .bind(id)
        .bind(pid)
        .execute(pool)
        .await?;
    Ok(())
}

/// Satz abhaken / wieder öffnen (direkt aus der Tagesansicht).
pub async fn set_done(pool: &SqlitePool, pid: &str, set_id: &str, done: bool) -> AppResult<()> {
    let r = sqlx::query(
        "UPDATE exercise_sets SET done = ? WHERE id = ? AND entry_id IN (SELECT id FROM exercise_entries WHERE profile_id = ?)",
    )
    .bind(done)
    .bind(set_id)
    .bind(pid)
    .execute(pool)
    .await?;
    if r.rows_affected() == 0 {
        return Err(AppError::NotFound("Satz nicht gefunden".into()));
    }
    Ok(())
}

// --- Vorlagen ---------------------------------------------------------------

async fn template_items(pool: &SqlitePool, pid: &str, template_id: &str) -> AppResult<Vec<TemplateItem>> {
    #[derive(sqlx::FromRow)]
    struct Row {
        exercise_id: String,
        sets: i64,
        reps: i64,
        weight_kg: f64,
        duration_min: f64,
    }
    let rows = sqlx::query_as::<_, Row>(
        "SELECT exercise_id, sets, reps, weight_kg, duration_min FROM workout_template_items WHERE template_id = ? ORDER BY position",
    )
    .bind(template_id)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for r in rows {
        if let Ok(exercise) = get_exercise(pool, pid, &r.exercise_id).await {
            out.push(TemplateItem {
                exercise,
                sets: r.sets,
                reps: r.reps,
                weight_kg: r.weight_kg,
                duration_min: r.duration_min,
            });
        }
    }
    Ok(out)
}

pub async fn templates(pool: &SqlitePool, pid: &str) -> AppResult<Vec<WorkoutTemplate>> {
    let rows: Vec<(String, String, Option<String>)> = sqlx::query_as(
        "SELECT id, name, notes FROM workout_templates WHERE profile_id = ? ORDER BY name COLLATE NOCASE",
    )
    .bind(pid)
    .fetch_all(pool)
    .await?;
    let mut out = Vec::with_capacity(rows.len());
    for (id, name, notes) in rows {
        let items = template_items(pool, pid, &id).await?;
        out.push(WorkoutTemplate { id, name, notes, items });
    }
    Ok(out)
}

pub async fn template(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<WorkoutTemplate> {
    let (id, name, notes): (String, String, Option<String>) =
        sqlx::query_as("SELECT id, name, notes FROM workout_templates WHERE id = ? AND profile_id = ?")
            .bind(id)
            .bind(pid)
            .fetch_optional(pool)
            .await?
            .ok_or_else(|| AppError::NotFound("Vorlage nicht gefunden".into()))?;
    let items = template_items(pool, pid, &id).await?;
    Ok(WorkoutTemplate { id, name, notes, items })
}

/// Vorlage anlegen (`id` = None) oder ersetzen.
pub async fn save_template(
    pool: &SqlitePool,
    pid: &str,
    id: Option<String>,
    name: String,
    notes: Option<String>,
    items: Vec<TemplateItemInput>,
) -> AppResult<WorkoutTemplate> {
    let name = name.trim().to_string();
    if name.is_empty() {
        return Err(AppError::BadRequest("Name fehlt".into()));
    }
    if items.is_empty() {
        return Err(AppError::BadRequest("Mindestens eine Übung hinzufügen".into()));
    }
    let mut tx = pool.begin().await?;
    let id = match id {
        Some(id) => {
            let r = sqlx::query("UPDATE workout_templates SET name = ?, notes = ? WHERE id = ? AND profile_id = ?")
                .bind(&name)
                .bind(clean(notes))
                .bind(&id)
                .bind(pid)
                .execute(&mut *tx)
                .await?;
            if r.rows_affected() == 0 {
                return Err(AppError::NotFound("Vorlage nicht gefunden".into()));
            }
            sqlx::query("DELETE FROM workout_template_items WHERE template_id = ?").bind(&id).execute(&mut *tx).await?;
            id
        }
        None => {
            let id = new_id();
            sqlx::query(
                "INSERT INTO workout_templates (id, profile_id, name, notes, created_at) VALUES (?, ?, ?, ?, ?)",
            )
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
    for (i, it) in items.into_iter().enumerate() {
        sqlx::query(
            "INSERT INTO workout_template_items (id, template_id, exercise_id, position, sets, reps, weight_kg, duration_min)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)",
        )
        .bind(new_id())
        .bind(&id)
        .bind(&it.exercise_id)
        .bind(i as i64)
        .bind(it.sets.clamp(1, 50))
        .bind(it.reps.clamp(0, 1000))
        .bind(pos(it.weight_kg))
        .bind(pos(it.duration_min))
        .execute(&mut *tx)
        .await?;
    }
    tx.commit().await?;
    template(pool, pid, &id).await
}

pub async fn delete_template(pool: &SqlitePool, pid: &str, id: &str) -> AppResult<()> {
    sqlx::query("DELETE FROM workout_templates WHERE id = ? AND profile_id = ?")
        .bind(id)
        .bind(pid)
        .execute(pool)
        .await?;
    Ok(())
}

/// Vorlage für einen Tag eintragen. Kraft-Sätze kommen offen (zum Abhaken), Gewicht ohne Vorgabe vom letzten Mal.
pub async fn apply_template(pool: &SqlitePool, pid: &str, id: &str, date: NaiveDate) -> AppResult<usize> {
    let tpl = template(pool, pid, id).await?;
    let (body, _) = body_weight(pool, pid).await?;
    for it in &tpl.items {
        let ex = &it.exercise;
        let last = last_entry(pool, pid, &ex.id, None).await?;
        let input = if ex.is_strength() {
            let kg = if it.weight_kg > 0.0 {
                it.weight_kg
            } else {
                last.as_ref().and_then(|l| l.sets.iter().map(|s| s.weight_kg).reduce(f64::max)).unwrap_or(0.0)
            };
            let n = it.sets.max(1) as usize;
            let sets = (0..n)
                .map(|_| SetRow { id: String::new(), reps: it.reps.max(1), weight_kg: kg, done: false })
                .collect();
            EntryInput {
                date,
                exercise_id: Some(ex.id.clone()),
                name: ex.name.clone(),
                category: ex.category.clone(),
                duration_min: 0.0,
                kcal: estimate_kcal(ex.met, body, 0.0, n),
                distance_km: None,
                notes: None,
                sets,
            }
        } else {
            let dur = if it.duration_min > 0.0 {
                it.duration_min
            } else {
                last.as_ref().map(|l| l.duration_min).unwrap_or(30.0)
            };
            EntryInput {
                date,
                exercise_id: Some(ex.id.clone()),
                name: ex.name.clone(),
                category: ex.category.clone(),
                duration_min: dur,
                kcal: estimate_kcal(ex.met, body, dur, 0),
                distance_km: None,
                notes: None,
                sets: Vec::new(),
            }
        };
        save_entry(pool, pid, None, input).await?;
    }
    Ok(tpl.items.len())
}

/// Aus den Einträgen eines Tages eine Vorlage machen.
pub async fn template_from_day(
    pool: &SqlitePool,
    pid: &str,
    date: NaiveDate,
    name: String,
) -> AppResult<WorkoutTemplate> {
    let items: Vec<TemplateItemInput> = entries(pool, pid, date)
        .await?
        .into_iter()
        .filter_map(|e| {
            let exercise_id = e.exercise_id.clone()?;
            Some(TemplateItemInput {
                exercise_id,
                sets: e.sets.len().max(1) as i64,
                reps: e.sets.first().map(|s| s.reps).unwrap_or(10),
                weight_kg: e.sets.iter().map(|s| s.weight_kg).fold(0.0, f64::max),
                duration_min: if e.is_strength() { 0.0 } else { e.duration_min },
            })
        })
        .collect();
    if items.is_empty() {
        return Err(AppError::BadRequest("Keine Übungen aus der Bibliothek an diesem Tag".into()));
    }
    save_template(pool, pid, None, name, None, items).await
}

// --- Verlauf & Rekorde ------------------------------------------------------

/// Wochenübersicht (letzte `weeks` Wochen bis inkl. der Woche von `today`) und Serien.
pub async fn history(pool: &SqlitePool, pid: &str, today: NaiveDate, weeks: i64) -> AppResult<TrainingHistory> {
    let days: Vec<(NaiveDate, f64, f64)> = sqlx::query_as(
        "SELECT date, IFNULL(SUM(duration_min), 0.0), IFNULL(SUM(kcal), 0.0) FROM exercise_entries
         WHERE profile_id = ? GROUP BY date ORDER BY date",
    )
    .bind(pid)
    .fetch_all(pool)
    .await?;
    let vol: Vec<(NaiveDate, f64)> = sqlx::query_as(
        "SELECT e.date, IFNULL(SUM(s.reps * s.weight_kg), 0.0) FROM exercise_sets s JOIN exercise_entries e ON e.id = s.entry_id
         WHERE e.profile_id = ? AND s.done = 1 GROUP BY e.date",
    )
    .bind(pid)
    .fetch_all(pool)
    .await?;
    let vol: HashMap<NaiveDate, f64> = vol.into_iter().collect();

    let weeks = weeks.clamp(1, 104);
    let first = week_start(today) - Duration::weeks(weeks - 1);
    let mut out: Vec<WeekStat> =
        (0..weeks).map(|i| WeekStat { start: first + Duration::weeks(i), ..Default::default() }).collect();
    for (d, min, kcal) in &days {
        if *d < first || *d > today {
            continue;
        }
        let idx = ((week_start(*d) - first).num_days() / 7) as usize;
        if let Some(w) = out.get_mut(idx) {
            w.minutes += min;
            w.kcal += kcal;
            w.volume += vol.get(d).copied().unwrap_or(0.0);
            w.days += 1;
        }
    }

    let set: HashSet<NaiveDate> = days.iter().map(|d| d.0).filter(|d| *d <= today).collect();
    let mut streak = 0;
    let mut cur =
        if set.contains(&today) { Some(today) } else { Some(today - Duration::days(1)).filter(|d| set.contains(d)) };
    while let Some(d) = cur.filter(|d| set.contains(d)) {
        streak += 1;
        cur = Some(d - Duration::days(1));
    }
    let mut sorted: Vec<NaiveDate> = set.iter().copied().collect();
    sorted.sort();
    let (mut best, mut run) = (0i64, 0i64);
    for (i, d) in sorted.iter().enumerate() {
        run = if i > 0 && (*d - sorted[i - 1]).num_days() == 1 { run + 1 } else { 1 };
        best = best.max(run);
    }
    let since = today - Duration::days(29);
    Ok(TrainingHistory {
        weeks: out,
        streak,
        best_streak: best,
        days_30: sorted.iter().filter(|d| **d >= since).count() as i64,
        total_days: sorted.len() as i64,
        last_date: sorted.last().copied(),
    })
}

#[derive(sqlx::FromRow)]
struct HistRow {
    id: String,
    exercise_id: String,
    name: String,
    category: String,
    date: NaiveDate,
    duration_min: f64,
    distance_km: f64,
}

async fn hist_rows(
    pool: &SqlitePool,
    pid: &str,
    exercise_id: Option<&str>,
) -> AppResult<(Vec<HistRow>, HashMap<String, Vec<(i64, f64)>>)> {
    let rows = sqlx::query_as::<_, HistRow>(
        "SELECT id, exercise_id, name, category, date, duration_min, IFNULL(distance_km, 0.0) AS distance_km
         FROM exercise_entries WHERE profile_id = ? AND exercise_id IS NOT NULL AND (? IS NULL OR exercise_id = ?)
         ORDER BY date, created_at",
    )
    .bind(pid)
    .bind(exercise_id)
    .bind(exercise_id)
    .fetch_all(pool)
    .await?;
    let sets: Vec<(String, i64, f64)> = sqlx::query_as(
        "SELECT s.entry_id, s.reps, s.weight_kg FROM exercise_sets s JOIN exercise_entries e ON e.id = s.entry_id
         WHERE e.profile_id = ? AND s.done = 1 AND e.exercise_id IS NOT NULL AND (? IS NULL OR e.exercise_id = ?)",
    )
    .bind(pid)
    .bind(exercise_id)
    .bind(exercise_id)
    .fetch_all(pool)
    .await?;
    let mut map: HashMap<String, Vec<(i64, f64)>> = HashMap::new();
    for (eid, reps, kg) in sets {
        map.entry(eid).or_default().push((reps, kg));
    }
    Ok((rows, map))
}

/// Persönliche Rekorde pro Übung (zuletzt trainierte zuerst).
pub async fn records(pool: &SqlitePool, pid: &str) -> AppResult<Vec<PersonalRecord>> {
    let (rows, sets) = hist_rows(pool, pid, None).await?;
    let mut by_ex: HashMap<String, PersonalRecord> = HashMap::new();
    let mut days: HashMap<String, HashSet<NaiveDate>> = HashMap::new();
    for r in rows {
        let pr = by_ex
            .entry(r.exercise_id.clone())
            .or_insert_with(|| PersonalRecord { exercise_id: r.exercise_id.clone(), ..Default::default() });
        // Name/Kategorie vom neuesten Eintrag (Zeilen sind nach Datum sortiert).
        pr.name = r.name.clone();
        pr.category = r.category.clone();
        pr.last_date = Some(r.date);
        days.entry(r.exercise_id.clone()).or_default().insert(r.date);
        pr.longest_min = pr.longest_min.max(r.duration_min);
        pr.longest_km = pr.longest_km.max(r.distance_km);
        let mut volume = 0.0;
        for (reps, kg) in sets.get(&r.id).map(|v| v.as_slice()).unwrap_or(&[]) {
            volume += *reps as f64 * kg;
            if *kg > pr.heaviest_kg || (*kg == pr.heaviest_kg && *reps > pr.heaviest_reps) {
                pr.heaviest_kg = *kg;
                pr.heaviest_reps = *reps;
            }
            let e = epley(*kg, *reps);
            if e > pr.best_e1rm {
                pr.best_e1rm = e;
                pr.best_e1rm_date = Some(r.date);
            }
            pr.most_reps = pr.most_reps.max(*reps);
        }
        pr.best_volume = pr.best_volume.max(volume);
    }
    let mut out: Vec<PersonalRecord> = by_ex
        .into_values()
        .map(|mut p| {
            p.sessions = days.get(&p.exercise_id).map(|d| d.len() as i64).unwrap_or(0);
            p
        })
        .collect();
    out.sort_by(|a, b| b.last_date.cmp(&a.last_date).then_with(|| a.name.cmp(&b.name)));
    Ok(out)
}

/// Fortschritt einer Übung pro Trainingstag.
pub async fn progress(pool: &SqlitePool, pid: &str, exercise_id: &str) -> AppResult<Vec<ProgressPoint>> {
    let (rows, sets) = hist_rows(pool, pid, Some(exercise_id)).await?;
    let mut by_day: BTreeMap<NaiveDate, ProgressPoint> = BTreeMap::new();
    for r in rows {
        let p = by_day.entry(r.date).or_insert_with(|| ProgressPoint { date: r.date, ..Default::default() });
        p.duration_min += r.duration_min;
        p.distance_km += r.distance_km;
        for (reps, kg) in sets.get(&r.id).map(|v| v.as_slice()).unwrap_or(&[]) {
            p.volume += *reps as f64 * kg;
            p.top_kg = p.top_kg.max(*kg);
            p.e1rm = p.e1rm.max(epley(*kg, *reps));
        }
    }
    Ok(by_day.into_values().collect())
}

#[cfg(test)]
mod tests {
    use super::*;

    async fn pool() -> SqlitePool {
        use std::str::FromStr;

        use sqlx::sqlite::{SqliteConnectOptions, SqlitePoolOptions};
        let opts = SqliteConnectOptions::from_str("sqlite::memory:").unwrap().foreign_keys(true);
        let pool = SqlitePoolOptions::new().max_connections(1).connect_with(opts).await.unwrap();
        crate::server::migrations::run(&pool).await.unwrap();
        seed_exercises(&pool).await.unwrap();
        seed_exercises(&pool).await.unwrap(); // idempotent
        sqlx::query("INSERT INTO profiles (id, name, created_at) VALUES ('p1', 'Test', '2026-01-01T00:00:00')")
            .execute(&pool)
            .await
            .unwrap();
        pool
    }

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn set(reps: i64, kg: f64, done: bool) -> SetRow {
        SetRow { id: String::new(), reps, weight_kg: kg, done }
    }

    fn bench(date: NaiveDate, sets: Vec<SetRow>) -> EntryInput {
        EntryInput {
            date,
            exercise_id: Some("ex-bankdruecken".into()),
            name: "Bankdrücken".into(),
            category: "strength".into(),
            kcal: 50.0,
            sets,
            ..Default::default()
        }
    }

    #[tokio::test]
    async fn library_entries_records() {
        let p = pool().await;
        let lib = library(&p, "p1").await.unwrap();
        assert!(lib.exercises.len() >= 80, "{}", lib.exercises.len());
        assert_eq!(lib.weight, DEFAULT_WEIGHT);
        assert!(lib.recent.is_empty());

        let custom = create_exercise(
            &p,
            "p1",
            ExerciseInput { name: "Meine Übung".into(), category: "cardio".into(), met: 6.0, ..Default::default() },
        )
        .await
        .unwrap();
        assert!(custom.custom);
        assert!(
            update_exercise(&p, "p1", "ex-bankdruecken", ExerciseInput { name: "x".into(), ..Default::default() })
                .await
                .is_err()
        );

        let e1 = save_entry(
            &p,
            "p1",
            None,
            bench(d("2026-11-16"), vec![set(10, 60.0, true), set(8, 70.0, true), set(0, 0.0, true)]),
        )
        .await
        .unwrap();
        assert_eq!(e1.sets.len(), 2);
        assert_eq!(e1.volume(), 1160.0);
        let e2 = save_entry(&p, "p1", None, bench(d("2026-11-17"), vec![set(5, 80.0, true), set(12, 50.0, false)]))
            .await
            .unwrap();
        let last = last_entry(&p, "p1", "ex-bankdruecken", None).await.unwrap().unwrap();
        assert_eq!(last.id, e2.id);
        let before = last_entry(&p, "p1", "ex-bankdruecken", Some(e2.id.clone())).await.unwrap().unwrap();
        assert_eq!(before.id, e1.id);

        let recs = records(&p, "p1").await.unwrap();
        assert_eq!(recs.len(), 1);
        assert_eq!(recs[0].heaviest_kg, 80.0);
        assert_eq!(recs[0].most_reps, 10); // 12 ist nicht erledigt
        assert!((recs[0].best_e1rm - 93.333).abs() < 0.01);
        assert_eq!(recs[0].sessions, 2);

        set_done(&p, "p1", &e2.sets[1].id, true).await.unwrap();
        assert_eq!(records(&p, "p1").await.unwrap()[0].most_reps, 12);

        let prog = progress(&p, "p1", "ex-bankdruecken").await.unwrap();
        assert_eq!(prog.len(), 2);
        assert_eq!(prog[0].top_kg, 70.0);

        let h = history(&p, "p1", d("2026-11-17"), 4).await.unwrap();
        assert_eq!(h.weeks.len(), 4);
        assert_eq!(h.streak, 2);
        assert_eq!(h.best_streak, 2);
        assert_eq!(h.weeks[3].days, 2);
        assert_eq!(history(&p, "p1", d("2026-11-19"), 4).await.unwrap().streak, 0);

        assert_eq!(recent(&p, "p1", 5).await.unwrap()[0].id, "ex-bankdruecken");

        // Vorlagen
        let tpl = template_from_day(&p, "p1", d("2026-11-16"), "Push".into()).await.unwrap();
        assert_eq!(tpl.items.len(), 1);
        assert_eq!(tpl.items[0].sets, 2);
        assert_eq!(tpl.items[0].weight_kg, 70.0);
        let n = apply_template(&p, "p1", &tpl.id, d("2026-11-20")).await.unwrap();
        assert_eq!(n, 1);
        let day = entries(&p, "p1", d("2026-11-20")).await.unwrap();
        assert_eq!(day.len(), 1);
        assert!(day[0].sets.iter().all(|s| !s.done));
        assert!(day[0].kcal > 0.0);

        delete_entry(&p, "p1", &e1.id).await.unwrap();
        assert!(get_entry(&p, "p1", &e1.id).await.is_err());
        delete_exercise(&p, "p1", &custom.id).await.unwrap();
        delete_template(&p, "p1", &tpl.id).await.unwrap();
        assert!(templates(&p, "p1").await.unwrap().is_empty());
    }
}

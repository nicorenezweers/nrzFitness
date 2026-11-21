//! Apple Health und Google Health Connect.
//!
//! - Apple-Health-Export (`export.zip` bzw. `export.xml`) hochladen: wird gestreamt gespeichert und
//!   ohne alles in den Speicher zu laden ausgewertet (Exporte haben oft mehrere hundert MB).
//! - Automatische Synchronisierung: `POST /api/health/hook/ingest` mit Profil-Token nimmt JSON von
//!   iOS-Kurzbefehlen, „Health Auto Export“ oder Android-Apps (Health Connect) an.
//! - Zurückschreiben: `GET /api/health/hook/day` liefert die Tages-Nährwerte, z.B. für einen
//!   Kurzbefehl, der sie als „Nahrungsenergie“ in Apple Health einträgt.

use std::collections::{BTreeMap, HashMap};
use std::io::{BufRead, BufReader, Read};
use std::path::{Path, PathBuf};

use chrono::{NaiveDate, NaiveDateTime};
use dioxus::server::axum::{
    Json, Router,
    body::Body,
    extract::{DefaultBodyLimit, Query},
    http::{HeaderMap, StatusCode},
    response::{IntoResponse, Response},
    routing::{get, post},
};
use futures_util::StreamExt;
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::SqlitePool;
use tokio::io::AsyncWriteExt;

use super::db::{new_id, now_ts};
use super::{AppError, AppResult, pool};
use crate::model::{HealthDay, HealthImport, HealthImportResult, HealthStatus, HealthSync, HealthWorkout, workout_label};

// ---------------------------------------------------------------- Aggregation

/// Sammelt Messwerte je Tag. Summen (Schritte, Aktivkalorien, Schlaf) werden je Quelle addiert und
/// pro Tag die grösste Quelle genommen – so zählen iPhone und Watch nicht doppelt.
#[derive(Default)]
struct Agg {
    steps: HashMap<(NaiveDate, String), f64>,
    active: HashMap<(NaiveDate, String), f64>,
    sleep: HashMap<(NaiveDate, String), f64>,
    /// Letzter Wert des Tages (nach Zeit).
    weight: HashMap<NaiveDate, (String, f64)>,
    body_fat: HashMap<NaiveDate, (String, f64)>,
    resting: HashMap<NaiveDate, (f64, u32)>,
    workouts: Vec<HealthWorkout>,
}

impl Agg {
    fn last(map: &mut HashMap<NaiveDate, (String, f64)>, d: NaiveDate, at: &str, v: f64) {
        match map.get(&d) {
            Some((t, _)) if t.as_str() > at => {}
            _ => {
                map.insert(d, (at.to_string(), v));
            }
        }
    }

    fn finish(self) -> HealthImport {
        let mut days: BTreeMap<NaiveDate, HealthDay> = BTreeMap::new();
        fn day(days: &mut BTreeMap<NaiveDate, HealthDay>, d: NaiveDate) -> &mut HealthDay {
            days.entry(d).or_insert_with(|| HealthDay { date: d, ..Default::default() })
        }
        let max_per_day = |m: HashMap<(NaiveDate, String), f64>| {
            let mut out: HashMap<NaiveDate, f64> = HashMap::new();
            for ((d, _), v) in m {
                let e = out.entry(d).or_insert(0.0);
                if v > *e {
                    *e = v;
                }
            }
            out
        };
        for (d, v) in max_per_day(self.steps) {
            day(&mut days, d).steps = Some(v.round() as i64);
        }
        for (d, v) in max_per_day(self.active) {
            day(&mut days, d).active_kcal = Some((v * 10.0).round() / 10.0);
        }
        for (d, v) in max_per_day(self.sleep) {
            if v > 0.25 {
                day(&mut days, d).sleep_hours = Some((v * 100.0).round() / 100.0);
            }
        }
        for (d, (_, v)) in self.weight {
            day(&mut days, d).weight = Some((v * 100.0).round() / 100.0);
        }
        for (d, (_, v)) in self.body_fat {
            day(&mut days, d).body_fat = Some((v * 10.0).round() / 10.0);
        }
        for (d, (sum, n)) in self.resting {
            if n > 0 {
                day(&mut days, d).resting_hr = Some((sum / n as f64).round() as i64);
            }
        }
        HealthImport { days: days.into_values().collect(), workouts: self.workouts }
    }
}

/// Datum aus „2026-11-20 08:00:00 +0100“, „2026-11-20T08:00:00Z“ oder „2026-11-20“ (Ortszeit des Geräts).
fn date_of(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.get(..10)?, "%Y-%m-%d").ok()
}

/// Sekunden zwischen zwei Zeitstempeln im Apple-Format (Zeitzone wird von beiden gleich behandelt).
fn hours_between(a: &str, b: &str) -> f64 {
    let p = |s: &str| {
        let s = s.get(..19)?.replace('T', " ");
        NaiveDateTime::parse_from_str(&s, "%Y-%m-%d %H:%M:%S").ok()
    };
    match (p(a), p(b)) {
        (Some(x), Some(y)) if y > x => (y - x).num_seconds() as f64 / 3600.0,
        _ => 0.0,
    }
}

fn to_kg(v: f64, unit: &str) -> f64 {
    match unit.to_lowercase().as_str() {
        "lb" | "lbs" | "pound" => v * 0.453_592_37,
        "g" => v / 1000.0,
        "st" => v * 6.350_293,
        _ => v,
    }
}

fn to_kcal(v: f64, unit: &str) -> f64 {
    match unit.to_lowercase().as_str() {
        "kj" => v / 4.184,
        "j" => v / 4184.0,
        _ => v,
    }
}

fn to_km(v: f64, unit: &str) -> f64 {
    match unit.to_lowercase().as_str() {
        "mi" => v * 1.609_344,
        "m" => v / 1000.0,
        "yd" => v * 0.000_914_4,
        _ => v,
    }
}

// ---------------------------------------------------------------- Apple-Health-Export

fn attrs(e: &quick_xml::events::BytesStart) -> HashMap<String, String> {
    e.attributes()
        .flatten()
        .filter_map(|a| {
            let k = String::from_utf8_lossy(a.key.as_ref()).to_string();
            let v = a.unescape_value().ok()?.to_string();
            Some((k, v))
        })
        .collect()
}

fn parse_apple_xml<R: BufRead>(reader: R, since: Option<NaiveDate>) -> AppResult<HealthImport> {
    use quick_xml::events::Event;

    let mut xml = quick_xml::Reader::from_reader(reader);
    xml.config_mut().trim_text(true);
    let mut buf = Vec::new();
    let mut agg = Agg::default();
    // Offenes <Workout> (Statistiken kommen als Kind-Elemente).
    let mut current: Option<(HashMap<String, String>, f64, Option<f64>)> = None;
    let keep = |d: NaiveDate| since.is_none_or(|s| d >= s);

    loop {
        let ev = xml.read_event_into(&mut buf).map_err(|e| AppError::BadRequest(format!("Export nicht lesbar: {e}")))?;
        match ev {
            Event::Eof => break,
            Event::Start(ref e) | Event::Empty(ref e) => {
                let is_empty = matches!(ev, Event::Empty(_));
                match e.name().as_ref() {
                    b"Record" => {
                        let a = attrs(e);
                        let kind = a.get("type").map(String::as_str).unwrap_or("");
                        let start = a.get("startDate").map(String::as_str).unwrap_or("");
                        let end = a.get("endDate").map(String::as_str).unwrap_or(start);
                        let src = a.get("sourceName").cloned().unwrap_or_default();
                        let unit = a.get("unit").map(String::as_str).unwrap_or("");
                        let value = a.get("value").map(String::as_str).unwrap_or("");
                        let num = value.parse::<f64>().ok();
                        match kind {
                            "HKQuantityTypeIdentifierStepCount" => {
                                if let (Some(d), Some(v)) = (date_of(start), num)
                                    && keep(d)
                                {
                                    *agg.steps.entry((d, src)).or_insert(0.0) += v;
                                }
                            }
                            "HKQuantityTypeIdentifierActiveEnergyBurned" => {
                                if let (Some(d), Some(v)) = (date_of(start), num)
                                    && keep(d)
                                {
                                    *agg.active.entry((d, src)).or_insert(0.0) += to_kcal(v, unit);
                                }
                            }
                            "HKQuantityTypeIdentifierBodyMass" => {
                                if let (Some(d), Some(v)) = (date_of(start), num)
                                    && keep(d)
                                {
                                    Agg::last(&mut agg.weight, d, start, to_kg(v, unit));
                                }
                            }
                            "HKQuantityTypeIdentifierBodyFatPercentage" => {
                                if let (Some(d), Some(v)) = (date_of(start), num)
                                    && keep(d)
                                {
                                    Agg::last(&mut agg.body_fat, d, start, if v <= 1.0 { v * 100.0 } else { v });
                                }
                            }
                            "HKQuantityTypeIdentifierRestingHeartRate" => {
                                if let (Some(d), Some(v)) = (date_of(start), num)
                                    && keep(d)
                                {
                                    let e = agg.resting.entry(d).or_insert((0.0, 0));
                                    e.0 += v;
                                    e.1 += 1;
                                }
                            }
                            "HKCategoryTypeIdentifierSleepAnalysis" => {
                                // Nur Schlafphasen zählen, nicht „Im Bett“ oder „Wach“. Nacht = Datum des Aufwachens.
                                if value.starts_with("HKCategoryValueSleepAnalysisAsleep")
                                    && let Some(d) = date_of(end)
                                    && keep(d)
                                {
                                    *agg.sleep.entry((d, src)).or_insert(0.0) += hours_between(start, end);
                                }
                            }
                            _ => {}
                        }
                    }
                    b"Workout" => {
                        let a = attrs(e);
                        let mut minutes = a.get("duration").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                        match a.get("durationUnit").map(String::as_str) {
                            Some("s") | Some("sec") => minutes /= 60.0,
                            Some("hr") | Some("h") => minutes *= 60.0,
                            _ => {}
                        }
                        let kcal = a
                            .get("totalEnergyBurned")
                            .and_then(|v| v.parse::<f64>().ok())
                            .map(|v| to_kcal(v, a.get("totalEnergyBurnedUnit").map(String::as_str).unwrap_or("kcal")))
                            .unwrap_or(0.0);
                        let dist = a
                            .get("totalDistance")
                            .and_then(|v| v.parse::<f64>().ok())
                            .map(|v| to_km(v, a.get("totalDistanceUnit").map(String::as_str).unwrap_or("km")))
                            .filter(|v| *v > 0.0);
                        if is_empty {
                            finish_workout(&mut agg, (a, minutes, dist), kcal, since);
                        } else {
                            // Neuere Exporte liefern kcal/Distanz als <WorkoutStatistics>-Kinder.
                            let mut a = a;
                            a.insert("__kcal".into(), kcal.to_string());
                            current = Some((a, minutes, dist));
                        }
                    }
                    b"WorkoutStatistics" => {
                        if let Some((a, _, dist)) = current.as_mut() {
                            let s = attrs(e);
                            let sum = s.get("sum").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                            let unit = s.get("unit").map(String::as_str).unwrap_or("");
                            match s.get("type").map(String::as_str) {
                                Some("HKQuantityTypeIdentifierActiveEnergyBurned") => {
                                    let old = a.get("__kcal").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                                    if old <= 0.0 {
                                        a.insert("__kcal".into(), to_kcal(sum, unit).to_string());
                                    }
                                }
                                Some(t) if t.starts_with("HKQuantityTypeIdentifierDistance") && dist.is_none() && sum > 0.0 => {
                                    *dist = Some(to_km(sum, unit));
                                }
                                _ => {}
                            }
                        }
                    }
                    _ => {}
                }
            }
            Event::End(ref e) if e.name().as_ref() == b"Workout" => {
                if let Some((a, minutes, dist)) = current.take() {
                    let kcal = a.get("__kcal").and_then(|v| v.parse::<f64>().ok()).unwrap_or(0.0);
                    finish_workout(&mut agg, (a, minutes, dist), kcal, since);
                }
            }
            _ => {}
        }
        buf.clear();
    }
    Ok(agg.finish())
}

fn finish_workout(agg: &mut Agg, (a, minutes, dist): (HashMap<String, String>, f64, Option<f64>), kcal: f64, since: Option<NaiveDate>) {
    let start = a.get("startDate").cloned().unwrap_or_default();
    let Some(date) = date_of(&start) else { return };
    if since.is_some_and(|s| date < s) {
        return;
    }
    let kind = a.get("workoutActivityType").cloned().unwrap_or_default();
    let (name, cat) = workout_label(&kind);
    agg.workouts.push(HealthWorkout {
        external_id: format!("apple:{start}:{kind}"),
        date,
        name,
        category: cat.into(),
        minutes: (minutes * 10.0).round() / 10.0,
        kcal: kcal.round(),
        distance_km: dist.map(|d| (d * 100.0).round() / 100.0),
    });
}

/// `export.zip` (darin `apple_health_export/export.xml`) oder direkt `export.xml` auswerten.
fn read_apple_export(path: &Path, since: Option<NaiveDate>) -> AppResult<HealthImport> {
    let io = |e: std::io::Error| AppError::BadRequest(format!("Datei nicht lesbar: {e}"));
    let mut f = std::fs::File::open(path).map_err(io)?;
    let mut magic = [0u8; 2];
    let n = f.read(&mut magic).map_err(io)?;
    drop(f);
    if n == 2 && &magic == b"PK" {
        let file = std::fs::File::open(path).map_err(io)?;
        let mut zip = zip::ZipArchive::new(file).map_err(|e| AppError::BadRequest(format!("ZIP nicht lesbar: {e}")))?;
        let idx = (0..zip.len())
            .find(|&i| zip.name_for_index(i).is_some_and(|n| n.ends_with("export.xml") && !n.contains("__MACOSX")))
            .ok_or_else(|| AppError::BadRequest("Im ZIP fehlt export.xml – ist das ein Apple-Health-Export?".into()))?;
        let entry = zip.by_index(idx).map_err(|e| AppError::BadRequest(format!("ZIP nicht lesbar: {e}")))?;
        parse_apple_xml(BufReader::with_capacity(1 << 20, entry), since)
    } else {
        let file = std::fs::File::open(path).map_err(io)?;
        parse_apple_xml(BufReader::with_capacity(1 << 20, file), since)
    }
}

// ---------------------------------------------------------------- JSON (Automationen)

fn num(v: Option<&Value>) -> Option<f64> {
    match v? {
        Value::Number(n) => n.as_f64(),
        Value::String(s) => s.trim().replace(',', ".").parse().ok(),
        Value::Object(o) => num(o.get("qty").or_else(|| o.get("value"))),
        _ => None,
    }
    .filter(|x: &f64| x.is_finite())
}

fn unit_of(v: Option<&Value>) -> String {
    v.and_then(|o| o.get("units").or_else(|| o.get("unit"))).and_then(Value::as_str).unwrap_or("").to_string()
}

/// JSON aus Kurzbefehlen/Apps lesen. Unterstützt:
/// - eigenes Format: `{"entries":[{"type":"steps","date":"2026-11-20","value":8123}], "workouts":[…]}`
///   (Werte sind Tagessummen bzw. -werte; Typen: steps, weight, body_fat, active_kcal, sleep_hours, resting_hr)
/// - „Health Auto Export“: `{"data":{"metrics":[…],"workouts":[…]}}`
pub fn parse_json(v: &Value) -> AppResult<HealthImport> {
    let mut agg = Agg::default();
    let root = v.get("data").unwrap_or(v);

    // Eigenes Format
    if let Some(list) = root.get("entries").and_then(Value::as_array) {
        let mut days: BTreeMap<NaiveDate, HealthDay> = BTreeMap::new();
        for e in list {
            let Some(d) = e.get("date").and_then(Value::as_str).and_then(date_of) else { continue };
            let Some(val) = num(e.get("value")) else { continue };
            let unit = e.get("unit").and_then(Value::as_str).unwrap_or("");
            let day = days.entry(d).or_insert_with(|| HealthDay { date: d, ..Default::default() });
            match e.get("type").and_then(Value::as_str).unwrap_or("") {
                "steps" | "step_count" => day.steps = Some(val.round() as i64),
                "weight" | "body_mass" => day.weight = Some(to_kg(val, unit)),
                "body_fat" => day.body_fat = Some(if val <= 1.0 { val * 100.0 } else { val }),
                "active_kcal" | "active_energy" => day.active_kcal = Some(to_kcal(val, unit)),
                "sleep_hours" | "sleep" => day.sleep_hours = Some(if unit == "min" { val / 60.0 } else { val }),
                "resting_hr" | "resting_heart_rate" => day.resting_hr = Some(val.round() as i64),
                _ => {}
            }
        }
        let mut out = HealthImport { days: days.into_values().collect(), workouts: Vec::new() };
        out.workouts = parse_workouts(root);
        return Ok(out);
    }

    // Health Auto Export
    if let Some(metrics) = root.get("metrics").and_then(Value::as_array) {
        for m in metrics {
            let name = m.get("name").and_then(Value::as_str).unwrap_or("");
            let unit = m.get("units").and_then(Value::as_str).unwrap_or("");
            for (i, p) in m.get("data").and_then(Value::as_array).into_iter().flatten().enumerate() {
                let date_s = p.get("date").and_then(Value::as_str).unwrap_or("");
                let Some(d) = date_of(date_s) else { continue };
                let src = p.get("source").and_then(Value::as_str).unwrap_or("").to_string();
                let q = num(p.get("qty"));
                match name {
                    "step_count" => {
                        if let Some(q) = q {
                            *agg.steps.entry((d, src)).or_insert(0.0) += q;
                        }
                    }
                    "active_energy" => {
                        if let Some(q) = q {
                            *agg.active.entry((d, src)).or_insert(0.0) += to_kcal(q, unit);
                        }
                    }
                    "weight_body_mass" => {
                        if let Some(q) = q {
                            Agg::last(&mut agg.weight, d, &format!("{date_s}{i:08}"), to_kg(q, unit));
                        }
                    }
                    "body_fat_percentage" => {
                        if let Some(q) = q {
                            Agg::last(&mut agg.body_fat, d, &format!("{date_s}{i:08}"), if q <= 1.0 { q * 100.0 } else { q });
                        }
                    }
                    "resting_heart_rate" => {
                        if let Some(q) = q {
                            let e = agg.resting.entry(d).or_insert((0.0, 0));
                            e.0 += q;
                            e.1 += 1;
                        }
                    }
                    "sleep_analysis" => {
                        let h = num(p.get("totalSleep")).or_else(|| num(p.get("asleep"))).or(q).unwrap_or(0.0);
                        let h = if unit == "min" { h / 60.0 } else { h };
                        *agg.sleep.entry((d, src)).or_insert(0.0) += h;
                    }
                    _ => {}
                }
            }
        }
        agg.workouts = parse_workouts(root);
        return Ok(agg.finish());
    }

    if root.get("workouts").is_some() {
        return Ok(HealthImport { days: Vec::new(), workouts: parse_workouts(root) });
    }
    Err(AppError::BadRequest("Unbekanntes Format – erwartet \"entries\" oder \"data.metrics\"".into()))
}

fn parse_workouts(root: &Value) -> Vec<HealthWorkout> {
    let mut out = Vec::new();
    for w in root.get("workouts").and_then(Value::as_array).into_iter().flatten() {
        let start = w.get("start").or_else(|| w.get("date")).and_then(Value::as_str).unwrap_or("");
        let Some(date) = date_of(start) else { continue };
        let raw = w.get("name").or_else(|| w.get("type")).and_then(Value::as_str).unwrap_or("Training");
        let (name, cat) = workout_label(raw);
        let minutes = num(w.get("minutes")).or_else(|| num(w.get("duration")).map(|s| s / 60.0)).unwrap_or(0.0);
        let kcal_v = w.get("kcal").or_else(|| w.get("activeEnergyBurned")).or_else(|| w.get("activeEnergy"));
        let kcal = num(kcal_v).map(|k| to_kcal(k, &unit_of(kcal_v))).unwrap_or(0.0);
        let dist_v = w.get("distance_km").or_else(|| w.get("distance"));
        let dist = num(dist_v).map(|d| if w.get("distance_km").is_some() { d } else { to_km(d, &unit_of(dist_v)) }).filter(|d| *d > 0.0);
        let id = w
            .get("id")
            .and_then(Value::as_str)
            .map(|i| format!("ext:{i}"))
            .unwrap_or_else(|| format!("ext:{start}:{raw}"));
        out.push(HealthWorkout {
            external_id: id,
            date,
            name,
            category: cat.into(),
            minutes: (minutes * 10.0).round() / 10.0,
            kcal: kcal.round(),
            distance_km: dist.map(|d| (d * 100.0).round() / 100.0),
        });
    }
    out
}

// ---------------------------------------------------------------- Speichern

pub async fn apply(pool: &SqlitePool, pid: &str, data: HealthImport, source: &str) -> AppResult<HealthImportResult> {
    let mut tx = pool.begin().await?;
    let mut dates: Vec<NaiveDate> = Vec::new();
    for d in &data.days {
        if d.weight.is_none() && d.body_fat.is_none() && d.steps.is_none() && d.active_kcal.is_none() && d.sleep_hours.is_none() && d.resting_hr.is_none() {
            continue;
        }
        sqlx::query(
            "INSERT INTO checkins (profile_id, date, weight, body_fat, steps, active_kcal, sleep_hours, resting_hr)
             VALUES (?, ?, ?, ?, ?, ?, ?, ?)
             ON CONFLICT(profile_id, date) DO UPDATE SET
               weight = COALESCE(excluded.weight, weight),
               body_fat = COALESCE(excluded.body_fat, body_fat),
               steps = COALESCE(excluded.steps, steps),
               active_kcal = COALESCE(excluded.active_kcal, active_kcal),
               sleep_hours = COALESCE(excluded.sleep_hours, sleep_hours),
               resting_hr = COALESCE(excluded.resting_hr, resting_hr)",
        )
        .bind(pid)
        .bind(d.date)
        .bind(d.weight.filter(|w| (20.0..400.0).contains(w)))
        .bind(d.body_fat.filter(|f| (1.0..80.0).contains(f)))
        .bind(d.steps.filter(|s| (0..200_000).contains(s)))
        .bind(d.active_kcal.filter(|k| (0.0..20_000.0).contains(k)))
        .bind(d.sleep_hours.filter(|h| (0.0..24.0).contains(h)))
        .bind(d.resting_hr.filter(|h| (20..250).contains(h)))
        .execute(&mut *tx)
        .await?;
        dates.push(d.date);
    }
    let mut workouts = 0;
    for w in &data.workouts {
        if w.minutes <= 0.0 && w.kcal <= 0.0 {
            continue;
        }
        let existing: Option<String> = sqlx::query_scalar("SELECT id FROM exercise_entries WHERE profile_id = ? AND external_id = ?")
            .bind(pid)
            .bind(&w.external_id)
            .fetch_optional(&mut *tx)
            .await?;
        match existing {
            Some(id) => {
                sqlx::query("UPDATE exercise_entries SET duration_min = ?, kcal = ?, distance_km = ? WHERE id = ?")
                    .bind(w.minutes)
                    .bind(w.kcal)
                    .bind(w.distance_km)
                    .bind(id)
                    .execute(&mut *tx)
                    .await?;
            }
            None => {
                sqlx::query(
                    "INSERT INTO exercise_entries (id, profile_id, date, exercise_id, name, category, duration_min, kcal, distance_km, notes, created_at, external_id)
                     VALUES (?, ?, ?, NULL, ?, ?, ?, ?, ?, ?, ?, ?)",
                )
                .bind(new_id())
                .bind(pid)
                .bind(w.date)
                .bind(&w.name)
                .bind(&w.category)
                .bind(w.minutes)
                .bind(w.kcal)
                .bind(w.distance_km)
                .bind(source)
                .bind(now_ts())
                .bind(&w.external_id)
                .execute(&mut *tx)
                .await?;
            }
        }
        workouts += 1;
        dates.push(w.date);
    }
    let result = HealthImportResult {
        days: data.days.len(),
        workouts,
        from: dates.iter().min().copied(),
        to: dates.iter().max().copied(),
    };
    sqlx::query("INSERT INTO health_syncs (id, profile_id, source, summary, created_at) VALUES (?, ?, ?, ?, ?)")
        .bind(new_id())
        .bind(pid)
        .bind(source)
        .bind(result.summary())
        .bind(now_ts())
        .execute(&mut *tx)
        .await?;
    sqlx::query(
        "DELETE FROM health_syncs WHERE profile_id = ? AND id NOT IN
           (SELECT id FROM health_syncs WHERE profile_id = ? ORDER BY created_at DESC LIMIT 20)",
    )
    .bind(pid)
    .bind(pid)
    .execute(&mut *tx)
    .await?;
    tx.commit().await?;
    Ok(result)
}

pub async fn status(pool: &SqlitePool, pid: &str) -> AppResult<HealthStatus> {
    let token: Option<String> = sqlx::query_scalar("SELECT health_token FROM profiles WHERE id = ?").bind(pid).fetch_optional(pool).await?.flatten();
    let rows: Vec<(String, String, String)> =
        sqlx::query_as("SELECT source, summary, created_at FROM health_syncs WHERE profile_id = ? ORDER BY created_at DESC LIMIT 8")
            .bind(pid)
            .fetch_all(pool)
            .await?;
    Ok(HealthStatus {
        token,
        syncs: rows.into_iter().map(|(source, summary, created_at)| HealthSync { source, summary, created_at }).collect(),
    })
}

/// Neuen Token erzeugen (`None` = Verbindung trennen).
pub async fn set_token(pool: &SqlitePool, pid: &str, enable: bool) -> AppResult<Option<String>> {
    let token = enable.then(|| format!("nrzh_{}", uuid::Uuid::new_v4().simple()));
    sqlx::query("UPDATE profiles SET health_token = ? WHERE id = ?").bind(&token).bind(pid).execute(pool).await?;
    Ok(token)
}

async fn profile_for_token(pool: &SqlitePool, token: &str) -> AppResult<Option<String>> {
    if token.len() < 20 {
        return Ok(None);
    }
    Ok(sqlx::query_scalar("SELECT id FROM profiles WHERE health_token = ?").bind(token).fetch_optional(pool).await?)
}

// ---------------------------------------------------------------- HTTP

#[derive(Deserialize)]
struct ImportQuery {
    pid: String,
    /// Nur Daten ab diesem Datum (YYYY-MM-DD).
    since: Option<String>,
}

#[derive(Deserialize)]
struct TokenQuery {
    token: Option<String>,
    date: Option<String>,
}

fn err(status: StatusCode, msg: impl Into<String>) -> Response {
    (status, Json(json!({ "ok": false, "error": msg.into() }))).into_response()
}

fn token_from(headers: &HeaderMap, q: &TokenQuery) -> String {
    headers
        .get("authorization")
        .and_then(|v| v.to_str().ok())
        .map(|v| v.trim_start_matches("Bearer ").trim().to_string())
        .or_else(|| q.token.clone())
        .unwrap_or_default()
}

/// Upload des Apple-Health-Exports (Rohdaten der Datei als Body, hinter der umbrelOS-Anmeldung).
async fn import_apple(Query(q): Query<ImportQuery>, body: Body) -> Response {
    let exists: Option<String> = match sqlx::query_scalar("SELECT id FROM profiles WHERE id = ?").bind(&q.pid).fetch_optional(pool()).await {
        Ok(v) => v,
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    if exists.is_none() {
        return err(StatusCode::NOT_FOUND, "Profil nicht gefunden");
    }
    let since = q.since.as_deref().and_then(date_of);
    let dir = std::env::temp_dir().join(format!("nrzfitness-health-{}", uuid::Uuid::new_v4()));
    let path: PathBuf = dir.join("upload");
    let r: AppResult<HealthImportResult> = async {
        tokio::fs::create_dir_all(&dir).await.map_err(|e| AppError::Http(e.to_string()))?;
        let mut file = tokio::fs::File::create(&path).await.map_err(|e| AppError::Http(e.to_string()))?;
        let mut stream = body.into_data_stream();
        let mut size: u64 = 0;
        while let Some(chunk) = stream.next().await {
            let chunk = chunk.map_err(|e| AppError::Http(format!("Upload abgebrochen: {e}")))?;
            size += chunk.len() as u64;
            if size > 4 * 1024 * 1024 * 1024 {
                return Err(AppError::BadRequest("Datei zu gross (max. 4 GB)".into()));
            }
            file.write_all(&chunk).await.map_err(|e| AppError::Http(e.to_string()))?;
        }
        file.flush().await.map_err(|e| AppError::Http(e.to_string()))?;
        drop(file);
        if size == 0 {
            return Err(AppError::BadRequest("Leere Datei".into()));
        }
        let p = path.clone();
        let data = tokio::task::spawn_blocking(move || read_apple_export(&p, since))
            .await
            .map_err(|e| AppError::Http(e.to_string()))??;
        if data.days.is_empty() && data.workouts.is_empty() {
            return Err(AppError::BadRequest("Keine passenden Daten gefunden (Schritte, Gewicht, Schlaf, Trainings …)".into()));
        }
        apply(pool(), &q.pid, data, "Apple Health (Export)").await
    }
    .await;
    let _ = tokio::fs::remove_dir_all(&dir).await;
    match r {
        Ok(res) => Json(json!({ "ok": true, "summary": res.summary(), "result": res })).into_response(),
        Err(AppError::BadRequest(m)) => err(StatusCode::BAD_REQUEST, m),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// Automatische Synchronisierung (Kurzbefehl, Health Auto Export, Android-Apps). Ohne umbrelOS-Anmeldung,
/// geschützt durch den Profil-Token.
async fn ingest(headers: HeaderMap, Query(q): Query<TokenQuery>, body: String) -> Response {
    let token = token_from(&headers, &q);
    let pid = match profile_for_token(pool(), &token).await {
        Ok(Some(p)) => p,
        Ok(None) => return err(StatusCode::UNAUTHORIZED, "Token ungültig"),
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let v: Value = match serde_json::from_str(&body) {
        Ok(v) => v,
        Err(e) => return err(StatusCode::BAD_REQUEST, format!("Kein gültiges JSON: {e}")),
    };
    let source = v
        .get("source")
        .and_then(Value::as_str)
        .map(|s| s.chars().take(40).collect::<String>())
        .unwrap_or_else(|| if v.pointer("/data/metrics").is_some() { "Health Auto Export".into() } else { "Automation".into() });
    let data = match parse_json(&v) {
        Ok(d) => d,
        Err(e) => return err(StatusCode::BAD_REQUEST, e.to_string()),
    };
    match apply(pool(), &pid, data, &source).await {
        Ok(res) => Json(json!({ "ok": true, "summary": res.summary() })).into_response(),
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

/// Tages-Nährwerte für Kurzbefehle (z.B. „Nahrungsenergie“ in Apple Health protokollieren).
async fn day(headers: HeaderMap, Query(q): Query<TokenQuery>) -> Response {
    let token = token_from(&headers, &q);
    let pid = match profile_for_token(pool(), &token).await {
        Ok(Some(p)) => p,
        Ok(None) => return err(StatusCode::UNAUTHORIZED, "Token ungültig"),
        Err(e) => return err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    };
    let date = q.date.as_deref().and_then(date_of).unwrap_or_else(crate::util::today);
    match super::db::diary::day(pool(), &pid, date).await {
        Ok(d) => {
            let n = d.totals();
            let r = |v: f64| (v * 10.0).round() / 10.0;
            Json(json!({
                "date": date,
                "kcal": r(n.kcal),
                "protein": r(n.protein),
                "carbs": r(n.carbs),
                "fat": r(n.fat),
                "fiber": r(n.fiber),
                "sugar": r(n.sugar),
                "sodium_mg": r(n.sodium_mg),
                "water_ml": r(d.water_ml),
                "burned_kcal": r(d.burned_kcal),
            }))
            .into_response()
        }
        Err(e) => err(StatusCode::INTERNAL_SERVER_ERROR, e.to_string()),
    }
}

pub fn routes() -> Router {
    Router::new()
        .route("/api/health/import/apple", post(import_apple).layer(DefaultBodyLimit::disable()))
        .route("/api/health/hook/ingest", post(ingest).layer(DefaultBodyLimit::max(32 * 1024 * 1024)))
        .route("/api/health/hook/day", get(day))
}

#[cfg(test)]
mod tests {
    use super::*;

    const XML: &str = r#"<?xml version="1.0" encoding="UTF-8"?>
<HealthData locale="de_CH">
 <Record type="HKQuantityTypeIdentifierStepCount" sourceName="iPhone" unit="count" startDate="2026-11-20 08:00:00 +0100" endDate="2026-11-20 08:10:00 +0100" value="1000"/>
 <Record type="HKQuantityTypeIdentifierStepCount" sourceName="iPhone" unit="count" startDate="2026-11-20 12:00:00 +0100" endDate="2026-11-20 12:10:00 +0100" value="2000"/>
 <Record type="HKQuantityTypeIdentifierStepCount" sourceName="Apple Watch" unit="count" startDate="2026-11-20 08:00:00 +0100" endDate="2026-11-20 08:10:00 +0100" value="3500"/>
 <Record type="HKQuantityTypeIdentifierBodyMass" sourceName="Waage" unit="lb" startDate="2026-11-20 07:00:00 +0100" endDate="2026-11-20 07:00:00 +0100" value="185"/>
 <Record type="HKQuantityTypeIdentifierBodyFatPercentage" sourceName="Waage" unit="%" startDate="2026-11-20 07:00:00 +0100" endDate="2026-11-20 07:00:00 +0100" value="0.21"/>
 <Record type="HKCategoryTypeIdentifierSleepAnalysis" sourceName="Watch" startDate="2026-11-19 23:00:00 +0100" endDate="2026-11-20 03:00:00 +0100" value="HKCategoryValueSleepAnalysisAsleepCore"/>
 <Record type="HKCategoryTypeIdentifierSleepAnalysis" sourceName="Watch" startDate="2026-11-20 03:00:00 +0100" endDate="2026-11-20 03:30:00 +0100" value="HKCategoryValueSleepAnalysisAwake"/>
 <Record type="HKCategoryTypeIdentifierSleepAnalysis" sourceName="Watch" startDate="2026-11-20 03:30:00 +0100" endDate="2026-11-20 06:30:00 +0100" value="HKCategoryValueSleepAnalysisAsleepREM"/>
 <Workout workoutActivityType="HKWorkoutActivityTypeRunning" duration="30" durationUnit="min" sourceName="Watch" startDate="2026-11-20 18:00:00 +0100" endDate="2026-11-20 18:30:00 +0100">
  <WorkoutStatistics type="HKQuantityTypeIdentifierActiveEnergyBurned" startDate="2026-11-20 18:00:00 +0100" endDate="2026-11-20 18:30:00 +0100" sum="1255" unit="kJ"/>
  <WorkoutStatistics type="HKQuantityTypeIdentifierDistanceWalkingRunning" startDate="2026-11-20 18:00:00 +0100" endDate="2026-11-20 18:30:00 +0100" sum="5.2" unit="km"/>
 </Workout>
 <Workout workoutActivityType="HKWorkoutActivityTypeYoga" duration="20" durationUnit="min" totalEnergyBurned="80" totalEnergyBurnedUnit="kcal" startDate="2026-11-18 07:00:00 +0100" endDate="2026-11-18 07:20:00 +0100"/>
</HealthData>"#;

    #[test]
    fn parses_apple_export() {
        let r = parse_apple_xml(XML.as_bytes(), None).unwrap();
        let d = r.days.iter().find(|d| d.date.to_string() == "2026-11-20").unwrap();
        assert_eq!(d.steps, Some(3500)); // Watch (3500) > iPhone (3000), nicht 6500
        assert!((d.weight.unwrap() - 83.91).abs() < 0.01);
        assert_eq!(d.body_fat, Some(21.0));
        assert_eq!(d.sleep_hours, Some(7.0)); // 4 h + 3 h, „Wach“ zählt nicht
        assert_eq!(r.workouts.len(), 2);
        let run = r.workouts.iter().find(|w| w.name == "Laufen").unwrap();
        assert_eq!(run.kcal, 300.0);
        assert_eq!(run.distance_km, Some(5.2));
        assert_eq!(run.minutes, 30.0);
        let since = parse_apple_xml(XML.as_bytes(), NaiveDate::from_ymd_opt(2026, 11, 19)).unwrap();
        assert_eq!(since.workouts.len(), 1);
    }

    #[test]
    fn parses_own_json() {
        let v: Value = serde_json::from_str(
            r#"{"entries":[{"type":"steps","date":"2026-11-20","value":"8123"},{"type":"weight","date":"2026-11-20T07:00:00+01:00","value":84.2}],
                "workouts":[{"name":"EXERCISE_TYPE_BIKING","date":"2026-11-20","minutes":45,"kcal":400,"distance_km":15}]}"#,
        )
        .unwrap();
        let r = parse_json(&v).unwrap();
        assert_eq!(r.days[0].steps, Some(8123));
        assert_eq!(r.days[0].weight, Some(84.2));
        assert_eq!(r.workouts[0].name, "Velofahren");
        assert_eq!(r.workouts[0].distance_km, Some(15.0));
    }

    #[test]
    fn parses_health_auto_export() {
        let v: Value = serde_json::from_str(
            r#"{"data":{"metrics":[
                {"name":"step_count","units":"count","data":[{"date":"2026-11-20 00:00:00 +0100","qty":4000,"source":"iPhone"},{"date":"2026-11-20 13:00:00 +0100","qty":1000,"source":"iPhone"}]},
                {"name":"active_energy","units":"kJ","data":[{"date":"2026-11-20 00:00:00 +0100","qty":2092}]},
                {"name":"sleep_analysis","units":"hr","data":[{"date":"2026-11-20 00:00:00 +0100","totalSleep":7.5}]}],
              "workouts":[{"id":"abc","name":"Outdoor Run","start":"2026-11-20 18:00:00 +0100","duration":1800,"activeEnergyBurned":{"qty":320,"units":"kcal"},"distance":{"qty":3.1,"units":"mi"}}]}}"#,
        )
        .unwrap();
        let r = parse_json(&v).unwrap();
        assert_eq!(r.days[0].steps, Some(5000));
        assert_eq!(r.days[0].active_kcal, Some(500.0));
        assert_eq!(r.days[0].sleep_hours, Some(7.5));
        assert_eq!(r.workouts[0].name, "Laufen");
        assert_eq!(r.workouts[0].minutes, 30.0);
        assert!((r.workouts[0].distance_km.unwrap() - 4.99).abs() < 0.01);
    }
}

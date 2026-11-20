//! Training: Übungen, Einträge mit Sätzen, Vorlagen, Verlauf und Rekorde.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Kategorien (Schlüssel, Anzeigename).
pub const CATEGORIES: [(&str, &str); 4] =
    [("strength", "Kraft"), ("cardio", "Cardio"), ("flexibility", "Dehnen"), ("sport", "Sport")];

pub fn category_label(key: &str) -> &'static str {
    CATEGORIES.iter().find(|(k, _)| *k == key).map(|(_, l)| *l).unwrap_or("Kraft")
}

#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn valid_category(key: &str) -> &'static str {
    CATEGORIES.iter().find(|(k, _)| *k == key).map(|(k, _)| *k).unwrap_or("strength")
}

pub const MUSCLES: [&str; 8] = ["Brust", "Rücken", "Beine", "Schultern", "Arme", "Bauch", "Ganzkörper", "Cardio"];

pub const EQUIPMENT: [&str; 10] = [
    "Langhantel",
    "Kurzhantel",
    "Maschine",
    "Kabelzug",
    "Körpergewicht",
    "Kettlebell",
    "Widerstandsband",
    "SZ-Stange",
    "Gerät",
    "Ohne",
];

/// Körpergewicht, falls keins erfasst ist (für kcal-Schätzung).
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub const DEFAULT_WEIGHT: f64 = 75.0;
/// Geschätzte Dauer pro Kraft-Satz inkl. Pause (min), falls keine Dauer angegeben.
pub const MIN_PER_SET: f64 = 2.5;

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Exercise {
    pub id: String,
    pub name: String,
    pub category: String,
    pub muscle: Option<String>,
    pub equipment: Option<String>,
    pub met: f64,
    /// Eigene Übung des Profils (bearbeitbar)?
    #[cfg_attr(feature = "server", sqlx(default))]
    pub custom: bool,
}

impl Exercise {
    pub fn is_strength(&self) -> bool {
        self.category == "strength"
    }
    pub fn subtitle(&self) -> String {
        let mut parts: Vec<&str> = Vec::new();
        if let Some(m) = self.muscle.as_deref().filter(|m| !m.is_empty()) {
            parts.push(m);
        }
        if let Some(e) = self.equipment.as_deref().filter(|e| !e.is_empty()) {
            parts.push(e);
        }
        if parts.is_empty() {
            parts.push(category_label(&self.category));
        }
        parts.join(" · ")
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ExerciseInput {
    pub name: String,
    pub category: String,
    pub muscle: Option<String>,
    pub equipment: Option<String>,
    pub met: f64,
}

/// Ein Satz (Wiederholungen × Gewicht).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct SetRow {
    pub id: String,
    pub reps: i64,
    pub weight_kg: f64,
    pub done: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct ExerciseEntry {
    pub id: String,
    pub date: NaiveDate,
    pub exercise_id: Option<String>,
    pub name: String,
    pub category: String,
    pub duration_min: f64,
    pub kcal: f64,
    pub distance_km: Option<f64>,
    pub notes: Option<String>,
    #[cfg_attr(feature = "server", sqlx(skip))]
    pub sets: Vec<SetRow>,
}

impl ExerciseEntry {
    pub fn is_strength(&self) -> bool {
        self.category == "strength"
    }
    /// Volumen (kg) der erledigten Sätze.
    pub fn volume(&self) -> f64 {
        self.sets.iter().filter(|s| s.done).map(|s| s.reps as f64 * s.weight_kg).sum()
    }
    pub fn done_sets(&self) -> usize {
        self.sets.iter().filter(|s| s.done).count()
    }
}

/// Kurzform der Sätze, z.B. "3×10 · 60 kg" oder "10/8/6 · 60–70 kg".
pub fn sets_summary(sets: &[SetRow]) -> String {
    use crate::util::num;
    if sets.is_empty() {
        return String::new();
    }
    let reps_same = sets.iter().all(|s| s.reps == sets[0].reps);
    let reps = if reps_same {
        format!("{}×{}", sets.len(), sets[0].reps)
    } else {
        sets.iter().map(|s| s.reps.to_string()).collect::<Vec<_>>().join("/")
    };
    let min = sets.iter().map(|s| s.weight_kg).fold(f64::INFINITY, f64::min);
    let max = sets.iter().map(|s| s.weight_kg).fold(0.0, f64::max);
    if max <= 0.0 {
        reps
    } else if (max - min).abs() < 1e-9 {
        format!("{reps} · {} kg", num(max, 2))
    } else {
        format!("{reps} · {}–{} kg", num(min, 2), num(max, 2))
    }
}

/// Neuer oder geänderter Eintrag.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct EntryInput {
    pub date: NaiveDate,
    pub exercise_id: Option<String>,
    pub name: String,
    pub category: String,
    pub duration_min: f64,
    pub kcal: f64,
    pub distance_km: Option<f64>,
    pub notes: Option<String>,
    pub sets: Vec<SetRow>,
}

/// Übungsbibliothek mit „Zuletzt verwendet“ und Körpergewicht für die kcal-Schätzung.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Library {
    pub exercises: Vec<Exercise>,
    pub recent: Vec<Exercise>,
    /// Letztes Gewicht oder `DEFAULT_WEIGHT`.
    pub weight: f64,
    /// Gewicht wurde tatsächlich erfasst (sonst Annahme).
    pub weight_known: bool,
}

/// Trainings-Vorlage (Routine).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WorkoutTemplate {
    pub id: String,
    pub name: String,
    pub notes: Option<String>,
    pub items: Vec<TemplateItem>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TemplateItem {
    pub exercise: Exercise,
    pub sets: i64,
    pub reps: i64,
    pub weight_kg: f64,
    pub duration_min: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TemplateItemInput {
    pub exercise_id: String,
    pub sets: i64,
    pub reps: i64,
    pub weight_kg: f64,
    pub duration_min: f64,
}

/// Eine Woche im Verlauf (Montag = `start`).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct WeekStat {
    pub start: NaiveDate,
    pub minutes: f64,
    pub kcal: f64,
    pub volume: f64,
    pub days: i64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct TrainingHistory {
    pub weeks: Vec<WeekStat>,
    /// Aktuelle Serie: aufeinanderfolgende Trainingstage bis heute (oder gestern).
    pub streak: i64,
    pub best_streak: i64,
    /// Trainingstage in den letzten 30 Tagen.
    pub days_30: i64,
    pub total_days: i64,
    pub last_date: Option<NaiveDate>,
}

/// Persönliche Rekorde einer Übung (nur erledigte Sätze).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct PersonalRecord {
    pub exercise_id: String,
    pub name: String,
    pub category: String,
    /// Schwerster Satz (kg, Wdh).
    pub heaviest_kg: f64,
    pub heaviest_reps: i64,
    /// Bestes geschätztes 1RM (Epley) mit Datum.
    pub best_e1rm: f64,
    pub best_e1rm_date: Option<NaiveDate>,
    pub most_reps: i64,
    pub best_volume: f64,
    /// Cardio: längste Dauer / weiteste Distanz.
    pub longest_min: f64,
    pub longest_km: f64,
    pub sessions: i64,
    pub last_date: Option<NaiveDate>,
}

/// Ein Punkt im Fortschritts-Diagramm (pro Trainingstag).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ProgressPoint {
    pub date: NaiveDate,
    pub e1rm: f64,
    pub volume: f64,
    pub top_kg: f64,
    pub duration_min: f64,
    pub distance_km: f64,
}

/// Geschätztes 1RM nach Epley.
#[cfg_attr(not(feature = "server"), allow(dead_code))]
pub fn epley(weight_kg: f64, reps: i64) -> f64 {
    if weight_kg <= 0.0 || reps <= 0 {
        0.0
    } else if reps == 1 {
        weight_kg
    } else {
        weight_kg * (1.0 + reps as f64 / 30.0)
    }
}

/// kcal-Schätzung: MET × kg × h. Ohne Dauer bei Kraft: `MIN_PER_SET` pro Satz.
pub fn estimate_kcal(met: f64, body_kg: f64, duration_min: f64, sets: usize) -> f64 {
    let minutes = if duration_min > 0.0 { duration_min } else { sets as f64 * MIN_PER_SET };
    crate::util::met_kcal(met, body_kg, minutes).max(0.0)
}

/// Pace "5:30 /km" aus Dauer und Distanz.
pub fn pace(duration_min: f64, km: f64) -> Option<String> {
    if duration_min <= 0.0 || km <= 0.0 {
        return None;
    }
    let secs = (duration_min * 60.0 / km).round() as i64;
    Some(format!("{}:{:02} /km", secs / 60, secs % 60))
}

pub fn speed_kmh(duration_min: f64, km: f64) -> Option<f64> {
    (duration_min > 0.0 && km > 0.0).then(|| km / (duration_min / 60.0))
}

/// Zusammenfassung eines Tages.
#[derive(Debug, Clone, Copy, PartialEq, Default)]
pub struct DaySummary {
    pub minutes: f64,
    pub kcal: f64,
    pub volume: f64,
    pub count: usize,
    pub sets: usize,
}

pub fn day_summary(entries: &[ExerciseEntry]) -> DaySummary {
    let mut s = DaySummary { count: entries.len(), ..Default::default() };
    for e in entries {
        s.minutes += e.duration_min;
        s.kcal += e.kcal;
        s.volume += e.volume();
        s.sets += e.done_sets();
    }
    s
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn epley_and_pace() {
        assert_eq!(epley(100.0, 1), 100.0);
        assert!((epley(60.0, 10) - 80.0).abs() < 1e-9);
        assert_eq!(pace(30.0, 6.0).as_deref(), Some("5:00 /km"));
        assert_eq!(pace(0.0, 6.0), None);
    }

    #[test]
    fn summary_text() {
        let s = |r, w| SetRow { id: String::new(), reps: r, weight_kg: w, done: true };
        assert_eq!(sets_summary(&[s(10, 60.0), s(10, 60.0), s(10, 60.0)]), "3×10 · 60 kg");
        assert_eq!(sets_summary(&[s(10, 60.0), s(8, 70.0)]), "10/8 · 60–70 kg");
        assert_eq!(sets_summary(&[s(12, 0.0)]), "1×12");
    }
}

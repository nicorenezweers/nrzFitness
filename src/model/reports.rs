//! Berichte: Tageswerte eines Zeitraums, Zusammenfassungen, Top-Listen, Konsistenz.

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

use super::Goals;

/// Wählbare Zeiträume (Schlüssel, Beschriftung, Tage).
pub const REPORT_RANGES: [(&str, &str, i64); 4] =
    [("7", "7T", 7), ("30", "30T", 30), ("90", "90T", 90), ("365", "1J", 365)];

/// Alle Werte eines Tages (fehlende Messungen = None, Summen = 0).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct DayRow {
    pub date: NaiveDate,
    /// Anzahl Tagebuch-Einträge (0 = nichts gegessen erfasst).
    pub food_count: i64,
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    pub fiber: f64,
    pub sugar: f64,
    pub water_ml: f64,
    pub burned_kcal: f64,
    pub exercise_min: f64,
    pub workouts: i64,
    pub weight: Option<f64>,
    pub body_fat: Option<f64>,
    pub waist: Option<f64>,
    pub steps: Option<i64>,
    pub mood: Option<f64>,
    pub energy: Option<f64>,
    pub sleep_h: Option<f64>,
    pub sleep_quality: Option<f64>,
    /// Summe der Stunden aller an diesem Tag beendeten Fasten.
    pub fast_hours: Option<f64>,
    /// Längstes an diesem Tag beendetes Fasten (h).
    pub fast_max_h: Option<f64>,
    pub fasts: i64,
    pub fasts_reached: i64,
}

impl DayRow {
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub fn empty(date: NaiveDate) -> Self {
        Self { date, ..Default::default() }
    }
}

/// Zusammenfassung eines Zeitraums (Durchschnitte nur über Tage mit Daten).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Summary {
    pub days: i64,
    pub food_days: i64,
    pub avg_kcal: Option<f64>,
    pub avg_protein: Option<f64>,
    pub avg_carbs: Option<f64>,
    pub avg_fat: Option<f64>,
    pub avg_fiber: Option<f64>,
    pub avg_sugar: Option<f64>,
    pub water_days: i64,
    pub avg_water: Option<f64>,
    pub workouts: i64,
    pub exercise_min: f64,
    pub burned_kcal: f64,
    pub weight_first: Option<f64>,
    pub weight_last: Option<f64>,
    pub weight_min: Option<f64>,
    pub weight_max: Option<f64>,
    pub avg_body_fat: Option<f64>,
    pub avg_sleep: Option<f64>,
    pub avg_sleep_quality: Option<f64>,
    pub avg_mood: Option<f64>,
    pub avg_energy: Option<f64>,
    pub avg_steps: Option<f64>,
    pub steps_goal_days: i64,
    pub step_days: i64,
    pub fasts: i64,
    pub fasts_reached: i64,
    pub avg_fast_h: Option<f64>,
    pub longest_fast_h: Option<f64>,
}

impl Summary {
    pub fn weight_change(&self) -> Option<f64> {
        match (self.weight_first, self.weight_last) {
            (Some(a), Some(b)) => Some(b - a),
            _ => None,
        }
    }

    /// Berechnet die Zusammenfassung aus Tageszeilen.
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub fn from_days(days: &[DayRow], step_goal: i64) -> Self {
        fn avg(it: impl Iterator<Item = f64>) -> Option<f64> {
            let (s, n) = it.fold((0.0, 0usize), |(s, n), v| (s + v, n + 1));
            (n > 0).then_some(s / n as f64)
        }
        let food: Vec<&DayRow> = days.iter().filter(|d| d.food_count > 0).collect();
        let weights: Vec<f64> = days.iter().filter_map(|d| d.weight).collect();
        let steps: Vec<i64> = days.iter().filter_map(|d| d.steps).collect();
        let fasts: i64 = days.iter().map(|d| d.fasts).sum();
        let fast_h: f64 = days.iter().filter_map(|d| d.fast_hours).sum();
        Self {
            days: days.len() as i64,
            food_days: food.len() as i64,
            avg_kcal: avg(food.iter().map(|d| d.kcal)),
            avg_protein: avg(food.iter().map(|d| d.protein)),
            avg_carbs: avg(food.iter().map(|d| d.carbs)),
            avg_fat: avg(food.iter().map(|d| d.fat)),
            avg_fiber: avg(food.iter().map(|d| d.fiber)),
            avg_sugar: avg(food.iter().map(|d| d.sugar)),
            water_days: days.iter().filter(|d| d.water_ml > 0.0).count() as i64,
            avg_water: avg(days.iter().filter(|d| d.water_ml > 0.0).map(|d| d.water_ml)),
            workouts: days.iter().map(|d| d.workouts).sum(),
            exercise_min: days.iter().map(|d| d.exercise_min).sum(),
            burned_kcal: days.iter().map(|d| d.burned_kcal).sum(),
            weight_first: weights.first().copied(),
            weight_last: weights.last().copied(),
            weight_min: weights.iter().copied().reduce(f64::min),
            weight_max: weights.iter().copied().reduce(f64::max),
            avg_body_fat: avg(days.iter().filter_map(|d| d.body_fat)),
            avg_sleep: avg(days.iter().filter_map(|d| d.sleep_h)),
            avg_sleep_quality: avg(days.iter().filter_map(|d| d.sleep_quality)),
            avg_mood: avg(days.iter().filter_map(|d| d.mood)),
            avg_energy: avg(days.iter().filter_map(|d| d.energy)),
            avg_steps: avg(steps.iter().map(|s| *s as f64)),
            steps_goal_days: steps.iter().filter(|s| step_goal > 0 && **s >= step_goal).count() as i64,
            step_days: steps.len() as i64,
            fasts,
            fasts_reached: days.iter().map(|d| d.fasts_reached).sum(),
            avg_fast_h: (fasts > 0).then_some(fast_h / fasts as f64),
            longest_fast_h: days.iter().filter_map(|d| d.fast_max_h).reduce(f64::max),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct TopFood {
    pub name: String,
    pub count: i64,
    pub kcal: f64,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct TopExercise {
    pub name: String,
    pub count: i64,
    pub minutes: f64,
    pub kcal: f64,
}

/// Erfasste Bereiche eines Tages (für den Konsistenz-Kalender).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ActivityDay {
    pub date: NaiveDate,
    /// Anzahl Bereiche mit Einträgen (Essen, Wasser, Training, Check-in, Fasten): 0..5.
    pub level: u8,
}

/// Kompletter Bericht für einen Zeitraum.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct Report {
    pub from: NaiveDate,
    pub to: NaiveDate,
    pub days: Vec<DayRow>,
    pub summary: Summary,
    /// Gleich langer Zeitraum direkt davor (für Trends).
    pub prev: Summary,
    pub goals: Goals,
    pub top_foods_count: Vec<TopFood>,
    pub top_foods_kcal: Vec<TopFood>,
    pub top_exercises: Vec<TopExercise>,
    /// Tage mit Einträgen (Konsistenz-Kalender), aufsteigend, nur Tage mit level > 0.
    pub activity: Vec<ActivityDay>,
    pub heat_from: NaiveDate,
    /// Aktuelle Erfassungs-Serie in Tagen (bis heute bzw. gestern).
    pub streak: i64,
    /// Längste Serie im betrachteten Jahr.
    pub best_streak: i64,
}

/// Ergebnis eines Imports.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct ImportResult {
    /// Fertig (letzter Teil empfangen und importiert)?
    pub done: bool,
    pub imported: u64,
    pub skipped: u64,
    pub tables: u64,
}

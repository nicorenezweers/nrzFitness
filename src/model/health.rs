//! Apple Health / Google Health Connect: importierte Tageswerte und Trainings.
#![cfg_attr(not(feature = "server"), allow(dead_code))]

use chrono::NaiveDate;
use serde::{Deserialize, Serialize};

/// Tageswerte aus einer Gesundheits-App (nur gesetzte Felder werden übernommen).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthDay {
    pub date: NaiveDate,
    pub weight: Option<f64>,
    pub body_fat: Option<f64>,
    pub steps: Option<i64>,
    pub active_kcal: Option<f64>,
    pub sleep_hours: Option<f64>,
    pub resting_hr: Option<i64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthWorkout {
    /// Eindeutig je Quelle (verhindert Doppelte bei erneutem Import).
    pub external_id: String,
    pub date: NaiveDate,
    pub name: String,
    pub category: String,
    pub minutes: f64,
    pub kcal: f64,
    pub distance_km: Option<f64>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthImport {
    pub days: Vec<HealthDay>,
    pub workouts: Vec<HealthWorkout>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthImportResult {
    pub days: usize,
    pub workouts: usize,
    pub from: Option<NaiveDate>,
    pub to: Option<NaiveDate>,
}

impl HealthImportResult {
    pub fn summary(&self) -> String {
        let range = match (self.from, self.to) {
            (Some(a), Some(b)) if a != b => format!(" ({} – {})", a.format("%d.%m.%Y"), b.format("%d.%m.%Y")),
            (Some(a), _) => format!(" ({})", a.format("%d.%m.%Y")),
            _ => String::new(),
        };
        format!("{} Tage, {} Trainings{range}", self.days, self.workouts)
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthSync {
    pub source: String,
    pub summary: String,
    pub created_at: String,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct HealthStatus {
    pub token: Option<String>,
    pub syncs: Vec<HealthSync>,
}

/// Name und Kategorie eines Trainingstyps (Apple `HKWorkoutActivityType…`, Health Connect `EXERCISE_TYPE_…`
/// oder freier Text).
pub fn workout_label(raw: &str) -> (String, &'static str) {
    let k = raw
        .trim_start_matches("HKWorkoutActivityType")
        .trim_start_matches("EXERCISE_TYPE_")
        .replace(['_', ' ', '-'], "")
        .to_lowercase();
    let (name, cat) = match k.as_str() {
        "running" | "run" | "outdoorrun" | "indoorrun" | "treadmill" | "runningtreadmill" => ("Laufen", "cardio"),
        "walking" | "walk" | "outdoorwalk" | "indoorwalk" => ("Gehen", "cardio"),
        "hiking" | "hike" => ("Wandern", "cardio"),
        "cycling" | "biking" | "outdoorcycle" | "indoorcycle" | "bikingstationary" => ("Velofahren", "cardio"),
        "swimming" | "poolswim" | "openwaterswim" | "swimmingpool" | "swimmingopenwater" => ("Schwimmen", "cardio"),
        "rowing" | "rowingmachine" | "indoorrow" => ("Rudern", "cardio"),
        "elliptical" => ("Crosstrainer", "cardio"),
        "stairclimbing" | "stairs" | "stairclimbingmachine" => ("Treppensteigen", "cardio"),
        "traditionalstrengthtraining" | "functionalstrengthtraining" | "strengthtraining" | "weightlifting" => ("Krafttraining", "strength"),
        "highintensityintervaltraining" | "hiit" | "highintensityintervaltrainingworkout" => ("HIIT", "cardio"),
        "crosstraining" | "crossfit" => ("Crosstraining", "strength"),
        "yoga" => ("Yoga", "flexibility"),
        "pilates" => ("Pilates", "flexibility"),
        "flexibility" | "stretching" | "cooldown" => ("Dehnen", "flexibility"),
        "dance" | "dancing" | "socialdance" | "cardiodance" => ("Tanzen", "cardio"),
        "soccer" | "football" | "footballamerican" => ("Fussball", "sport"),
        "tennis" => ("Tennis", "sport"),
        "badminton" => ("Badminton", "sport"),
        "basketball" => ("Basketball", "sport"),
        "volleyball" => ("Volleyball", "sport"),
        "tabletennis" => ("Tischtennis", "sport"),
        "golf" => ("Golf", "sport"),
        "downhillskiing" | "skiing" | "crosscountryskiing" => ("Skifahren", "sport"),
        "snowboarding" => ("Snowboarden", "sport"),
        "climbing" | "rockclimbing" => ("Klettern", "sport"),
        "boxing" | "kickboxing" | "martialarts" => ("Kampfsport", "sport"),
        "coretraining" => ("Core-Training", "strength"),
        _ => ("", "cardio"),
    };
    if name.is_empty() {
        let t = raw.trim_start_matches("HKWorkoutActivityType").trim_start_matches("EXERCISE_TYPE_").replace('_', " ");
        let t = t.trim();
        (if t.is_empty() { "Training".to_string() } else { t.to_string() }, cat)
    } else {
        (name.to_string(), cat)
    }
}

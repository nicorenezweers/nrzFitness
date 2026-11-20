//! Körper / täglicher Check-in: Gewicht, Umfänge, Schritte, Stimmung, Schlaf.

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

/// Ein Check-in-Tag; alle Werte optional (`None` = nicht erfasst).
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Checkin {
    pub date: NaiveDate,
    pub weight: Option<f64>,
    pub body_fat: Option<f64>,
    pub waist: Option<f64>,
    pub hips: Option<f64>,
    pub chest: Option<f64>,
    pub neck: Option<f64>,
    pub arm: Option<f64>,
    pub thigh: Option<f64>,
    pub steps: Option<i64>,
    pub mood: Option<i64>,
    pub energy: Option<i64>,
    pub stress: Option<i64>,
    pub sleep_hours: Option<f64>,
    pub sleep_quality: Option<i64>,
    pub resting_hr: Option<i64>,
    pub notes: Option<String>,
}

impl Checkin {
    pub fn empty(date: NaiveDate) -> Self {
        Self { date, ..Default::default() }
    }

    /// Nichts erfasst? (Server: leere Check-ins werden gelöscht.)
    #[cfg_attr(not(feature = "server"), allow(dead_code))]
    pub fn is_empty(&self) -> bool {
        let Checkin {
            date: _,
            weight,
            body_fat,
            waist,
            hips,
            chest,
            neck,
            arm,
            thigh,
            steps,
            mood,
            energy,
            stress,
            sleep_hours,
            sleep_quality,
            resting_hr,
            notes,
        } = self;
        [weight, body_fat, waist, hips, chest, neck, arm, thigh, sleep_hours].iter().all(|v| v.is_none())
            && [steps, mood, energy, stress, sleep_quality, resting_hr].iter().all(|v| v.is_none())
            && notes.as_ref().is_none_or(|n| n.trim().is_empty())
    }

    /// Umfänge (Schlüssel, Beschriftung, Wert).
    pub fn measurements(&self) -> [(&'static str, &'static str, Option<f64>); 6] {
        [
            ("waist", "Taille", self.waist),
            ("hips", "Hüfte", self.hips),
            ("chest", "Brust", self.chest),
            ("neck", "Hals", self.neck),
            ("arm", "Arm", self.arm),
            ("thigh", "Oberschenkel", self.thigh),
        ]
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct WeightPoint {
    pub date: NaiveDate,
    pub weight: f64,
}

/// Übersicht für die Körper-Seite.
#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct BodyOverview {
    /// Alle Gewichte (aufsteigend nach Datum).
    pub weights: Vec<WeightPoint>,
    /// Letzte Check-ins (absteigend).
    pub recent: Vec<Checkin>,
}

pub const MOOD_EMOJI: [&str; 5] = ["😞", "😕", "😐", "🙂", "😄"];
pub const ENERGY_EMOJI: [&str; 5] = ["🪫", "😴", "😐", "⚡", "🚀"];
pub const STRESS_EMOJI: [&str; 5] = ["😌", "🙂", "😐", "😣", "🤯"];
pub const SLEEP_EMOJI: [&str; 5] = ["😫", "😪", "😐", "😊", "😴"];

/// Letztes Gewicht am oder vor `on`.
pub fn weight_on_or_before(weights: &[WeightPoint], on: NaiveDate) -> Option<WeightPoint> {
    weights.iter().rev().find(|w| w.date <= on).copied()
}

/// Veränderung des aktuellsten Gewichts gegenüber dem Wert vor `days` Tagen (oder dem ältesten danach).
pub fn weight_change(weights: &[WeightPoint], days: i64) -> Option<f64> {
    let last = weights.last()?;
    let cutoff = last.date - Duration::days(days);
    let base = weight_on_or_before(weights, cutoff)
        .or_else(|| weights.iter().find(|w| w.date >= cutoff && w.date < last.date).copied())?;
    (base.date < last.date).then(|| last.weight - base.weight)
}

/// Gleitender Durchschnitt (Fenster `window` Tage, nur vorhandene Werte) für jeden Tag von `from` bis `to`.
pub fn moving_average(
    weights: &[WeightPoint],
    from: NaiveDate,
    to: NaiveDate,
    window: i64,
) -> Vec<(NaiveDate, Option<f64>)> {
    let mut out = Vec::new();
    let mut d = from;
    while d <= to {
        let lo = d - Duration::days(window - 1);
        let vals: Vec<f64> = weights.iter().filter(|w| w.date >= lo && w.date <= d).map(|w| w.weight).collect();
        out.push((d, (!vals.is_empty()).then(|| vals.iter().sum::<f64>() / vals.len() as f64)));
        d += Duration::days(1);
    }
    out
}

/// Fortschritt zum Zielgewicht 0..1 (Start = ältester Wert).
pub fn target_progress(weights: &[WeightPoint], target: f64) -> Option<f64> {
    let start = weights.first()?.weight;
    let cur = weights.last()?.weight;
    let total = start - target;
    if total.abs() < 0.05 {
        return Some(if (cur - target).abs() < 0.05 { 1.0 } else { 0.0 });
    }
    Some(((start - cur) / total).clamp(0.0, 1.0))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn wp(s: &str, w: f64) -> WeightPoint {
        WeightPoint { date: d(s), weight: w }
    }

    #[test]
    fn change_and_average() {
        let ws = vec![wp("2026-10-01", 80.0), wp("2026-11-10", 78.0), wp("2026-11-17", 77.0)];
        assert_eq!(weight_change(&ws, 7), Some(-1.0));
        assert_eq!(weight_change(&ws, 30), Some(-3.0));
        let ma = moving_average(&ws, d("2026-11-16"), d("2026-11-17"), 7);
        assert_eq!(ma[0].1, Some(78.0));
        assert_eq!(ma[1].1, Some(77.0));
        assert_eq!(target_progress(&ws, 75.0), Some(0.6));
    }

    #[test]
    fn empty_checkin() {
        let mut c = Checkin::empty(d("2026-11-20"));
        assert!(c.is_empty());
        c.notes = Some("  ".into());
        assert!(c.is_empty());
        c.steps = Some(10);
        assert!(!c.is_empty());
    }
}

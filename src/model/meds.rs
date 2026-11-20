//! Medikamente (Einnahmeplan, Protokoll) und Symptome.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Medication {
    pub id: String,
    pub name: String,
    pub dose: Option<String>,
    /// Kommagetrennte Uhrzeiten, z.B. "08:00,20:00".
    pub times: String,
    pub active: bool,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
}

impl Medication {
    /// Einnahmezeiten (sortiert, ohne Duplikate).
    pub fn slots(&self) -> Vec<String> {
        parse_times(&self.times)
    }
}

/// "20:00, 8:00" → ["08:00", "20:00"]
pub fn parse_times(s: &str) -> Vec<String> {
    let mut v: Vec<String> = s
        .split(',')
        .filter_map(|t| {
            let (h, m) = t.trim().split_once(':')?;
            let (h, m) = (h.trim().parse::<u32>().ok()?, m.trim().parse::<u32>().ok()?);
            (h < 24 && m < 60).then(|| format!("{h:02}:{m:02}"))
        })
        .collect();
    v.sort();
    v.dedup();
    v
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MedicationInput {
    pub name: String,
    pub dose: Option<String>,
    pub times: Vec<String>,
    pub notes: Option<String>,
    pub active: bool,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct MedLog {
    pub medication_id: String,
    pub date: NaiveDate,
    pub slot: String,
    pub taken_at: NaiveDateTime,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct MedsData {
    pub meds: Vec<Medication>,
    /// Protokoll der letzten 30 Tage (bis heute) plus gewählter Tag.
    pub logs: Vec<MedLog>,
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Symptom {
    pub id: String,
    pub date: NaiveDate,
    pub name: String,
    pub severity: i64,
    pub notes: Option<String>,
    pub created_at: NaiveDateTime,
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct SymptomsData {
    /// Neueste zuerst.
    pub list: Vec<Symptom>,
    /// Zuletzt verwendete Namen (für Schnell-Chips).
    pub recent_names: Vec<String>,
}

pub const SYMPTOM_SUGGESTIONS: [&str; 8] = [
    "Kopfschmerzen",
    "Müdigkeit",
    "Übelkeit",
    "Bauchschmerzen",
    "Krämpfe",
    "Schwindel",
    "Rückenschmerzen",
    "Schlafprobleme",
];

pub const SEVERITY: [&str; 5] = ["Sehr leicht", "Leicht", "Mittel", "Stark", "Sehr stark"];

pub fn severity_label(s: i64) -> &'static str {
    SEVERITY.get((s.clamp(1, 5) - 1) as usize).copied().unwrap_or("Mittel")
}

/// Tageszeit-Bezeichnung für eine Uhrzeit "HH:MM".
pub fn slot_label(slot: &str) -> &'static str {
    match slot.get(0..2).and_then(|h| h.parse::<u32>().ok()).unwrap_or(8) {
        4..=10 => "Morgens",
        11..=13 => "Mittags",
        14..=17 => "Nachmittags",
        18..=21 => "Abends",
        _ => "Nachts",
    }
}

/// Einhaltung in % für die Tage `today-(days-1)..=today`: genommene / geplante Einnahmen.
/// Heute zählen nur Zeiten, die schon vorbei oder bereits genommen sind. `None`, wenn nichts geplant war.
pub fn adherence(meds: &[Medication], logs: &[MedLog], today: NaiveDate, now: NaiveDateTime, days: i64) -> Option<f64> {
    let (mut planned, mut taken) = (0u32, 0u32);
    let now_hm = now.format("%H:%M").to_string();
    for i in 0..days {
        let d = today - Duration::days(i);
        for m in meds.iter().filter(|m| m.active && m.created_at.date() <= d) {
            for s in m.slots() {
                let done = logs.iter().any(|l| l.medication_id == m.id && l.date == d && l.slot == s);
                if d == today && !done && s > now_hm {
                    continue;
                }
                planned += 1;
                if done {
                    taken += 1;
                }
            }
        }
    }
    (planned > 0).then(|| taken as f64 * 100.0 / planned as f64)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
    }

    #[test]
    fn times() {
        assert_eq!(parse_times("20:00, 8:00,8:00,xx,25:00"), vec!["08:00", "20:00"]);
        assert_eq!(slot_label("08:00"), "Morgens");
        assert_eq!(slot_label("22:30"), "Nachts");
    }

    #[test]
    fn adherence_counts() {
        let m = Medication {
            id: "a".into(),
            name: "Vitamin D".into(),
            dose: None,
            times: "08:00,20:00".into(),
            active: true,
            notes: None,
            created_at: dt("2026-11-19T07:00"),
        };
        let today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let log = |d: &str, s: &str| MedLog {
            medication_id: "a".into(),
            date: dt(&format!("{d}T00:00")).date(),
            slot: s.into(),
            taken_at: dt("2026-11-20T08:00"),
        };
        let logs = vec![log("2026-11-19", "08:00"), log("2026-11-20", "08:00")];
        // Gestern 1/2, heute (12 Uhr) 1/1 → 2/3
        let a = adherence(&[m.clone()], &logs, today, dt("2026-11-20T12:00"), 7).unwrap();
        assert!((a - 200.0 / 3.0).abs() < 1e-9);
        assert_eq!(adherence(&[], &logs, today, dt("2026-11-20T12:00"), 7), None);
    }
}

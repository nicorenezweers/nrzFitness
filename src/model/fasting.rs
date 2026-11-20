//! Intervallfasten: Fastenzeiten, Phasen und Statistiken.

use chrono::{Duration, NaiveDate, NaiveDateTime};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct Fast {
    pub id: String,
    pub start_at: NaiveDateTime,
    pub end_at: Option<NaiveDateTime>,
    pub target_hours: f64,
    pub notes: Option<String>,
}

impl Fast {
    /// Dauer in Sekunden (laufend: bis `now`).
    pub fn secs(&self, now: NaiveDateTime) -> i64 {
        (self.end_at.unwrap_or(now) - self.start_at).num_seconds().max(0)
    }

    pub fn hours(&self, now: NaiveDateTime) -> f64 {
        self.secs(now) as f64 / 3600.0
    }

    pub fn reached(&self, now: NaiveDateTime) -> bool {
        self.hours(now) + 1e-9 >= self.target_hours
    }

    /// Geplantes Ende (Start + Ziel).
    pub fn target_end(&self) -> NaiveDateTime {
        self.start_at + Duration::seconds((self.target_hours * 3600.0).round() as i64)
    }
}

#[derive(Debug, Clone, Default, PartialEq, Serialize, Deserialize)]
pub struct FastingData {
    pub active: Option<Fast>,
    /// Abgeschlossene Fastenzeiten (neueste zuerst).
    pub history: Vec<Fast>,
    /// Standardziel aus den Zielen.
    pub default_hours: f64,
}

pub const PRESETS: [f64; 7] = [13.0, 16.0, 18.0, 20.0, 24.0, 36.0, 48.0];

/// Preset-Bezeichnung, z.B. "16:8".
pub fn preset_label(h: f64) -> String {
    if h < 24.0 { format!("{}:{}", h as i64, 24 - h as i64) } else { format!("{} h", h as i64) }
}

pub struct Phase {
    pub from_h: f64,
    pub name: &'static str,
    pub text: &'static str,
    pub emoji: &'static str,
}

pub const PHASES: [Phase; 6] = [
    Phase {
        from_h: 0.0,
        name: "Verdauung",
        text: "Der Körper verarbeitet die letzte Mahlzeit, der Blutzucker steigt.",
        emoji: "🍽️",
    },
    Phase {
        from_h: 4.0,
        name: "Blutzucker sinkt",
        text: "Insulin fällt, die Glykogenspeicher werden angezapft.",
        emoji: "📉",
    },
    Phase {
        from_h: 12.0,
        name: "Fettverbrennung",
        text: "Die Speicher leeren sich, der Körper nutzt vermehrt Fett.",
        emoji: "🔥",
    },
    Phase {
        from_h: 18.0,
        name: "Ketose",
        text: "Ketonkörper entstehen und liefern Energie für das Gehirn.",
        emoji: "⚡",
    },
    Phase {
        from_h: 24.0,
        name: "Autophagie",
        text: "Zellen räumen auf und recyceln beschädigte Bestandteile.",
        emoji: "♻️",
    },
    Phase {
        from_h: 48.0,
        name: "Tiefe Ketose",
        text: "Wachstumshormone steigen, Ketose ist voll ausgeprägt.",
        emoji: "🧬",
    },
];

/// Index der aktuellen Phase für eine Dauer in Stunden.
pub fn phase_index(hours: f64) -> usize {
    PHASES.iter().rposition(|p| hours >= p.from_h).unwrap_or(0)
}

#[derive(Debug, Clone, Default, PartialEq)]
pub struct FastingStats {
    pub count: usize,
    pub avg_hours: f64,
    pub longest_hours: f64,
    /// Ziel erreicht (in %).
    pub success_rate: f64,
    /// Aufeinanderfolgende Tage (bis heute bzw. gestern) mit erreichtem Ziel (nach Endtag).
    pub streak: i64,
}

/// Statistik über abgeschlossene Fastenzeiten.
pub fn stats(history: &[Fast], today: NaiveDate) -> FastingStats {
    let done: Vec<&Fast> = history.iter().filter(|f| f.end_at.is_some()).collect();
    if done.is_empty() {
        return FastingStats::default();
    }
    let now = today.and_hms_opt(0, 0, 0).unwrap_or_default();
    let hours: Vec<f64> = done.iter().map(|f| f.hours(now)).collect();
    let reached = done.iter().filter(|f| f.reached(now)).count();
    let mut days: Vec<NaiveDate> =
        done.iter().filter(|f| f.reached(now)).filter_map(|f| f.end_at.map(|e| e.date())).collect();
    days.sort();
    days.dedup();
    let mut streak = 0;
    let mut cur = if days.last() == Some(&today) { today } else { today - Duration::days(1) };
    while days.binary_search(&cur).is_ok() {
        streak += 1;
        cur -= Duration::days(1);
    }
    FastingStats {
        count: done.len(),
        avg_hours: hours.iter().sum::<f64>() / hours.len() as f64,
        longest_hours: hours.iter().cloned().fold(0.0, f64::max),
        success_rate: reached as f64 * 100.0 / done.len() as f64,
        streak,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dt(s: &str) -> NaiveDateTime {
        NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M").unwrap()
    }

    fn fast(start: &str, end: &str, target: f64) -> Fast {
        Fast { id: start.into(), start_at: dt(start), end_at: Some(dt(end)), target_hours: target, notes: None }
    }

    #[test]
    fn phases() {
        assert_eq!(phase_index(0.0), 0);
        assert_eq!(phase_index(12.5), 2);
        assert_eq!(phase_index(30.0), 4);
        assert_eq!(phase_index(100.0), 5);
        assert_eq!(preset_label(16.0), "16:8");
        assert_eq!(preset_label(36.0), "36 h");
    }

    #[test]
    fn streak_and_avg() {
        let today = NaiveDate::from_ymd_opt(2026, 11, 20).unwrap();
        let h = vec![
            fast("2026-11-19T20:00", "2026-11-20T12:00", 16.0), // 16 h, erreicht
            fast("2026-11-18T20:00", "2026-11-19T12:30", 16.0), // erreicht
            fast("2026-11-17T20:00", "2026-11-18T08:00", 16.0), // 12 h, nicht erreicht
            fast("2026-11-15T20:00", "2026-11-16T14:00", 18.0), // 18 h, erreicht
        ];
        let s = stats(&h, today);
        assert_eq!(s.count, 4);
        assert_eq!(s.streak, 2);
        assert_eq!(s.longest_hours, 18.0);
        assert!((s.avg_hours - (16.0 + 16.5 + 12.0 + 18.0) / 4.0).abs() < 1e-9);
        assert_eq!(s.success_rate, 75.0);
    }
}

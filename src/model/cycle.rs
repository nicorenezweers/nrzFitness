//! Zyklus: Periodentage, Zyklen und Vorhersage.

use chrono::{Duration, NaiveDate};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Serialize, Deserialize)]
#[cfg_attr(feature = "server", derive(sqlx::FromRow))]
pub struct CycleDay {
    pub date: NaiveDate,
    /// 1 leicht, 2 mittel, 3 stark
    pub flow: i64,
}

pub const FLOWS: [(i64, &str); 3] = [(1, "Leicht"), (2, "Mittel"), (3, "Stark")];

pub fn flow_label(f: i64) -> &'static str {
    FLOWS.iter().find(|(k, _)| *k == f).map(|(_, l)| *l).unwrap_or("Mittel")
}

pub const DEFAULT_CYCLE: i64 = 28;
pub const DEFAULT_PERIOD: i64 = 5;
/// Lücken bis zu so vielen Tagen gelten noch als dieselbe Periode.
const MAX_GAP: i64 = 2;

/// Eine erfasste Periode (zusammenhängende Periodentage).
#[derive(Debug, Clone, PartialEq)]
pub struct Period {
    pub start: NaiveDate,
    pub end: NaiveDate,
    /// Länge des Zyklus bis zur nächsten Periode (falls bekannt).
    pub cycle_len: Option<i64>,
}

impl Period {
    pub fn days(&self) -> i64 {
        (self.end - self.start).num_days() + 1
    }
}

/// Markierung eines Kalendertags.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DayMark {
    None,
    PredictedPeriod,
    Fertile,
    Ovulation,
}

#[derive(Debug, Clone, PartialEq)]
pub struct CycleInfo {
    /// Erfasste Perioden (älteste zuerst).
    pub periods: Vec<Period>,
    pub avg_cycle: i64,
    pub avg_period: i64,
    /// Durchschnitt aus echten Daten (sonst Standardwerte)?
    pub from_history: bool,
    pub next_period: Option<NaiveDate>,
    pub ovulation: Option<NaiveDate>,
    pub fertile: Option<(NaiveDate, NaiveDate)>,
    /// Aktueller Zyklustag (1 = erster Periodentag).
    pub cycle_day: Option<i64>,
    /// Periode ist seit so vielen Tagen überfällig.
    pub late_days: i64,
    /// Läuft gerade eine Periode?
    pub in_period: bool,
}

/// Periodentage zu Perioden gruppieren.
pub fn periods(days: &[CycleDay]) -> Vec<Period> {
    let mut ds: Vec<NaiveDate> = days.iter().map(|d| d.date).collect();
    ds.sort();
    ds.dedup();
    let mut out: Vec<Period> = Vec::new();
    for d in ds {
        match out.last_mut() {
            Some(p) if (d - p.end).num_days() <= MAX_GAP + 1 => p.end = d,
            _ => out.push(Period { start: d, end: d, cycle_len: None }),
        }
    }
    for i in 1..out.len() {
        let len = (out[i].start - out[i - 1].start).num_days();
        out[i - 1].cycle_len = Some(len);
    }
    out
}

/// Auswertung und Vorhersage aus der Historie (Standard 28/5 Tage, falls zu wenig Daten).
pub fn analyze(days: &[CycleDay], today: NaiveDate) -> CycleInfo {
    let ps = periods(days);
    // Nur plausible Zykluslängen der letzten 6 Zyklen berücksichtigen.
    let lens: Vec<i64> =
        ps.iter().filter_map(|p| p.cycle_len).filter(|l| (18..=45).contains(l)).rev().take(6).collect();
    // Laufende Periode (kann noch länger werden) nicht für die Periodendauer zählen.
    let pdays: Vec<i64> = ps
        .iter()
        .filter(|p| p.end < today - Duration::days(1))
        .map(|p| p.days())
        .filter(|d| (1..=10).contains(d))
        .rev()
        .take(6)
        .collect();
    let avg = |v: &[i64], def: i64| {
        if v.is_empty() { def } else { (v.iter().sum::<i64>() as f64 / v.len() as f64).round() as i64 }
    };
    let avg_cycle = avg(&lens, DEFAULT_CYCLE);
    let avg_period = avg(&pdays, DEFAULT_PERIOD);
    let mut info = CycleInfo {
        periods: ps.clone(),
        avg_cycle,
        avg_period,
        from_history: !lens.is_empty(),
        next_period: None,
        ovulation: None,
        fertile: None,
        cycle_day: None,
        late_days: 0,
        in_period: false,
    };
    let Some(last) = ps.iter().rev().find(|p| p.start <= today) else { return info };
    info.cycle_day = Some((today - last.start).num_days() + 1);
    info.in_period =
        today >= last.start && today <= last.end + Duration::days(1) && (today - last.start).num_days() < 10;
    let mut next = last.start + Duration::days(avg_cycle);
    let late = (today - next).num_days();
    if late > 0 && late <= 14 {
        info.late_days = late;
    } else {
        while next <= today {
            next += Duration::days(avg_cycle);
        }
    }
    let ov = next - Duration::days(14);
    info.next_period = Some(next);
    info.ovulation = Some(ov);
    info.fertile = Some((ov - Duration::days(5), ov + Duration::days(1)));
    info
}

/// Vorhergesagte Markierungen für die nächsten `cycles` Zyklen (ab der nächsten Periode, inkl. aktuellem fruchtbaren Fenster).
pub fn predicted_marks(info: &CycleInfo, cycles: i64) -> Vec<(NaiveDate, DayMark)> {
    let Some(next) = info.next_period else { return Vec::new() };
    let mut out = Vec::new();
    for c in 0..cycles.max(1) {
        let start = next + Duration::days(c * info.avg_cycle);
        let ov = start - Duration::days(14);
        for i in -5..=1 {
            let d = ov + Duration::days(i);
            out.push((d, if i == 0 { DayMark::Ovulation } else { DayMark::Fertile }));
        }
        for i in 0..info.avg_period.max(1) {
            out.push((start + Duration::days(i), DayMark::PredictedPeriod));
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn d(s: &str) -> NaiveDate {
        NaiveDate::parse_from_str(s, "%Y-%m-%d").unwrap()
    }

    fn period(start: &str, n: i64) -> Vec<CycleDay> {
        (0..n).map(|i| CycleDay { date: d(start) + Duration::days(i), flow: 2 }).collect()
    }

    #[test]
    fn defaults_without_data() {
        let info = analyze(&[], d("2026-11-20"));
        assert_eq!((info.avg_cycle, info.avg_period), (28, 5));
        assert!(info.next_period.is_none());
        assert!(!info.from_history);
    }

    #[test]
    fn groups_with_gap() {
        let mut days = period("2026-10-01", 2);
        days.extend(period("2026-10-05", 2)); // Lücke von 2 Tagen → gleiche Periode
        let ps = periods(&days);
        assert_eq!(ps.len(), 1);
        assert_eq!(ps[0].days(), 6);
    }

    #[test]
    fn prediction_from_history() {
        let mut days = period("2026-08-01", 5);
        days.extend(period("2026-08-31", 4)); // 30 Tage
        days.extend(period("2026-09-30", 5)); // 30 Tage
        days.extend(period("2026-10-28", 4)); // 28 Tage
        let today = d("2026-11-10");
        let info = analyze(&days, today);
        assert_eq!(info.avg_cycle, 29); // (30+30+28)/3 = 29.3
        assert_eq!(info.avg_period, 5); // (5+4+5+4)/4 = 4.5 → 5
        assert_eq!(info.next_period, Some(d("2026-11-26")));
        assert_eq!(info.ovulation, Some(d("2026-11-12")));
        assert_eq!(info.fertile, Some((d("2026-11-07"), d("2026-11-13"))));
        assert_eq!(info.cycle_day, Some(14));
        assert_eq!(info.late_days, 0);
        assert!(!info.in_period);
        let marks = predicted_marks(&info, 2);
        assert!(marks.contains(&(d("2026-11-26"), DayMark::PredictedPeriod)));
        assert!(marks.contains(&(d("2026-12-25"), DayMark::PredictedPeriod)));
    }

    #[test]
    fn late_period() {
        let days = period("2026-10-01", 5);
        let info = analyze(&days, d("2026-11-02"));
        assert_eq!(info.next_period, Some(d("2026-10-29")));
        assert_eq!(info.late_days, 4);
        // Lange keine Daten → nächste Periode wird in die Zukunft verschoben.
        let info = analyze(&days, d("2027-03-01"));
        assert!(info.next_period.unwrap() > d("2027-03-01"));
        assert_eq!(info.late_days, 0);
    }
}

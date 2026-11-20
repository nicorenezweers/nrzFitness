//! Hilfsfunktionen: Datum (deutsch), Zahlenformat, Nährwert-Rechnungen.
#![allow(dead_code)]

use chrono::{Datelike, Duration, Local, NaiveDate, NaiveDateTime};

pub const WEEKDAYS_SHORT: [&str; 7] = ["Mo", "Di", "Mi", "Do", "Fr", "Sa", "So"];
pub const WEEKDAYS: [&str; 7] = ["Montag", "Dienstag", "Mittwoch", "Donnerstag", "Freitag", "Samstag", "Sonntag"];
pub const MONTHS: [&str; 12] = [
    "Januar",
    "Februar",
    "März",
    "April",
    "Mai",
    "Juni",
    "Juli",
    "August",
    "September",
    "Oktober",
    "November",
    "Dezember",
];

pub fn today() -> NaiveDate {
    Local::now().date_naive()
}

pub fn now() -> NaiveDateTime {
    Local::now().naive_local()
}

/// "Freitag, 13. November"
pub fn fmt_long(d: NaiveDate) -> String {
    format!("{}, {}. {}", WEEKDAYS[d.weekday().num_days_from_monday() as usize], d.day(), MONTHS[d.month0() as usize])
}

/// "13. Nov."
pub fn fmt_short(d: NaiveDate) -> String {
    let m = MONTHS[d.month0() as usize];
    let m: String = if m.chars().count() > 4 { format!("{}.", m.chars().take(3).collect::<String>()) } else { m.into() };
    format!("{}. {}", d.day(), m)
}

/// "13.11."
pub fn fmt_dm(d: NaiveDate) -> String {
    d.format("%d.%m.").to_string()
}

/// Relativ: "Heute", "Morgen", "Gestern", Wochentag (innerhalb 6 Tagen) oder kurzes Datum.
pub fn fmt_relative(d: NaiveDate) -> String {
    let diff = (d - today()).num_days();
    match diff {
        0 => "Heute".into(),
        1 => "Morgen".into(),
        -1 => "Gestern".into(),
        -6..=-2 => WEEKDAYS[d.weekday().num_days_from_monday() as usize].into(),
        _ => fmt_short(d),
    }
}

pub fn fmt_time(dt: NaiveDateTime) -> String {
    dt.format("%H:%M").to_string()
}

/// "12 h 05 min" / "45 min"
pub fn fmt_duration_min(mins: i64) -> String {
    let mins = mins.max(0);
    if mins >= 60 { format!("{} h {:02} min", mins / 60, mins % 60) } else { format!("{mins} min") }
}

/// "16:05:09"
pub fn fmt_hms(secs: i64) -> String {
    let s = secs.max(0);
    format!("{:02}:{:02}:{:02}", s / 3600, (s / 60) % 60, s % 60)
}

pub fn parse_date(s: &str) -> Option<NaiveDate> {
    NaiveDate::parse_from_str(s.trim(), "%Y-%m-%d").ok()
}

pub fn date_input(d: NaiveDate) -> String {
    d.format("%Y-%m-%d").to_string()
}

/// `datetime-local`-Wert ("2026-11-20T08:30").
pub fn datetime_input(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M").to_string()
}

pub fn parse_datetime(s: &str) -> Option<NaiveDateTime> {
    let s = s.trim();
    NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M")
        .or_else(|_| NaiveDateTime::parse_from_str(s, "%Y-%m-%dT%H:%M:%S"))
        .ok()
}

pub fn greeting() -> &'static str {
    use chrono::Timelike;
    match Local::now().hour() {
        5..=10 => "Guten Morgen",
        11..=17 => "Guten Tag",
        18..=22 => "Guten Abend",
        _ => "Gute Nacht",
    }
}

/// Erster Tag (Montag) der Woche von `d`.
pub fn week_start(d: NaiveDate) -> NaiveDate {
    d - Duration::days(d.weekday().num_days_from_monday() as i64)
}

/// 6×7-Raster für eine Monatsansicht (beginnt am Montag).
pub fn month_grid(year: i32, month: u32) -> Vec<NaiveDate> {
    let first = NaiveDate::from_ymd_opt(year, month, 1).unwrap_or_else(today);
    let start = week_start(first);
    (0..42).map(|i| start + Duration::days(i)).collect()
}

pub fn add_months(year: i32, month: u32, delta: i32) -> (i32, u32) {
    let idx = year * 12 + month as i32 - 1 + delta;
    (idx.div_euclid(12), (idx.rem_euclid(12) + 1) as u32)
}

/// Zahl deutsch mit max. `dec` Nachkommastellen ("1,5", "12", "1'250" ohne Tausendertrennung).
pub fn num(v: f64, dec: usize) -> String {
    let s = format!("{v:.dec$}");
    let s = if s.contains('.') { s.trim_end_matches('0').trim_end_matches('.').to_string() } else { s };
    let s = if s == "-0" { "0".to_string() } else { s };
    s.replace('.', ",")
}

/// Ganzzahl mit Tausendertrennzeichen ("12’345").
pub fn int(v: f64) -> String {
    let n = v.round() as i64;
    let neg = n < 0;
    let digits = n.unsigned_abs().to_string();
    let mut out = String::new();
    for (i, c) in digits.chars().enumerate() {
        if i > 0 && (digits.len() - i) % 3 == 0 {
            out.push('’');
        }
        out.push(c);
    }
    if neg { format!("-{out}") } else { out }
}

/// Eingabe "1,5" / "1.5" → 1.5
pub fn parse_num(s: &str) -> Option<f64> {
    let t = s.trim().replace(',', ".");
    if t.is_empty() { None } else { t.parse::<f64>().ok().filter(|v| v.is_finite()) }
}

pub fn clamp01(v: f64) -> f64 {
    if v.is_finite() { v.clamp(0.0, 1.0) } else { 0.0 }
}

/// Alter in Jahren am Stichtag.
pub fn age_on(birth: NaiveDate, on: NaiveDate) -> i32 {
    let mut a = on.year() - birth.year();
    if (on.month(), on.day()) < (birth.month(), birth.day()) {
        a -= 1;
    }
    a.max(0)
}

/// Grundumsatz nach Mifflin-St Jeor (kcal/Tag).
pub fn bmr(sex: &str, weight_kg: f64, height_cm: f64, age: i32) -> f64 {
    let base = 10.0 * weight_kg + 6.25 * height_cm - 5.0 * age as f64;
    if sex == "f" { base - 161.0 } else { base + 5.0 }
}

pub fn activity_factor(level: &str) -> f64 {
    match level {
        "sedentary" => 1.2,
        "light" => 1.375,
        "moderate" => 1.55,
        "active" => 1.725,
        "very_active" => 1.9,
        _ => 1.375,
    }
}

pub const ACTIVITY_LEVELS: [(&str, &str); 5] = [
    ("sedentary", "Sitzend (kaum Bewegung)"),
    ("light", "Leicht aktiv (1–3× Sport/Woche)"),
    ("moderate", "Mässig aktiv (3–5× Sport/Woche)"),
    ("active", "Sehr aktiv (6–7× Sport/Woche)"),
    ("very_active", "Extrem aktiv (körperliche Arbeit)"),
];

pub fn bmi(weight_kg: f64, height_cm: f64) -> Option<f64> {
    (height_cm > 0.0 && weight_kg > 0.0).then(|| weight_kg / (height_cm / 100.0).powi(2))
}

pub fn bmi_label(b: f64) -> &'static str {
    match b {
        x if x < 18.5 => "Untergewicht",
        x if x < 25.0 => "Normalgewicht",
        x if x < 30.0 => "Übergewicht",
        _ => "Adipositas",
    }
}

/// Kalorien aus MET: kcal = MET × kg × Stunden.
pub fn met_kcal(met: f64, weight_kg: f64, minutes: f64) -> f64 {
    met * weight_kg * minutes / 60.0
}

/// Profilfarbe → Tailwind-Klassen (Hintergrund für Avatar).
pub fn profile_bg(color: &str) -> &'static str {
    match color {
        "sky" => "bg-sky-500/90",
        "violet" => "bg-violet-500/90",
        "rose" => "bg-rose-500/90",
        "amber" => "bg-amber-500/90",
        "teal" => "bg-teal-500/90",
        "orange" => "bg-orange-500/90",
        "slate" => "bg-slate-500/90",
        _ => "bg-emerald-500/90",
    }
}

pub const PROFILE_COLORS: [&str; 8] = ["emerald", "sky", "violet", "rose", "amber", "teal", "orange", "slate"];

pub fn initials(name: &str) -> String {
    let mut it = name.split_whitespace();
    let a = it.next().and_then(|w| w.chars().next());
    let b = it.next().and_then(|w| w.chars().next());
    match (a, b) {
        (Some(a), Some(b)) => format!("{a}{b}").to_uppercase(),
        (Some(a), None) => a.to_uppercase().to_string(),
        _ => "?".into(),
    }
}

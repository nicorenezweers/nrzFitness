//! Datenbank-Logik, ein Untermodul pro Bereich. Nur hier wird SQL geschrieben.
#![allow(dead_code)]

pub mod body;
pub mod cycle;
pub mod diary;
pub mod fasting;
pub mod foods;
pub mod meds;
pub mod profile;
pub mod reports;
pub mod seed;
pub mod training;

use chrono::{Local, NaiveDateTime};

/// Neue ID (UUID v7, zeitlich sortierbar).
pub fn new_id() -> String {
    uuid::Uuid::now_v7().to_string()
}

/// Zeitstempel für die DB, immer `%Y-%m-%dT%H:%M:%S`.
pub fn ts(dt: NaiveDateTime) -> String {
    dt.format("%Y-%m-%dT%H:%M:%S").to_string()
}

pub fn now_ts() -> String {
    ts(Local::now().naive_local())
}

/// Leere Strings → None, sonst getrimmt.
pub fn clean(s: Option<String>) -> Option<String> {
    s.map(|v| v.trim().to_string()).filter(|v| !v.is_empty())
}

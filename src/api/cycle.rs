//! Zyklus.

use chrono::NaiveDate;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::CycleDay;
#[cfg(feature = "server")]
use crate::server::{db, pool};

#[server]
pub async fn cycle_days(pid: String) -> Result<Vec<CycleDay>, ServerFnError> {
    db::cycle::days(pool(), &pid).await.map_err(e)
}

/// Stärke 1–3 setzen oder Tag entfernen (`None`).
#[server]
pub async fn set_cycle_day(pid: String, date: NaiveDate, flow: Option<i64>) -> Result<(), ServerFnError> {
    db::cycle::set_day(pool(), &pid, date, flow).await.map_err(e)
}

/// Periode ab `start` für `days` Tage eintragen.
#[server]
pub async fn start_period(pid: String, start: NaiveDate, days: i64) -> Result<(), ServerFnError> {
    db::cycle::add_range(pool(), &pid, start, days, 2).await.map_err(e)
}

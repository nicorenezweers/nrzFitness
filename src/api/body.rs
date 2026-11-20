//! Körper / Check-in.

use chrono::NaiveDate;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{BodyOverview, Checkin};
#[cfg(feature = "server")]
use crate::server::{db, pool};

#[server]
pub async fn get_checkin(pid: String, date: NaiveDate) -> Result<Option<Checkin>, ServerFnError> {
    db::body::get(pool(), &pid, date).await.map_err(e)
}

/// Ganzen Check-in speichern (leere Felder = NULL).
#[server]
pub async fn save_checkin(pid: String, checkin: Checkin) -> Result<Option<Checkin>, ServerFnError> {
    db::body::save(pool(), &pid, checkin).await.map_err(e)
}

/// Nur Gewicht eintragen (Schnelleintrag).
#[server]
pub async fn quick_weight(pid: String, date: NaiveDate, weight: f64) -> Result<(), ServerFnError> {
    db::body::set_weight(pool(), &pid, date, weight).await.map_err(e)
}

#[server]
pub async fn delete_checkin(pid: String, date: NaiveDate) -> Result<(), ServerFnError> {
    db::body::delete(pool(), &pid, date).await.map_err(e)
}

#[server]
pub async fn body_overview(pid: String) -> Result<BodyOverview, ServerFnError> {
    db::body::overview(pool(), &pid).await.map_err(e)
}

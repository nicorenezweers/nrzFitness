//! Fasten. Zeitpunkte kommen vom Client (lokale Zeit des Geräts).

use chrono::NaiveDateTime;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{Fast, FastingData};
#[cfg(feature = "server")]
use crate::server::{db, pool};

#[server]
pub async fn fasting_data(pid: String) -> Result<FastingData, ServerFnError> {
    db::fasting::data(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn start_fast(pid: String, start_at: NaiveDateTime, target_hours: f64) -> Result<Fast, ServerFnError> {
    db::fasting::start(pool(), &pid, start_at, target_hours).await.map_err(e)
}

#[server]
pub async fn end_fast(pid: String, id: String, end_at: NaiveDateTime) -> Result<Fast, ServerFnError> {
    db::fasting::end(pool(), &pid, &id, end_at).await.map_err(e)
}

#[server]
pub async fn update_fast(
    pid: String,
    id: String,
    start_at: NaiveDateTime,
    end_at: Option<NaiveDateTime>,
    target_hours: f64,
    notes: Option<String>,
) -> Result<Fast, ServerFnError> {
    db::fasting::update(pool(), &pid, &id, start_at, end_at, target_hours, notes).await.map_err(e)
}

#[server]
pub async fn delete_fast(pid: String, id: String) -> Result<(), ServerFnError> {
    db::fasting::delete(pool(), &pid, &id).await.map_err(e)
}

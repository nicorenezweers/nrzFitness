//! Medikamente und Symptome.

use chrono::{NaiveDate, NaiveDateTime};
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{Medication, MedicationInput, MedsData, SymptomsData};
#[cfg(feature = "server")]
use crate::server::{db, pool};

#[server]
pub async fn meds_data(pid: String, date: NaiveDate, today: NaiveDate) -> Result<MedsData, ServerFnError> {
    db::meds::data(pool(), &pid, date, today).await.map_err(e)
}

/// Anlegen (`id` None) oder bearbeiten.
#[server]
pub async fn save_medication(
    pid: String,
    id: Option<String>,
    input: MedicationInput,
) -> Result<Medication, ServerFnError> {
    db::meds::save(pool(), &pid, id, input).await.map_err(e)
}

#[server]
pub async fn set_medication_active(pid: String, id: String, active: bool) -> Result<(), ServerFnError> {
    db::meds::set_active(pool(), &pid, &id, active).await.map_err(e)
}

#[server]
pub async fn delete_medication(pid: String, id: String) -> Result<(), ServerFnError> {
    db::meds::delete(pool(), &pid, &id).await.map_err(e)
}

/// Einnahme umschalten; gibt `true` zurück, wenn jetzt als genommen markiert.
#[server]
pub async fn toggle_med_log(
    pid: String,
    medication_id: String,
    date: NaiveDate,
    slot: String,
    at: NaiveDateTime,
) -> Result<bool, ServerFnError> {
    db::meds::toggle_log(pool(), &pid, &medication_id, date, &slot, at).await.map_err(e)
}

#[server]
pub async fn symptoms_data(pid: String) -> Result<SymptomsData, ServerFnError> {
    db::meds::symptoms(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn add_symptom(
    pid: String,
    date: NaiveDate,
    name: String,
    severity: i64,
    notes: Option<String>,
) -> Result<(), ServerFnError> {
    db::meds::add_symptom(pool(), &pid, date, name, severity, notes).await.map_err(e)
}

#[server]
pub async fn delete_symptom(pid: String, id: String) -> Result<(), ServerFnError> {
    db::meds::delete_symptom(pool(), &pid, &id).await.map_err(e)
}

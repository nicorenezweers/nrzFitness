//! Globaler App-Zustand: aktives Profil, Neu-Laden, Toast, globale Sheets.

use chrono::NaiveDate;
use dioxus::prelude::*;

use crate::model::Profile;

/// Anfrage, das „Essen hinzufügen“-Sheet zu öffnen.
#[derive(Clone, PartialEq, Debug)]
pub struct FoodSheetReq {
    pub date: NaiveDate,
    pub meal: String,
}

#[derive(Clone, Copy)]
pub struct AppCtx {
    /// Aktives Profil (unterhalb von `ProfileGate` immer gesetzt).
    pub profile: Signal<Option<Profile>>,
    pub profiles: Signal<Vec<Profile>>,
    /// Hochzählen, wenn Daten ausserhalb der Seite geändert wurden (Sheets, Schnellaktionen).
    pub refresh: Signal<u64>,
    pub toast: Signal<Option<(String, bool)>>,
    pub food_sheet: Signal<Option<FoodSheetReq>>,
    /// „Mehr“-Menü (mobil) offen?
    pub more_open: Signal<bool>,
    /// Schnellaktionen (+) offen?
    pub quick_open: Signal<bool>,
    /// Gewählter Tag im Tagebuch (bleibt beim Seitenwechsel erhalten).
    pub date: Signal<NaiveDate>,
}

pub fn use_app() -> AppCtx {
    use_context::<AppCtx>()
}

impl AppCtx {
    /// ID des aktiven Profils (leer, falls keins – kommt unterhalb von ProfileGate nicht vor).
    pub fn pid(&self) -> String {
        self.profile.peek().as_ref().map(|p| p.id.clone()).unwrap_or_default()
    }

    /// Reaktiv: liest das Profil-Signal mit (für use_resource-Abhängigkeiten).
    pub fn pid_tracked(&self) -> String {
        self.profile.read().as_ref().map(|p| p.id.clone()).unwrap_or_default()
    }

    pub fn profile(&self) -> Option<Profile> {
        self.profile.read().clone()
    }

    pub fn bump(&mut self) {
        self.refresh += 1;
    }

    pub fn toast(&mut self, msg: impl Into<String>) {
        self.toast.set(Some((msg.into(), false)));
    }

    pub fn error(&mut self, msg: impl std::fmt::Display) {
        self.toast.set(Some((clean_err(msg), true)));
    }

    pub fn open_food(&mut self, date: NaiveDate, meal: impl Into<String>) {
        self.food_sheet.set(Some(FoodSheetReq { date, meal: meal.into() }));
    }
}

/// Aktives Profil im Browser merken.
pub fn remember_profile(id: &str) {
    let _ = document::eval(&format!("localStorage.setItem('nrzfitness.profile', {});", serde_json::json!(id)));
}

/// Fehlertext ohne technische Präfixe von Server-Funktionen.
pub fn clean_err(e: impl std::fmt::Display) -> String {
    let m = e.to_string();
    let m = m.strip_prefix("error running server function: ").unwrap_or(&m);
    let m = m.split(" (details:").next().unwrap_or(m);
    m.trim().to_string()
}

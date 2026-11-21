//! Server-Funktionen für die UI, ein Untermodul pro Bereich.
//! Jede Funktion bekommt die Profil-ID (`pid`) des aktiven Profils.
#![allow(unused_imports)]

pub mod ai;
pub mod body;
pub mod cycle;
pub mod diary;
pub mod fasting;
pub mod health;
pub mod foods;
pub mod meds;
pub mod profile;
pub mod reports;
pub mod training;

#[cfg(feature = "server")]
pub(crate) fn e(err: crate::server::AppError) -> dioxus::prelude::ServerFnError {
    dioxus::prelude::ServerFnError::new(err.to_string())
}

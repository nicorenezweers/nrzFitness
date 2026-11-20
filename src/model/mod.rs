//! Gemeinsame Datentypen für UI und Server-Funktionen (ein Untermodul pro Bereich).
#![allow(unused_imports)]

pub mod ai;
pub mod body;
pub mod cycle;
pub mod diary;
pub mod fasting;
pub mod meds;
pub mod profile;
pub mod reports;
pub mod training;

pub use ai::*;
pub use body::*;
pub use cycle::*;
pub use diary::*;
pub use fasting::*;
pub use meds::*;
pub use profile::*;
pub use reports::*;
pub use training::*;

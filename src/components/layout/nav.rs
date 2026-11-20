//! Navigationseinträge (gemeinsam für Sidebar, Tab-Leiste und „Mehr“-Menü).

use dioxus::prelude::*;
use icons::{ChartColumn, Dumbbell, Flower2, Pill, Scale, Settings, Sun, Target, Timer, Utensils};

use crate::app::Route;

#[derive(Clone, Copy, PartialEq)]
pub struct NavItem {
    pub label: &'static str,
    pub path: &'static str,
    pub icon: &'static str,
}

pub const NAV: [NavItem; 10] = [
    NavItem { label: "Heute", path: "/", icon: "today" },
    NavItem { label: "Lebensmittel", path: "/lebensmittel", icon: "foods" },
    NavItem { label: "Training", path: "/training", icon: "training" },
    NavItem { label: "Körper", path: "/koerper", icon: "body" },
    NavItem { label: "Fasten", path: "/fasten", icon: "fasting" },
    NavItem { label: "Medikamente", path: "/medikamente", icon: "meds" },
    NavItem { label: "Zyklus", path: "/zyklus", icon: "cycle" },
    NavItem { label: "Ziele", path: "/ziele", icon: "goals" },
    NavItem { label: "Berichte", path: "/berichte", icon: "reports" },
    NavItem { label: "Einstellungen", path: "/einstellungen", icon: "settings" },
];

pub fn route_for(path: &str) -> Route {
    match path {
        "/lebensmittel" => Route::Foods {},
        "/training" => Route::Training {},
        "/koerper" => Route::Body {},
        "/fasten" => Route::Fasting {},
        "/medikamente" => Route::Meds {},
        "/zyklus" => Route::Cycle {},
        "/ziele" => Route::Goals {},
        "/berichte" => Route::Reports {},
        "/einstellungen" => Route::Settings {},
        _ => Route::Today {},
    }
}

#[component]
pub fn NavIcon(name: &'static str, #[props(into, default = "size-5".to_string())] class: String) -> Element {
    let c = class.as_str();
    match name {
        "today" => rsx! { Sun { class: c } },
        "foods" => rsx! { Utensils { class: c } },
        "training" => rsx! { Dumbbell { class: c } },
        "body" => rsx! { Scale { class: c } },
        "fasting" => rsx! { Timer { class: c } },
        "meds" => rsx! { Pill { class: c } },
        "cycle" => rsx! { Flower2 { class: c } },
        "goals" => rsx! { Target { class: c } },
        "reports" => rsx! { ChartColumn { class: c } },
        _ => rsx! { Settings { class: c } },
    }
}

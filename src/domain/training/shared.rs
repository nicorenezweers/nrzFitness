//! Kleine Bausteine für den Trainings-Bereich.

use dioxus::prelude::*;
use icons::{Activity, Dumbbell, PersonStanding, Volleyball};

use crate::components::ui::field::INPUT;
use crate::model::{Exercise, ExerciseEntry};
use crate::util::{int, num};

/// Symbol je Kategorie in einer runden Kachel.
#[component]
pub fn CategoryIcon(
    #[props(into)] category: String,
    #[props(default = "size-10".to_string())] size: String,
) -> Element {
    let icon = match category.as_str() {
        "cardio" => rsx! { Activity { class: "size-5" } },
        "flexibility" => rsx! { PersonStanding { class: "size-5" } },
        "sport" => rsx! { Volleyball { class: "size-5" } },
        _ => rsx! { Dumbbell { class: "size-5" } },
    };
    let color = match category.as_str() {
        "cardio" => "text-water",
        "flexibility" => "text-protein",
        "sport" => "text-carbs",
        _ => "text-kcal",
    };
    rsx! {
        div { class: "flex {size} shrink-0 items-center justify-center rounded-xl bg-muted {color}", {icon} }
    }
}

/// Grosses, zentriertes Zahlenfeld (Satz-Tabelle) mit Dezimal-/Zifferntastatur.
#[component]
pub fn BigNum(
    value: String,
    oninput: EventHandler<String>,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(default = false)] integer: bool,
) -> Element {
    rsx! {
        input {
            class: "{INPUT} h-12 px-1 text-center text-lg font-semibold tabular-nums sm:h-11",
            r#type: "text",
            inputmode: if integer { "numeric" } else { "decimal" },
            autocomplete: "off",
            enterkeyhint: "next",
            placeholder: placeholder.clone().unwrap_or_default(),
            value: "{value}",
            onfocus: move |_| {
                // Inhalt markieren, damit man direkt überschreiben kann.
                let _ = document::eval("document.activeElement && document.activeElement.select && document.activeElement.select();");
            },
            oninput: move |e| oninput.call(e.value()),
        }
    }
}

/// "12’345 kg" bzw. "12,3 t" ab 10 t.
pub fn fmt_volume(kg: f64) -> String {
    if kg >= 10_000.0 { format!("{} t", num(kg / 1000.0, 1)) } else { format!("{} kg", int(kg)) }
}

/// Untertitel eines Eintrags: Kraft "3 Sätze · 1’800 kg", sonst Dauer · Distanz · Pace.
pub fn entry_subtitle(e: &ExerciseEntry) -> String {
    if e.is_strength() {
        let mut parts = vec![format!("{} {}", e.sets.len(), if e.sets.len() == 1 { "Satz" } else { "Sätze" })];
        let v = e.volume();
        if v > 0.0 {
            parts.push(fmt_volume(v));
        }
        if e.duration_min > 0.0 {
            parts.push(format!("{} min", num(e.duration_min, 0)));
        }
        parts.join(" · ")
    } else {
        let mut parts = Vec::new();
        if e.duration_min > 0.0 {
            parts.push(crate::util::fmt_duration_min(e.duration_min.round() as i64));
        }
        if let Some(km) = e.distance_km.filter(|k| *k > 0.0) {
            parts.push(format!("{} km", num(km, 2)));
            if let Some(p) = crate::model::pace(e.duration_min, km) {
                parts.push(p);
            }
        }
        if parts.is_empty() {
            parts.push(crate::model::category_label(&e.category).to_string());
        }
        parts.join(" · ")
    }
}

/// Übung zu einem Eintrag (aus der Bibliothek oder behelfsmässig aus dem Eintrag).
pub fn exercise_for(entry: &ExerciseEntry, all: &[Exercise]) -> Exercise {
    entry.exercise_id.as_ref().and_then(|id| all.iter().find(|x| &x.id == id)).cloned().unwrap_or_else(|| Exercise {
        id: String::new(),
        name: entry.name.clone(),
        category: entry.category.clone(),
        met: if entry.is_strength() { 5.0 } else { 6.0 },
        ..Default::default()
    })
}

/// Kategorie-Optionen für Chips (mit „Alle“).
pub fn category_options(with_all: bool) -> Vec<(String, String)> {
    let mut v: Vec<(String, String)> = Vec::new();
    if with_all {
        v.push(("all".into(), "Alle".into()));
    }
    v.extend(crate::model::CATEGORIES.iter().map(|(k, l)| (k.to_string(), l.to_string())));
    v
}

/// Filter für Suche + Kategorie.
pub fn matches(e: &Exercise, q: &str, cat: &str) -> bool {
    if cat != "all" && e.category != cat {
        return false;
    }
    if q.is_empty() {
        return true;
    }
    let hay = format!("{} {} {}", e.name, e.muscle.as_deref().unwrap_or(""), e.equipment.as_deref().unwrap_or(""))
        .to_lowercase();
    q.split_whitespace().all(|w| hay.contains(w))
}

/// Zeile einer Übung in Listen (gross genug zum Tippen).
#[component]
pub fn ExerciseRow(
    exercise: Exercise,
    onclick: EventHandler<()>,
    #[props(optional)] trailing: Option<Element>,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "flex min-h-14 w-full items-center gap-3 rounded-xl px-2 py-2 text-left transition-colors hover:bg-accent active:bg-accent",
            onclick: move |_| onclick.call(()),
            CategoryIcon { category: exercise.category.clone() }
            div { class: "min-w-0 flex-1",
                p { class: "truncate text-[15px] font-medium", "{exercise.name}" }
                p { class: "truncate text-xs text-muted-foreground", "{exercise.subtitle()}" }
            }
            if exercise.custom {
                span { class: "shrink-0 rounded-md bg-muted px-1.5 py-0.5 text-[10px] font-medium text-muted-foreground", "Eigene" }
            }
            {trailing}
        }
    }
}

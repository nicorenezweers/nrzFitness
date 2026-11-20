//! Übungsbibliothek: suchen, filtern, eigene Übungen verwalten, Details mit Fortschritt.

use dioxus::prelude::*;
use icons::{ChevronRight, Pencil, Plus, Search};

use super::exercise_form::ExerciseForm;
use super::history::ProgressChart;
use super::log_sheet::LogSheet;
use super::page_training::use_training;
use super::shared::{CategoryIcon, ExerciseRow, category_options, matches};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::Card;
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::field::INPUT;
use crate::components::ui::segmented::Chips;
use crate::components::ui::sheet::Sheet;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{Exercise, MUSCLES, category_label};
use crate::state::use_app;
use crate::util::{fmt_relative, num};

#[component]
pub fn LibraryView() -> Element {
    let mut app = use_app();
    let mut t = use_training();
    let lib = t.lib();
    let loading = t.library.read().is_none();
    let mut q = use_signal(String::new);
    let mut cat = use_signal(|| "all".to_string());
    let mut creating = use_signal(|| false);
    let mut detail: Signal<Option<Exercise>> = use_signal(|| None);
    let mut editing: Signal<Option<Exercise>> = use_signal(|| None);
    let mut logging: Signal<Option<Exercise>> = use_signal(|| None);

    let ql = q().trim().to_lowercase();
    let c = cat();
    let list: Vec<Exercise> = lib.exercises.iter().filter(|e| matches(e, &ql, &c)).cloned().collect();
    // Nach Muskelgruppe gruppieren (Reihenfolge wie MUSCLES, Rest unter „Andere“).
    let mut groups: Vec<(String, Vec<Exercise>)> = MUSCLES
        .iter()
        .map(|m| (m.to_string(), list.iter().filter(|e| e.muscle.as_deref() == Some(*m)).cloned().collect::<Vec<_>>()))
        .collect();
    groups.push((
        "Andere".into(),
        list.iter().filter(|e| e.muscle.as_deref().is_none_or(|m| !MUSCLES.contains(&m))).cloned().collect(),
    ));
    groups.retain(|g| !g.1.is_empty());
    let custom_count = lib.exercises.iter().filter(|e| e.custom).count();
    let total = list.len();

    rsx! {
        div { class: "flex flex-col gap-3",
            div { class: "flex items-center gap-2",
                div { class: "relative flex-1",
                    Search { class: "pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" }
                    input {
                        class: "{INPUT} pl-10",
                        r#type: "search",
                        placeholder: "Übung suchen …",
                        enterkeyhint: "search",
                        value: "{q}",
                        oninput: move |e| q.set(e.value()),
                    }
                }
                Button { class: "h-11 shrink-0 sm:h-10", onclick: move |_| creating.set(true),
                    Plus { class: "size-4" }
                    span { class: "hidden sm:inline", "Eigene Übung" }
                    span { class: "sm:hidden", "Eigene" }
                }
            }
            Chips { options: category_options(true), value: c.clone(), onchange: move |v| cat.set(v) }
            p { class: "px-1 text-xs text-muted-foreground", "{total} Übungen · {custom_count} eigene" }

            if loading {
                Skeleton { class: "h-64 rounded-2xl" }
            } else if total == 0 {
                Empty {
                    EmptyTitle { "Keine Übung gefunden" }
                    EmptyDescription { "Lege sie als eigene Übung an." }
                }
            }
            div { class: "grid gap-4 lg:grid-cols-2 lg:items-start",
                for (muscle, items) in groups {
                    Card { key: "{muscle}", class: "p-2",
                        p { class: "px-2 pb-1 pt-2 text-xs font-semibold uppercase tracking-wide text-muted-foreground", "{muscle} · {items.len()}" }
                        for ex in items {
                            ExerciseRow {
                                key: "{ex.id}",
                                exercise: ex.clone(),
                                onclick: move |_| detail.set(Some(ex.clone())),
                                trailing: rsx! { ChevronRight { class: "size-4 shrink-0 text-muted-foreground" } },
                            }
                        }
                    }
                }
            }
        }

        if let Some(ex) = detail() {
            ExerciseDetail {
                key: "{ex.id}",
                exercise: ex,
                onclose: move |_| detail.set(None),
                onlog: move |ex: Exercise| {
                    detail.set(None);
                    logging.set(Some(ex));
                },
                onedit: move |ex: Exercise| {
                    detail.set(None);
                    editing.set(Some(ex));
                },
            }
        }
        if creating() {
            ExerciseForm {
                initial: None,
                preset_category: if c == "all" { "strength".to_string() } else { c.clone() },
                preset_name: q().trim().to_string(),
                onclose: move |_| creating.set(false),
                onsaved: move |_| {
                    creating.set(false);
                    t.library.restart();
                },
            }
        }
        if let Some(ex) = editing() {
            ExerciseForm {
                initial: Some(ex),
                onclose: move |_| editing.set(None),
                onsaved: move |_| {
                    editing.set(None);
                    app.bump();
                },
            }
        }
        if let Some(ex) = logging() {
            LogSheet {
                exercise: ex,
                entry: None,
                date: (app.date)(),
                onclose: move |_| logging.set(None),
                onsaved: move |_| {
                    logging.set(None);
                    app.bump();
                },
            }
        }
    }
}

#[component]
fn ExerciseDetail(
    exercise: Exercise,
    onclose: EventHandler<()>,
    onlog: EventHandler<Exercise>,
    onedit: EventHandler<Exercise>,
) -> Element {
    let app = use_app();
    let date = (app.date)();
    let ex_log = exercise.clone();
    let ex_edit = exercise.clone();
    let chip = "rounded-full bg-muted px-3 py-1 text-xs font-medium";
    rsx! {
        Sheet {
            title: exercise.name.clone(),
            onclose: move |_| onclose.call(()),
            header_action: rsx! { CategoryIcon { category: exercise.category.clone(), size: "size-9" } },
            footer: rsx! {
                div { class: "flex gap-2",
                    if exercise.custom {
                        Button { variant: ButtonVariant::Outline, size: ButtonSize::Lg, onclick: move |_| onedit.call(ex_edit.clone()),
                            Pencil { class: "size-4" }
                            "Bearbeiten"
                        }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", onclick: move |_| onlog.call(ex_log.clone()),
                        Plus { class: "size-5" }
                        "Eintragen · {fmt_relative(date)}"
                    }
                }
            },
            div { class: "flex flex-col gap-4 px-4 pb-5 pt-1 sm:px-5",
                div { class: "flex flex-wrap gap-1.5",
                    span { class: "{chip}", "{category_label(&exercise.category)}" }
                    if let Some(m) = exercise.muscle.as_ref() {
                        span { class: "{chip}", "{m}" }
                    }
                    if let Some(e) = exercise.equipment.as_ref() {
                        span { class: "{chip}", "{e}" }
                    }
                    span { class: "{chip} text-muted-foreground", "{num(exercise.met, 1)} MET" }
                    if exercise.custom {
                        span { class: "{chip} text-muted-foreground", "Eigene Übung" }
                    }
                }
                ProgressChart { exercise_id: exercise.id.clone(), strength: exercise.is_strength() }
            }
        }
    }
}

//! Tagesansicht: Datum, Zusammenfassung, Einträge, Vorlagen.

use dioxus::prelude::*;
use icons::{Check, ChevronRight, Clock, Dumbbell, Flame, ListChecks, Plus, Weight};

use super::log_sheet::LogSheet;
use super::page_training::use_training;
use super::picker::ExercisePicker;
use super::shared::{CategoryIcon, entry_subtitle, exercise_for, fmt_volume};
use super::templates::TemplatesPanel;
use crate::api::training::{get_training_day, set_exercise_set_done};
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::card::Card;
use crate::components::ui::date_nav::DateNav;
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::skeleton::Skeleton;
use crate::model::{Exercise, ExerciseEntry, day_summary};
use crate::state::use_app;
use crate::util::{int, num};

#[component]
pub fn DayView() -> Element {
    let mut app = use_app();
    let t = use_training();
    let mut day = use_resource(move || async move {
        let _ = (app.refresh)();
        let pid = app.pid_tracked();
        let date = (app.date)();
        get_training_day(pid, date).await
    });
    let mut picker = use_signal(|| false);
    let mut logging: Signal<Option<(Exercise, Option<ExerciseEntry>)>> = use_signal(|| None);

    let date = (app.date)();
    let state = day.read().clone();
    let entries: Vec<ExerciseEntry> = state.as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default();
    let s = day_summary(&entries);
    let lib = t.lib();
    let open_sets = entries.iter().any(|e| e.sets.iter().any(|s| !s.done));

    rsx! {
        div { class: "flex flex-col gap-4 lg:grid lg:grid-cols-[minmax(0,1fr)_20rem] lg:items-start lg:gap-6",
            div { class: "flex min-w-0 flex-col gap-4",
                div { class: "flex items-center justify-between gap-2",
                    DateNav { date, onchange: move |d| app.date.set(d) }
                    Button { class: "hidden sm:inline-flex", onclick: move |_| picker.set(true),
                        Plus { class: "size-4" }
                        "Übung"
                    }
                }

                // Zusammenfassung
                Card { class: "grid grid-cols-4 divide-x py-3",
                    SummaryTile { label: "Minuten", value: int(s.minutes), icon: rsx! { Clock { class: "size-3.5" } } }
                    SummaryTile { label: "kcal", value: int(s.kcal), color: "text-kcal", icon: rsx! { Flame { class: "size-3.5" } } }
                    SummaryTile {
                        label: "Volumen",
                        value: if s.volume > 0.0 { fmt_volume(s.volume) } else { "–".to_string() },
                        icon: rsx! { Weight { class: "size-3.5" } },
                    }
                    SummaryTile { label: "Übungen", value: s.count.to_string(), icon: rsx! { Dumbbell { class: "size-3.5" } } }
                }

                match state {
                    None => rsx! {
                        Skeleton { class: "h-24 rounded-2xl" }
                        Skeleton { class: "h-24 rounded-2xl" }
                    },
                    Some(Err(e)) => rsx! {
                        Empty { EmptyTitle { "Fehler beim Laden" } EmptyDescription { "{e}" } }
                    },
                    Some(Ok(_)) if entries.is_empty() => rsx! {
                        Empty { class: "py-10",
                            div { class: "flex size-14 items-center justify-center rounded-2xl bg-muted text-kcal", Dumbbell { class: "size-7" } }
                            EmptyTitle { "Noch kein Training an diesem Tag" }
                            EmptyDescription { "Füge eine Übung hinzu oder starte eine Vorlage." }
                        }
                    },
                    Some(Ok(_)) => rsx! {
                        div { class: "flex flex-col gap-3",
                            for e in entries.iter().cloned() {
                                EntryCard {
                                    key: "{e.id}",
                                    entry: e.clone(),
                                    onedit: {
                                        let ex = exercise_for(&e, &lib.exercises);
                                        let e = e.clone();
                                        move |_| logging.set(Some((ex.clone(), Some(e.clone()))))
                                    },
                                    ontoggle: move |_| day.restart(),
                                }
                            }
                            if open_sets {
                                p { class: "flex items-center gap-1.5 px-1 text-xs text-muted-foreground",
                                    ListChecks { class: "size-3.5" }
                                    "Satz antippen zum Abhaken – danach startet der Pausen-Timer."
                                }
                            }
                        }
                    },
                }

                Button { size: ButtonSize::Lg, class: "w-full", onclick: move |_| picker.set(true),
                    Plus { class: "size-5" }
                    "Übung hinzufügen"
                }
            }

            div { class: "flex min-w-0 flex-col gap-4",
                TemplatesPanel { date, has_entries: !entries.is_empty() }
            }
        }

        if picker() {
            ExercisePicker {
                title: "Übung hinzufügen",
                onclose: move |_| picker.set(false),
                onpick: move |ex: Exercise| {
                    picker.set(false);
                    logging.set(Some((ex, None)));
                },
            }
        }
        if let Some((ex, entry)) = logging() {
            LogSheet {
                key: "{ex.id}-{entry.as_ref().map(|e| e.id.clone()).unwrap_or_default()}",
                exercise: ex,
                entry,
                date,
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
fn SummaryTile(
    #[props(into)] label: String,
    value: String,
    #[props(into, default)] color: String,
    icon: Element,
) -> Element {
    rsx! {
        div { class: "flex min-w-0 flex-col items-center gap-0.5 px-1",
            span { class: "truncate text-base font-semibold tabular-nums sm:text-lg {color}", "{value}" }
            span { class: "flex items-center gap-1 text-[11px] text-muted-foreground", {icon} "{label}" }
        }
    }
}

#[component]
fn EntryCard(entry: ExerciseEntry, onedit: EventHandler<()>, ontoggle: EventHandler<()>) -> Element {
    let mut app = use_app();
    let mut t = use_training();
    let sub = entry_subtitle(&entry);
    rsx! {
        Card { class: "overflow-hidden",
            button {
                r#type: "button",
                class: "flex min-h-16 w-full items-center gap-3 px-3 py-3 text-left transition-colors hover:bg-accent/40 active:bg-accent/60",
                onclick: move |_| onedit.call(()),
                CategoryIcon { category: entry.category.clone() }
                div { class: "min-w-0 flex-1",
                    p { class: "truncate text-[15px] font-semibold", "{entry.name}" }
                    p { class: "truncate text-xs text-muted-foreground", "{sub}" }
                }
                div { class: "flex shrink-0 flex-col items-end leading-tight",
                    span { class: "text-base font-semibold tabular-nums text-kcal", "{int(entry.kcal)}" }
                    span { class: "text-[10px] text-muted-foreground", "kcal" }
                }
                ChevronRight { class: "size-4 shrink-0 text-muted-foreground" }
            }
            if entry.is_strength() && !entry.sets.is_empty() {
                div { class: "flex flex-wrap gap-1.5 px-3 pb-3",
                    for (i, s) in entry.sets.iter().cloned().enumerate() {
                        button {
                            key: "{s.id}",
                            r#type: "button",
                            class: "flex h-10 items-center gap-1.5 rounded-xl border px-2.5 text-sm tabular-nums transition-all active:scale-95",
                            class: if s.done { "border-good/40 bg-good/10" } else { "border-dashed text-muted-foreground" },
                            onclick: move |_| {
                                let id = s.id.clone();
                                let done = !s.done;
                                async move {
                                    match set_exercise_set_done(app.pid(), id, done).await {
                                        Ok(()) => {
                                            if done {
                                                t.start_rest();
                                            }
                                            ontoggle.call(());
                                        }
                                        Err(e) => app.error(e),
                                    }
                                }
                            },
                            span { class: "text-[10px] font-semibold text-muted-foreground", "{i + 1}" }
                            if s.weight_kg > 0.0 {
                                span { class: "font-medium", "{s.reps} × {num(s.weight_kg, 2)}" }
                                span { class: "text-[10px] text-muted-foreground", "kg" }
                            } else {
                                span { class: "font-medium", "{s.reps} Wdh." }
                            }
                            if s.done {
                                Check { class: "size-3.5 text-good" }
                            }
                        }
                    }
                }
            }
            if let Some(n) = entry.notes.as_ref() {
                p { class: "-mt-1 truncate px-4 pb-3 text-xs italic text-muted-foreground", "{n}" }
            }
        }
    }
}

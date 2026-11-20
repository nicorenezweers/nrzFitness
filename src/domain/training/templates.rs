//! Trainings-Vorlagen (Routinen): Liste, mit einem Tipp eintragen, bearbeiten, aus einem Tag erstellen.

use chrono::NaiveDate;
use dioxus::prelude::*;
use icons::{BookmarkPlus, ChevronDown, ChevronUp, Play, Plus, Trash2, X};

use super::picker::ExercisePicker;
use super::shared::CategoryIcon;
use crate::api::training::{
    apply_workout_template, delete_workout_template, list_workout_templates, save_workout_template, template_from_day,
};
use crate::components::layout::page::Section;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::Card;
use crate::components::ui::field::{Field, INPUT, NumberInput};
use crate::components::ui::sheet::Sheet;
use crate::model::{Exercise, TemplateItemInput, WorkoutTemplate};
use crate::state::use_app;
use crate::util::{fmt_short, num, parse_num};

#[component]
pub fn TemplatesPanel(date: NaiveDate, has_entries: bool) -> Element {
    let mut app = use_app();
    let mut list = use_resource(move || async move {
        let _ = (app.refresh)();
        let pid = app.pid_tracked();
        list_workout_templates(pid).await
    });
    // None = zu, Some(None) = neu, Some(Some(t)) = bearbeiten
    let mut editing: Signal<Option<Option<WorkoutTemplate>>> = use_signal(|| None);
    let mut save_day = use_signal(|| false);
    let mut busy = use_signal(|| None::<String>);

    let templates: Vec<WorkoutTemplate> =
        list.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default();

    rsx! {
        Section {
            title: "Vorlagen",
            count: templates.len(),
            action: rsx! {
                button {
                    r#type: "button",
                    class: "flex h-8 items-center gap-1 rounded-full px-2.5 text-xs font-medium text-muted-foreground hover:bg-accent hover:text-foreground",
                    onclick: move |_| editing.set(Some(None)),
                    Plus { class: "size-3.5" }
                    "Neu"
                }
            },
            if templates.is_empty() {
                Card { class: "p-4 text-sm text-muted-foreground",
                    "Speichere wiederkehrende Workouts (z.B. „Push“, „Beine“) als Vorlage und trage sie mit einem Tipp ein."
                }
            }
            for tpl in templates.iter().cloned() {
                Card { key: "{tpl.id}", class: "flex items-center gap-2 p-2 pl-4",
                    button {
                        r#type: "button",
                        class: "min-w-0 flex-1 py-1.5 text-left",
                        onclick: {
                            let tpl = tpl.clone();
                            move |_| editing.set(Some(Some(tpl.clone())))
                        },
                        p { class: "truncate text-[15px] font-semibold", "{tpl.name}" }
                        p { class: "truncate text-xs text-muted-foreground",
                            "{tpl.items.len()} Übungen · "
                            {tpl.items.iter().map(|i| i.exercise.name.clone()).collect::<Vec<_>>().join(", ")}
                        }
                    }
                    button {
                        r#type: "button",
                        aria_label: "Vorlage eintragen",
                        disabled: busy().is_some(),
                        class: "flex size-11 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-sm transition-transform active:scale-90 disabled:opacity-50",
                        onclick: {
                            let id = tpl.id.clone();
                            let name = tpl.name.clone();
                            move |_| {
                                let id = id.clone();
                                let name = name.clone();
                                async move {
                                    busy.set(Some(id.clone()));
                                    let r = apply_workout_template(app.pid(), id, date).await;
                                    busy.set(None);
                                    match r {
                                        Ok(n) => {
                                            app.toast(format!("„{name}“ eingetragen ({n} Übungen) · {}", fmt_short(date)));
                                            app.bump();
                                        }
                                        Err(e) => app.error(e),
                                    }
                                }
                            }
                        },
                        Play { class: "size-5 translate-x-px" }
                    }
                }
            }
            if has_entries {
                Button { variant: ButtonVariant::Outline, class: "w-full", onclick: move |_| save_day.set(true),
                    BookmarkPlus { class: "size-4" }
                    "Tag als Vorlage speichern"
                }
            }
        }

        if let Some(initial) = editing() {
            TemplateSheet {
                initial,
                onclose: move |_| editing.set(None),
                onsaved: move |_| {
                    editing.set(None);
                    list.restart();
                },
            }
        }
        if save_day() {
            SaveDaySheet {
                date,
                onclose: move |_| save_day.set(false),
                onsaved: move |_| {
                    save_day.set(false);
                    list.restart();
                },
            }
        }
    }
}

#[derive(Clone, PartialEq)]
struct ItemDraft {
    exercise: Exercise,
    sets: String,
    reps: String,
    kg: String,
    dur: String,
}

#[component]
fn TemplateSheet(
    #[props(!optional)] initial: Option<WorkoutTemplate>,
    onclose: EventHandler<()>,
    onsaved: EventHandler<()>,
) -> Element {
    let mut app = use_app();
    let init = initial.clone();
    let mut name = use_signal(|| init.as_ref().map(|t| t.name.clone()).unwrap_or_default());
    let mut notes = use_signal(|| init.as_ref().and_then(|t| t.notes.clone()).unwrap_or_default());
    let mut items = use_signal(|| {
        init.as_ref()
            .map(|t| {
                t.items
                    .iter()
                    .map(|i| ItemDraft {
                        exercise: i.exercise.clone(),
                        sets: i.sets.to_string(),
                        reps: i.reps.to_string(),
                        kg: if i.weight_kg > 0.0 { num(i.weight_kg, 2) } else { String::new() },
                        dur: if i.duration_min > 0.0 { num(i.duration_min, 0) } else { String::new() },
                    })
                    .collect::<Vec<_>>()
            })
            .unwrap_or_default()
    });
    let mut picking = use_signal(|| false);
    let mut busy = use_signal(|| false);
    let mut confirm = use_signal(|| false);
    let id = initial.as_ref().map(|t| t.id.clone());
    let id_del = id.clone();

    let save = move |_| {
        let id = id.clone();
        async move {
            let list: Vec<TemplateItemInput> = items
                .read()
                .iter()
                .map(|d| TemplateItemInput {
                    exercise_id: d.exercise.id.clone(),
                    sets: parse_num(&d.sets).unwrap_or(3.0).round() as i64,
                    reps: parse_num(&d.reps).unwrap_or(10.0).round() as i64,
                    weight_kg: parse_num(&d.kg).unwrap_or(0.0),
                    duration_min: parse_num(&d.dur).unwrap_or(0.0),
                })
                .collect();
            busy.set(true);
            let r = save_workout_template(app.pid(), id, name(), Some(notes()).filter(|n| !n.trim().is_empty()), list)
                .await;
            busy.set(false);
            match r {
                Ok(_) => {
                    app.toast("Vorlage gespeichert");
                    onsaved.call(());
                }
                Err(e) => app.error(e),
            }
        }
    };
    let delete = move |_| {
        let id = id_del.clone();
        async move {
            let Some(id) = id else { return };
            if !confirm() {
                confirm.set(true);
                return;
            }
            match delete_workout_template(app.pid(), id).await {
                Ok(()) => {
                    app.toast("Vorlage gelöscht");
                    onsaved.call(());
                }
                Err(e) => app.error(e),
            }
        }
    };

    let rows = items.read().clone();
    let n = rows.len();
    let editing = initial.is_some();
    rsx! {
        Sheet {
            title: if editing { "Vorlage bearbeiten" } else { "Neue Vorlage" },
            tall: true,
            onclose: move |_| onclose.call(()),
            footer: rsx! {
                div { class: "flex gap-2",
                    if editing {
                        Button {
                            variant: if confirm() { ButtonVariant::Destructive } else { ButtonVariant::Outline },
                            size: ButtonSize::Lg,
                            class: if confirm() { "" } else { "w-12 px-0" },
                            onclick: delete,
                            Trash2 { class: "size-5" }
                            if confirm() { "Löschen?" }
                        }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", disabled: busy() || n == 0, onclick: save, "Speichern" }
                }
            },
            div { class: "flex flex-col gap-4 px-4 pb-5 pt-1 sm:px-5",
                Field { label: "Name",
                    input { class: "{INPUT}", value: "{name}", placeholder: "z.B. Push, Beine, Ganzkörper A", oninput: move |e| name.set(e.value()) }
                }
                div { class: "flex flex-col gap-2",
                    span { class: "px-1 text-xs font-medium text-muted-foreground", "Übungen ({n})" }
                    for (i, d) in rows.into_iter().enumerate() {
                        div { key: "{i}-{d.exercise.id}", class: "flex flex-col gap-2 rounded-2xl border p-3",
                            div { class: "flex items-center gap-2",
                                CategoryIcon { category: d.exercise.category.clone(), size: "size-8" }
                                p { class: "min-w-0 flex-1 truncate text-sm font-semibold", "{d.exercise.name}" }
                                button {
                                    r#type: "button",
                                    aria_label: "Nach oben",
                                    disabled: i == 0,
                                    class: "flex size-9 items-center justify-center rounded-full text-muted-foreground hover:bg-accent disabled:opacity-30",
                                    onclick: move |_| items.with_mut(|v| if i > 0 { v.swap(i, i - 1) }),
                                    ChevronUp { class: "size-4" }
                                }
                                button {
                                    r#type: "button",
                                    aria_label: "Nach unten",
                                    disabled: i + 1 >= n,
                                    class: "flex size-9 items-center justify-center rounded-full text-muted-foreground hover:bg-accent disabled:opacity-30",
                                    onclick: move |_| items.with_mut(|v| if i + 1 < v.len() { v.swap(i, i + 1) }),
                                    ChevronDown { class: "size-4" }
                                }
                                button {
                                    r#type: "button",
                                    aria_label: "Entfernen",
                                    class: "flex size-9 items-center justify-center rounded-full text-muted-foreground hover:bg-accent",
                                    onclick: move |_| items.with_mut(|v| { if i < v.len() { v.remove(i); } }),
                                    X { class: "size-4" }
                                }
                            }
                            if d.exercise.is_strength() {
                                div { class: "grid grid-cols-3 gap-2",
                                    Field { label: "Sätze",
                                        NumberInput { value: d.sets.clone(), integer: true, oninput: move |v| items.with_mut(|x| if let Some(it) = x.get_mut(i) { it.sets = v; }) }
                                    }
                                    Field { label: "Wdh.",
                                        NumberInput { value: d.reps.clone(), integer: true, oninput: move |v| items.with_mut(|x| if let Some(it) = x.get_mut(i) { it.reps = v; }) }
                                    }
                                    Field { label: "kg",
                                        NumberInput { value: d.kg.clone(), placeholder: "letztes", oninput: move |v| items.with_mut(|x| if let Some(it) = x.get_mut(i) { it.kg = v; }) }
                                    }
                                }
                            } else {
                                Field { label: "Dauer",
                                    NumberInput { value: d.dur.clone(), unit: "min", placeholder: "30", oninput: move |v| items.with_mut(|x| if let Some(it) = x.get_mut(i) { it.dur = v; }) }
                                }
                            }
                        }
                    }
                    button {
                        r#type: "button",
                        class: "flex h-12 w-full items-center justify-center gap-2 rounded-2xl border border-dashed text-sm font-medium hover:bg-accent active:bg-accent",
                        onclick: move |_| picking.set(true),
                        Plus { class: "size-4" }
                        "Übung hinzufügen"
                    }
                    p { class: "px-1 text-[11px] text-muted-foreground", "Ohne Gewicht wird beim Eintragen das Gewicht vom letzten Mal übernommen." }
                }
                Field { label: "Notiz",
                    input { class: "{INPUT}", value: "{notes}", placeholder: "optional", oninput: move |e| notes.set(e.value()) }
                }
            }
        }
        if picking() {
            ExercisePicker {
                title: "Übung zur Vorlage",
                onclose: move |_| picking.set(false),
                onpick: move |ex: Exercise| {
                    picking.set(false);
                    items.with_mut(|v| {
                        v.push(ItemDraft { exercise: ex, sets: "3".into(), reps: "10".into(), kg: String::new(), dur: String::new() })
                    });
                },
            }
        }
    }
}

#[component]
fn SaveDaySheet(date: NaiveDate, onclose: EventHandler<()>, onsaved: EventHandler<()>) -> Element {
    let mut app = use_app();
    let mut name = use_signal(String::new);
    let mut busy = use_signal(|| false);
    let save = move |_| async move {
        if name().trim().is_empty() {
            app.error("Bitte einen Namen eingeben");
            return;
        }
        busy.set(true);
        let r = template_from_day(app.pid(), date, name()).await;
        busy.set(false);
        match r {
            Ok(t) => {
                app.toast(format!("Vorlage „{}“ gespeichert", t.name));
                onsaved.call(());
            }
            Err(e) => app.error(e),
        }
    };
    rsx! {
        Sheet {
            title: "Tag als Vorlage",
            onclose: move |_| onclose.call(()),
            footer: rsx! {
                Button { size: ButtonSize::Lg, class: "w-full", disabled: busy(), onclick: save, "Vorlage speichern" }
            },
            div { class: "flex flex-col gap-3 px-5 pb-5 pt-1",
                p { class: "text-sm text-muted-foreground", "Alle Übungen vom {fmt_short(date)} mit ihren Sätzen werden als Vorlage gespeichert." }
                Field { label: "Name",
                    input {
                        class: "{INPUT}",
                        value: "{name}",
                        placeholder: "z.B. Push",
                        onmounted: move |e| async move { let _ = e.set_focus(true).await; },
                        oninput: move |e| name.set(e.value()),
                    }
                }
            }
        }
    }
}

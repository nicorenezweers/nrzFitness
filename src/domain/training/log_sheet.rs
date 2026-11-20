//! Übung erfassen / bearbeiten: Satz-Tabelle (Kraft) bzw. Dauer/Distanz (Cardio, Sport, Dehnen).

use chrono::NaiveDate;
use dioxus::prelude::*;
use icons::{Check, History, Plus, RotateCcw, Trash2, X};

use super::page_training::use_training;
use super::shared::{BigNum, CategoryIcon};
use crate::api::training::{delete_exercise_entry, last_exercise_entry, save_exercise_entry};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, INPUT, NumberInput};
use crate::components::ui::sheet::Sheet;
use crate::model::{EntryInput, Exercise, ExerciseEntry, SetRow, estimate_kcal, pace, sets_summary, speed_kmh};
use crate::state::use_app;
use crate::util::{fmt_relative, num, parse_num};

#[derive(Clone, PartialEq, Default)]
struct SetDraft {
    reps: String,
    kg: String,
    done: bool,
}

impl SetDraft {
    fn from_row(s: &SetRow, done: bool) -> Self {
        Self {
            reps: if s.reps > 0 { s.reps.to_string() } else { String::new() },
            kg: if s.weight_kg > 0.0 { num(s.weight_kg, 2) } else { String::new() },
            done,
        }
    }
    fn to_row(&self) -> SetRow {
        SetRow {
            id: String::new(),
            reps: parse_num(&self.reps).unwrap_or(0.0).round().max(0.0) as i64,
            weight_kg: parse_num(&self.kg).unwrap_or(0.0).max(0.0),
            done: self.done,
        }
    }
    fn filled(&self) -> bool {
        let r = self.to_row();
        r.reps > 0 || r.weight_kg > 0.0
    }
}

fn opt_num(v: f64, dec: usize) -> String {
    if v > 0.0 { num(v, dec) } else { String::new() }
}

#[component]
pub fn LogSheet(
    exercise: Exercise,
    #[props(!optional)] entry: Option<ExerciseEntry>,
    date: NaiveDate,
    onclose: EventHandler<()>,
    onsaved: EventHandler<()>,
) -> Element {
    let mut app = use_app();
    let mut t = use_training();
    let lib = t.lib();
    let body = lib.weight;
    let strength = exercise.is_strength();
    let editing = entry.is_some();

    let init = entry.clone();
    let mut sets = use_signal(|| match &init {
        Some(e) => e.sets.iter().map(|s| SetDraft::from_row(s, s.done)).collect::<Vec<_>>(),
        None => vec![SetDraft::default()],
    });
    let mut duration = use_signal(|| init.as_ref().map(|e| opt_num(e.duration_min, 1)).unwrap_or_default());
    let mut distance =
        use_signal(|| init.as_ref().and_then(|e| e.distance_km).map(|d| opt_num(d, 2)).unwrap_or_default());
    let mut notes = use_signal(|| init.as_ref().and_then(|e| e.notes.clone()).unwrap_or_default());
    // Gespeicherte kcal behalten, wenn sie von der Schätzung abweichen (von Hand geändert).
    let met = exercise.met;
    let mut kcal_manual = use_signal(|| {
        init.as_ref().is_some_and(|e| {
            let n = e.sets.len();
            (e.kcal - estimate_kcal(met, body, e.duration_min, if e.is_strength() { n } else { 0 })).abs() > 1.0
        })
    });
    let mut kcal = use_signal(|| init.as_ref().map(|e| num(e.kcal, 0)).unwrap_or_default());
    let mut touched = use_signal(|| editing);
    let mut busy = use_signal(|| false);
    let mut confirm = use_signal(|| false);

    let ex_id = exercise.id.clone();
    let exclude = entry.as_ref().map(|e| e.id.clone());
    let last = use_resource(move || {
        let ex_id = ex_id.clone();
        let exclude = exclude.clone();
        async move {
            if ex_id.is_empty() {
                return Ok(None);
            }
            last_exercise_entry(app.pid(), ex_id, exclude).await
        }
    });
    let last_entry: Option<ExerciseEntry> = last.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().flatten();

    // Vorbelegung mit dem letzten Mal (nur bei neuen Einträgen, solange nichts getippt wurde).
    let mut prefilled = use_signal(|| editing);
    let prefill_src = last_entry.clone();
    use_effect(use_reactive!(|prefill_src| {
        if *prefilled.peek() || *touched.peek() {
            return;
        }
        let Some(l) = prefill_src else { return };
        prefilled.set(true);
        if strength {
            if !l.sets.is_empty() {
                sets.set(l.sets.iter().map(|s| SetDraft::from_row(s, false)).collect());
            }
        } else {
            duration.set(opt_num(l.duration_min, 1));
            distance.set(l.distance_km.map(|d| opt_num(d, 2)).unwrap_or_default());
        }
    }));

    // --- Berechnungen ---
    let drafts = sets.read().clone();
    let filled = drafts.iter().filter(|d| d.filled()).count();
    let dur = parse_num(&duration()).unwrap_or(0.0).max(0.0);
    let km = parse_num(&distance()).unwrap_or(0.0).max(0.0);
    let auto = estimate_kcal(met, body, dur, if strength { filled } else { 0 });
    let kcal_text = if kcal_manual() { kcal() } else { num(auto, 0) };
    let est_min = if dur > 0.0 {
        dur
    } else if strength {
        filled as f64 * crate::model::MIN_PER_SET
    } else {
        0.0
    };
    let kcal_hint = format!(
        "geschätzt: {} MET × {} kg{} × {} min",
        num(met, 1),
        num(body, 1),
        if lib.weight_known { "" } else { " (Annahme)" },
        num(est_min, 1)
    );
    let volume: f64 = drafts
        .iter()
        .filter(|d| d.done || !drafts.iter().any(|x| x.done))
        .map(|d| d.to_row())
        .map(|r| r.reps as f64 * r.weight_kg)
        .sum();
    let all_done = !drafts.is_empty() && drafts.iter().all(|d| d.done);

    let last_text = last_entry.as_ref().map(|l| {
        let what = if l.is_strength() && !l.sets.is_empty() {
            sets_summary(&l.sets)
        } else {
            let mut p = Vec::new();
            if l.duration_min > 0.0 {
                p.push(format!("{} min", num(l.duration_min, 0)));
            }
            if let Some(d) = l.distance_km {
                p.push(format!("{} km", num(d, 2)));
            }
            p.join(" · ")
        };
        (what, fmt_relative(l.date))
    });

    // --- Aktionen ---
    let entry_id = entry.as_ref().map(|e| e.id.clone());
    let entry_id_del = entry_id.clone();
    let ex = exercise.clone();
    let save = move |_| {
        let id = entry_id.clone();
        let ex = ex.clone();
        async move {
            let mut rows: Vec<SetRow> = sets.read().iter().filter(|d| d.filled()).map(|d| d.to_row()).collect();
            let dur = parse_num(&duration()).unwrap_or(0.0).max(0.0);
            if strength && rows.is_empty() {
                app.error("Mindestens einen Satz mit Wiederholungen oder Gewicht erfassen");
                return;
            }
            if !strength && dur <= 0.0 {
                app.error("Bitte eine Dauer angeben");
                return;
            }
            // Neu erfasst und nichts abgehakt → alles gilt als erledigt.
            if id.is_none() && !rows.iter().any(|s| s.done) {
                for s in &mut rows {
                    s.done = true;
                }
            }
            let n = rows.len();
            let kcal_val = if kcal_manual() {
                parse_num(&kcal()).unwrap_or(0.0)
            } else {
                estimate_kcal(ex.met, body, dur, if strength { n } else { 0 })
            };
            let input = EntryInput {
                date,
                exercise_id: Some(ex.id.clone()).filter(|i| !i.is_empty()),
                name: ex.name.clone(),
                category: ex.category.clone(),
                duration_min: dur,
                kcal: kcal_val.round(),
                distance_km: if strength { None } else { parse_num(&distance()).filter(|d| *d > 0.0) },
                notes: Some(notes()).filter(|n| !n.trim().is_empty()),
                sets: rows,
            };
            busy.set(true);
            let res = save_exercise_entry(app.pid(), id.clone(), input).await;
            busy.set(false);
            match res {
                Ok(_) => {
                    app.toast(if id.is_some() {
                        "Gespeichert".to_string()
                    } else {
                        format!("{} eingetragen", ex.name)
                    });
                    onsaved.call(());
                }
                Err(e) => app.error(e),
            }
        }
    };
    let delete = move |_| {
        let id = entry_id_del.clone();
        async move {
            let Some(id) = id else { return };
            if !confirm() {
                confirm.set(true);
                return;
            }
            match delete_exercise_entry(app.pid(), id).await {
                Ok(()) => {
                    app.toast("Eintrag gelöscht");
                    onsaved.call(());
                }
                Err(e) => app.error(e),
            }
        }
    };
    let apply_last = {
        let l = last_entry.clone();
        move |_| {
            let Some(l) = l.clone() else { return };
            touched.set(true);
            if strength {
                sets.set(l.sets.iter().map(|s| SetDraft::from_row(s, false)).collect());
            } else {
                duration.set(opt_num(l.duration_min, 1));
                distance.set(l.distance_km.map(|d| opt_num(d, 2)).unwrap_or_default());
            }
        }
    };

    let header_badge = rsx! { CategoryIcon { category: exercise.category.clone(), size: "size-9" } };
    let title = exercise.name.clone();

    rsx! {
        Sheet {
            title,
            tall: strength,
            header_action: header_badge,
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
                    Button { size: ButtonSize::Lg, class: "flex-1", disabled: busy(), onclick: save,
                        Check { class: "size-5" }
                        if editing { "Speichern" } else { "Eintragen" }
                        span { class: "font-normal opacity-70", "· {kcal_text} kcal" }
                    }
                }
            },
            div { class: "flex flex-col gap-4 px-4 pb-5 pt-1 sm:px-5",
                p { class: "-mt-1 text-xs text-muted-foreground", "{exercise.subtitle()}" }

                // Letztes Mal
                if let Some((what, when)) = last_text.clone() {
                    div { class: "flex items-center gap-3 rounded-2xl bg-muted/60 px-3 py-2.5",
                        History { class: "size-4 shrink-0 text-muted-foreground" }
                        div { class: "min-w-0 flex-1 text-sm",
                            span { class: "text-muted-foreground", "Letztes Mal: " }
                            span { class: "font-medium", "{what}" }
                            span { class: "text-muted-foreground", " · {when}" }
                        }
                        button {
                            r#type: "button",
                            class: "flex h-9 shrink-0 items-center gap-1 rounded-full bg-background px-3 text-xs font-medium shadow-sm active:scale-95",
                            onclick: apply_last,
                            RotateCcw { class: "size-3.5" }
                            "Übernehmen"
                        }
                    }
                }

                if strength {
                    // Satz-Tabelle
                    div { class: "overflow-hidden rounded-2xl border",
                        div { class: "grid grid-cols-[2.25rem_minmax(0,1fr)_minmax(0,1fr)_2.75rem_2.25rem] items-center gap-2 bg-muted/50 px-2 py-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground",
                            span { class: "text-center", "Satz" }
                            span { class: "text-center", "Wdh." }
                            span { class: "text-center", "kg" }
                            button {
                                r#type: "button",
                                class: "flex h-7 items-center justify-center rounded-lg text-[10px] font-semibold uppercase hover:text-foreground",
                                title: "Alle abhaken",
                                onclick: move |_| {
                                    touched.set(true);
                                    let v = !all_done;
                                    sets.with_mut(|s| s.iter_mut().for_each(|d| d.done = v));
                                },
                                if all_done { "keine" } else { "alle" }
                            }
                            span {}
                        }
                        for (i, d) in drafts.iter().cloned().enumerate() {
                            div {
                                key: "{i}",
                                class: "grid grid-cols-[2.25rem_minmax(0,1fr)_minmax(0,1fr)_2.75rem_2.25rem] items-center gap-2 border-t px-2 py-1.5 transition-colors",
                                class: if d.done { "bg-good/10" },
                                span { class: "flex size-8 items-center justify-center justify-self-center rounded-full bg-muted text-sm font-semibold tabular-nums", "{i + 1}" }
                                BigNum {
                                    value: d.reps.clone(),
                                    integer: true,
                                    placeholder: "0",
                                    oninput: move |v| {
                                        touched.set(true);
                                        sets.with_mut(|s| if let Some(x) = s.get_mut(i) { x.reps = v; });
                                    },
                                }
                                BigNum {
                                    value: d.kg.clone(),
                                    placeholder: "0",
                                    oninput: move |v| {
                                        touched.set(true);
                                        sets.with_mut(|s| if let Some(x) = s.get_mut(i) { x.kg = v; });
                                    },
                                }
                                button {
                                    r#type: "button",
                                    aria_label: "Satz abhaken",
                                    class: "flex size-11 items-center justify-center rounded-xl transition-all active:scale-90",
                                    class: if d.done { "bg-good text-background shadow-sm" } else { "border bg-background text-muted-foreground" },
                                    onclick: move |_| {
                                        touched.set(true);
                                        let now_done = !d.done;
                                        sets.with_mut(|s| if let Some(x) = s.get_mut(i) { x.done = now_done; });
                                        if now_done {
                                            t.start_rest();
                                        }
                                    },
                                    Check { class: "size-5" }
                                }
                                button {
                                    r#type: "button",
                                    aria_label: "Satz löschen",
                                    class: "flex size-9 items-center justify-center rounded-full text-muted-foreground hover:bg-accent active:scale-90",
                                    onclick: move |_| {
                                        touched.set(true);
                                        sets.with_mut(|s| { if i < s.len() { s.remove(i); } });
                                    },
                                    X { class: "size-4" }
                                }
                            }
                        }
                        button {
                            r#type: "button",
                            class: "flex h-12 w-full items-center justify-center gap-2 border-t text-sm font-medium transition-colors hover:bg-accent active:bg-accent",
                            onclick: move |_| {
                                touched.set(true);
                                sets.with_mut(|s| {
                                    let next = s.last().map(|l| SetDraft { reps: l.reps.clone(), kg: l.kg.clone(), done: false }).unwrap_or_default();
                                    s.push(next);
                                });
                            },
                            Plus { class: "size-4" }
                            "Satz"
                        }
                    }
                    if volume > 0.0 {
                        p { class: "-mt-2 px-1 text-xs text-muted-foreground",
                            "{filled} Sätze · Volumen {super::shared::fmt_volume(volume)} · Abhaken startet den Pausen-Timer"
                        }
                    }
                    div { class: "grid grid-cols-2 gap-3",
                        Field { label: "Dauer (optional)",
                            NumberInput { value: duration(), unit: "min", placeholder: "–", oninput: move |v| { touched.set(true); duration.set(v); } }
                        }
                        KcalField { value: kcal_text.clone(), manual: kcal_manual(), oninput: move |v| { kcal_manual.set(true); kcal.set(v); }, onreset: move |_| kcal_manual.set(false) }
                    }
                } else {
                    div { class: "flex flex-col gap-2",
                        div { class: "grid grid-cols-2 gap-3",
                            Field { label: "Dauer",
                                NumberInput { value: duration(), unit: "min", placeholder: "0", autofocus: !editing, oninput: move |v| { touched.set(true); duration.set(v); } }
                            }
                            Field { label: "Distanz (optional)",
                                NumberInput { value: distance(), unit: "km", placeholder: "–", oninput: move |v| { touched.set(true); distance.set(v); } }
                            }
                        }
                        div { class: "no-scrollbar -mx-4 flex gap-2 overflow-x-auto px-4 sm:mx-0 sm:px-0",
                            for m in [10, 15, 20, 30, 45, 60, 90] {
                                button {
                                    key: "{m}",
                                    r#type: "button",
                                    class: "h-9 shrink-0 rounded-full border px-3.5 text-sm tabular-nums transition-transform active:scale-95",
                                    class: if dur == m as f64 { "border-primary bg-primary text-primary-foreground" } else { "text-muted-foreground hover:text-foreground" },
                                    onclick: move |_| { touched.set(true); duration.set(m.to_string()); },
                                    "{m} min"
                                }
                            }
                        }
                    }
                    if let Some(p) = pace(dur, km) {
                        div { class: "grid grid-cols-2 gap-3 rounded-2xl bg-muted/60 p-3",
                            div { class: "flex flex-col",
                                span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Pace" }
                                span { class: "text-lg font-semibold tabular-nums", "{p}" }
                            }
                            div { class: "flex flex-col",
                                span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Tempo" }
                                span { class: "text-lg font-semibold tabular-nums", "{num(speed_kmh(dur, km).unwrap_or(0.0), 1)} km/h" }
                            }
                        }
                    }
                    KcalField { value: kcal_text.clone(), manual: kcal_manual(), oninput: move |v| { kcal_manual.set(true); kcal.set(v); }, onreset: move |_| kcal_manual.set(false) }
                }
                p { class: "-mt-2 px-1 text-[11px] text-muted-foreground",
                    if kcal_manual() { "kcal von Hand gesetzt" } else { "{kcal_hint}" }
                }
                Field { label: "Notiz",
                    input {
                        class: "{INPUT}",
                        value: "{notes}",
                        placeholder: "z.B. Griffbreite, Gefühl, Strecke …",
                        oninput: move |e| notes.set(e.value()),
                    }
                }
            }
        }
    }
}

#[component]
fn KcalField(value: String, manual: bool, oninput: EventHandler<String>, onreset: EventHandler<()>) -> Element {
    rsx! {
        div { class: "flex min-w-0 flex-col gap-1.5",
            div { class: "flex h-4 items-center justify-between",
                span { class: "text-xs font-medium text-muted-foreground", "Kalorien" }
                if manual {
                    button { r#type: "button", class: "text-xs font-medium text-primary underline-offset-2 hover:underline", onclick: move |_| onreset.call(()), "Auto" }
                }
            }
            NumberInput { value, unit: "kcal", integer: true, oninput: move |v| oninput.call(v) }
        }
    }
}

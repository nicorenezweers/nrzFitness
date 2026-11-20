//! Verlauf: Wochenübersicht, Serie, persönliche Rekorde, Fortschritt pro Übung.

use dioxus::prelude::*;
use icons::{CalendarCheck, Flame, Medal, TrendingUp, Trophy};

use super::shared::CategoryIcon;
use crate::api::training::{get_exercise_progress, get_personal_records, get_training_history};
use crate::components::charts::{BarChart, LineChart};
use crate::components::layout::page::Section;
use crate::components::ui::card::{Card, CardHeader, Stat};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{PersonalRecord, TrainingHistory};
use crate::state::use_app;
use crate::util::{fmt_dm, fmt_relative, int, num, today};

fn opts(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter().map(|(k, l)| (k.to_string(), l.to_string())).collect()
}

#[component]
pub fn HistoryView() -> Element {
    let app = use_app();
    let hist = use_resource(move || async move {
        let _ = (app.refresh)();
        let pid = app.pid_tracked();
        get_training_history(pid, today(), 12).await
    });
    let recs = use_resource(move || async move {
        let _ = (app.refresh)();
        let pid = app.pid_tracked();
        get_personal_records(pid).await
    });
    let mut selected: Signal<Option<String>> = use_signal(|| None);

    let records: Vec<PersonalRecord> = recs.read().as_ref().and_then(|r| r.as_ref().ok()).cloned().unwrap_or_default();
    let sel_id = selected().or_else(|| {
        records.iter().find(|r| r.category == "strength").or(records.first()).map(|r| r.exercise_id.clone())
    });
    let sel = sel_id.as_ref().and_then(|id| records.iter().find(|r| &r.exercise_id == id)).cloned();

    rsx! {
        div { class: "flex flex-col gap-4 lg:grid lg:grid-cols-2 lg:items-start",
            match hist.read().clone() {
                None => rsx! { Skeleton { class: "h-28 rounded-2xl" } Skeleton { class: "h-56 rounded-2xl" } },
                Some(Err(e)) => rsx! { Empty { EmptyTitle { "Fehler beim Laden" } EmptyDescription { "{e}" } } },
                Some(Ok(h)) => rsx! {
                    StreakCard { h: h.clone() }
                    WeeksCard { h }
                },
            }

            if let Some(r) = sel.clone() {
                Card { class: "lg:col-span-2",
                    CardHeader { title: "Fortschritt", subtitle: r.name.clone(), icon: rsx! { TrendingUp { class: "size-4" } },
                        action: rsx! {
                            select {
                                class: "h-10 max-w-[45%] truncate rounded-xl border border-input bg-background px-2 text-sm",
                                aria_label: "Übung wählen",
                                onchange: move |e| selected.set(Some(e.value())),
                                for rec in records.iter() {
                                    option { key: "{rec.exercise_id}", value: "{rec.exercise_id}", selected: Some(&rec.exercise_id) == sel_id.as_ref(), "{rec.name}" }
                                }
                            }
                        },
                    }
                    div { class: "px-4 pb-4",
                        ProgressChart { key: "{r.exercise_id}", exercise_id: r.exercise_id.clone(), strength: r.category == "strength" }
                    }
                }
            }

            Section { title: "Persönliche Rekorde", count: records.len(), class: "lg:col-span-2",
                if recs.read().is_none() {
                    Skeleton { class: "h-20 rounded-2xl" }
                } else if records.is_empty() {
                    Empty {
                        div { class: "flex size-12 items-center justify-center rounded-2xl bg-muted text-carbs", Trophy { class: "size-6" } }
                        EmptyTitle { "Noch keine Rekorde" }
                        EmptyDescription { "Sobald du Übungen aus der Bibliothek einträgst, erscheinen hier deine Bestleistungen." }
                    }
                } else {
                    div { class: "grid gap-2 sm:grid-cols-2",
                        for r in records.iter().cloned() {
                            RecordCard {
                                key: "{r.exercise_id}",
                                active: Some(&r.exercise_id) == sel_id.as_ref(),
                                r: r.clone(),
                                onclick: move |_| selected.set(Some(r.exercise_id.clone())),
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn StreakCard(h: TrainingHistory) -> Element {
    let last = h.last_date.map(fmt_relative).unwrap_or_else(|| "–".into());
    rsx! {
        Card { class: "p-4",
            div { class: "flex items-center gap-4",
                div { class: "flex size-16 shrink-0 flex-col items-center justify-center rounded-2xl bg-muted",
                    Flame { class: if h.streak > 0 { "size-6 text-kcal" } else { "size-6 text-muted-foreground" } }
                    span { class: "text-lg font-bold leading-none tabular-nums", "{h.streak}" }
                }
                div { class: "min-w-0 flex-1",
                    p { class: "text-[15px] font-semibold",
                        if h.streak == 1 { "1 Trainingstag in Serie" } else { "{h.streak} Trainingstage in Serie" }
                    }
                    p { class: "text-xs text-muted-foreground",
                        if h.streak == 0 { "Heute trainieren startet eine neue Serie." } else { "Weiter so – morgen hält die Serie am Leben." }
                    }
                }
            }
            div { class: "mt-4 grid grid-cols-3 gap-2",
                Stat { label: "Beste Serie", value: h.best_streak.to_string(), unit: "Tage" }
                Stat { label: "30 Tage", value: h.days_30.to_string(), unit: "Tage" }
                Stat { label: "Zuletzt", value: last }
            }
        }
    }
}

#[component]
fn WeeksCard(h: TrainingHistory) -> Element {
    let mut metric = use_signal(|| "minutes".to_string());
    let m = metric();
    let (color, unit) = match m.as_str() {
        "kcal" => ("text-kcal", "kcal"),
        "days" => ("text-carbs", "Tage"),
        _ => ("text-protein", "min"),
    };
    let bars: Vec<(String, f64)> = h
        .weeks
        .iter()
        .map(|w| {
            let v = match m.as_str() {
                "kcal" => w.kcal,
                "days" => w.days as f64,
                _ => w.minutes,
            };
            (fmt_dm(w.start), v)
        })
        .collect();
    let active: Vec<f64> = bars.iter().map(|b| b.1).filter(|v| *v > 0.0).collect();
    let avg = if active.is_empty() { 0.0 } else { bars.iter().map(|b| b.1).sum::<f64>() / bars.len() as f64 };
    let this_week = bars.last().map(|b| b.1).unwrap_or(0.0);
    rsx! {
        Card {
            CardHeader { title: "Wochen", subtitle: "Letzte 12 Wochen", icon: rsx! { CalendarCheck { class: "size-4" } } }
            div { class: "flex flex-col gap-3 px-4 pb-4",
                Segmented { options: opts(&[("minutes", "Minuten"), ("kcal", "kcal"), ("days", "Tage")]), value: m.clone(), onchange: move |v| metric.set(v) }
                div { class: "{color}", BarChart { bars, color: color.to_string(), decimals: 0 } }
                div { class: "grid grid-cols-2 gap-2",
                    Stat { label: "Diese Woche", value: int(this_week), unit }
                    Stat { label: "Ø pro Woche", value: num(avg, if m == "days" { 1 } else { 0 }), unit }
                }
            }
        }
    }
}

#[component]
fn RecordCard(r: PersonalRecord, active: bool, onclick: EventHandler<()>) -> Element {
    let strength = r.category == "strength" && r.heaviest_kg + r.most_reps as f64 > 0.0;
    let last = r.last_date.map(fmt_relative).unwrap_or_default();
    rsx! {
        Card { class: if active { "border-primary/60 p-3" } else { "p-3" }, onclick: move |_| onclick.call(()),
            div { class: "flex items-center gap-3",
                CategoryIcon { category: r.category.clone() }
                div { class: "min-w-0 flex-1",
                    p { class: "truncate text-[15px] font-semibold", "{r.name}" }
                    p { class: "truncate text-xs text-muted-foreground", "{r.sessions}× trainiert · zuletzt {last}" }
                }
                Medal { class: "size-4 shrink-0 text-carbs" }
            }
            if strength {
                div { class: "mt-3 grid grid-cols-3 gap-2",
                    Stat { label: "Schwerster", value: if r.heaviest_kg > 0.0 { format!("{}×{}", num(r.heaviest_kg, 2), r.heaviest_reps) } else { "–".into() }, unit: if r.heaviest_kg > 0.0 { "kg" } else { "" } }
                    Stat { label: "1RM (Epley)", value: if r.best_e1rm > 0.0 { num(r.best_e1rm, 1) } else { "–".into() }, unit: if r.best_e1rm > 0.0 { "kg" } else { "" } }
                    Stat { label: "Meiste Wdh.", value: r.most_reps.to_string() }
                }
            } else {
                div { class: "mt-3 grid grid-cols-3 gap-2",
                    Stat { label: "Längste", value: num(r.longest_min, 0), unit: "min" }
                    Stat { label: "Weiteste", value: if r.longest_km > 0.0 { num(r.longest_km, 2) } else { "–".into() }, unit: if r.longest_km > 0.0 { "km" } else { "" } }
                    Stat { label: "Einheiten", value: r.sessions.to_string() }
                }
            }
        }
    }
}

/// Fortschritts-Diagramm einer Übung (e1RM / Volumen / Top-Satz bzw. Dauer / Distanz).
#[component]
pub fn ProgressChart(exercise_id: String, strength: bool) -> Element {
    let app = use_app();
    let id = exercise_id.clone();
    let data = use_resource(move || {
        let id = id.clone();
        async move {
            let _ = (app.refresh)();
            let pid = app.pid_tracked();
            get_exercise_progress(pid, id).await
        }
    });
    let mut metric = use_signal(move || if strength { "e1rm".to_string() } else { "duration".to_string() });
    let m = metric();
    let options = if strength {
        opts(&[("e1rm", "1RM"), ("volume", "Volumen"), ("top", "Top-Satz")])
    } else {
        opts(&[("duration", "Dauer"), ("distance", "Distanz")])
    };
    let read = data.read();
    let Some(Ok(points)) = read.as_ref() else {
        return rsx! { Skeleton { class: "h-40 rounded-xl" } };
    };
    let pts: Vec<(String, Option<f64>)> = points
        .iter()
        .rev()
        .take(30)
        .rev()
        .map(|p| {
            let v = match m.as_str() {
                "volume" => p.volume,
                "top" => p.top_kg,
                "duration" => p.duration_min,
                "distance" => p.distance_km,
                _ => p.e1rm,
            };
            (fmt_dm(p.date), Some(v).filter(|v| *v > 0.0))
        })
        .collect();
    let (unit, color) = match m.as_str() {
        "volume" => ("kg", "text-carbs"),
        "duration" => ("min", "text-protein"),
        "distance" => ("km", "text-water"),
        _ => ("kg", "text-kcal"),
    };
    let vals: Vec<f64> = pts.iter().filter_map(|p| p.1).collect();
    let best = vals.iter().cloned().fold(0.0, f64::max);
    let first = vals.first().copied().unwrap_or(0.0);
    let lastv = vals.last().copied().unwrap_or(0.0);
    let delta = lastv - first;
    rsx! {
        div { class: "flex flex-col gap-3",
            Segmented { options, value: m.clone(), onchange: move |v| metric.set(v) }
            LineChart { points: pts, color: color.to_string(), decimals: if m == "distance" { 1 } else { 0 } }
            div { class: "grid grid-cols-3 gap-2",
                Stat { label: "Bestwert", value: num(best, 1), unit }
                Stat { label: "Aktuell", value: num(lastv, 1), unit }
                Stat {
                    label: "Veränderung",
                    value: if vals.len() > 1 { format!("{}{}", if delta > 0.0 { "+" } else { "" }, num(delta, 1)) } else { "–".into() },
                    unit: if vals.len() > 1 { unit } else { "" },
                    class: if delta > 0.0 { "text-good" } else { "" },
                }
            }
            p { class: "text-[11px] text-muted-foreground", "{points.len()} Trainingstage" }
        }
    }
}

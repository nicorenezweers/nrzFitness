use dioxus::prelude::*;
use icons::{Check, Pill, Plus};

use super::med_sheet::MedSheet;
use super::symptoms::SymptomsTab;
use crate::api::meds::{meds_data, set_medication_active, toggle_med_log};
use crate::components::layout::page::{PageBody, PageHeader, Section};
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::date_nav::DateNav;
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::field::Switch;
use crate::components::ui::ring::{Bar, Ring};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{MedLog, Medication, MedsData, adherence, slot_label};
use crate::state::use_app;
use crate::util::{fmt_time, now, num, today};

#[component]
pub fn MedsPage() -> Element {
    let mut app = use_app();
    let mut tab = use_signal(|| "meds".to_string());
    let mut sheet = use_signal(|| None::<Option<Medication>>);
    let date = (app.date)();
    rsx! {
        PageHeader {
            title: if tab() == "meds" { "Medikamente" } else { "Symptome" },
            subtitle: "Einnahme & Befinden",
            actions: rsx! {
                if tab() == "meds" {
                    Button { size: ButtonSize::Sm, class: "h-10 rounded-full px-4", onclick: move |_| sheet.set(Some(None)),
                        Plus { class: "size-4" }
                        "Neu"
                    }
                }
            },
        }
        PageBody {
            div { class: "flex flex-col gap-3 sm:flex-row sm:items-center sm:justify-between",
                Segmented {
                    class: "sm:max-w-xs",
                    options: vec![("meds".into(), "Medikamente".into()), ("symptoms".into(), "Symptome".into())],
                    value: tab(),
                    onchange: move |v| tab.set(v),
                }
                div { class: "flex justify-center", DateNav { date, onchange: move |d| app.date.set(d) } }
            }
            if tab() == "meds" {
                MedsTab { onedit: move |m| sheet.set(Some(m)) }
            } else {
                SymptomsTab {}
            }
        }
        if let Some(m) = sheet() {
            MedSheet { key: "{m.as_ref().map(|m| m.id.clone()).unwrap_or_default()}", med: m, onclose: move |_| sheet.set(None) }
        }
    }
}

#[component]
fn MedsTab(onedit: EventHandler<Option<Medication>>) -> Element {
    let app = use_app();
    let res = use_resource(move || async move {
        let _ = (app.refresh)();
        let d = (app.date)();
        meds_data(app.pid_tracked(), d, today()).await.map_err(|e| e.to_string())
    });
    let date = (app.date)();
    let data: MedsData = match &*res.read() {
        Some(Ok(d)) => d.clone(),
        Some(Err(e)) => return rsx! { p { class: "text-sm text-bad", "{e}" } },
        None => return rsx! { Skeleton { class: "h-64 rounded-2xl" } },
    };
    if data.meds.is_empty() {
        return rsx! {
            Empty { class: "py-12",
                div { class: "flex size-12 items-center justify-center rounded-full bg-protein/15 text-protein", Pill { class: "size-6" } }
                EmptyTitle { "Noch keine Medikamente" }
                EmptyDescription { "Lege Medikamente oder Nahrungsergänzungen mit Einnahmezeiten an und hake sie täglich ab." }
                Button { size: ButtonSize::Lg, onclick: move |_| onedit.call(None), Plus { class: "size-4" } "Medikament hinzufügen" }
            }
        };
    }

    // Checkliste: (Zeit, Medikament) für aktive Medikamente, die an diesem Tag schon existierten.
    let mut slots: Vec<(String, Medication)> = data
        .meds
        .iter()
        .filter(|m| m.active && m.created_at.date() <= date)
        .flat_map(|m| m.slots().into_iter().map(move |s| (s, m.clone())))
        .collect();
    slots.sort_by(|a, b| a.0.cmp(&b.0).then(a.1.name.to_lowercase().cmp(&b.1.name.to_lowercase())));
    let taken = |m: &Medication, s: &str| {
        data.logs.iter().find(|l| l.medication_id == m.id && l.date == date && l.slot == s).cloned()
    };
    let done = slots.iter().filter(|(s, m)| taken(m, s).is_some()).count();
    let total = slots.len();
    let mut groups: Vec<(String, Vec<(Medication, Option<MedLog>)>)> = Vec::new();
    for (s, m) in &slots {
        let log = taken(m, s);
        match groups.last_mut() {
            Some((gs, items)) if gs == s => items.push((m.clone(), log)),
            _ => groups.push((s.clone(), vec![(m.clone(), log)])),
        }
    }
    let t = now();
    let a7 = adherence(&data.meds, &data.logs, today(), t, 7);
    let a30 = adherence(&data.meds, &data.logs, today(), t, 30);
    let progress = if total > 0 { done as f64 / total as f64 } else { 0.0 };

    rsx! {
        div { class: "grid gap-4 lg:grid-cols-2 lg:items-start",
            div { class: "flex min-w-0 flex-col gap-4",
                Card { class: "flex items-center gap-4 p-4",
                    Ring { value: progress, size: 84.0, stroke: 9.0, color: "text-good", over_color: "text-good",
                        span { class: "text-lg font-bold tabular-nums leading-none", "{done}/{total}" }
                    }
                    div { class: "flex min-w-0 flex-1 flex-col gap-2",
                        p { class: "text-sm font-semibold",
                            if total == 0 { "Heute nichts geplant" } else if done == total { "Alles genommen 🎉" } else { "{total - done} offen" }
                        }
                        div { class: "grid grid-cols-2 gap-3",
                            AdherenceStat { label: "7 Tage", value: a7 }
                            AdherenceStat { label: "30 Tage", value: a30 }
                        }
                    }
                }
                if groups.is_empty() {
                    Empty {
                        EmptyTitle { "Keine Einnahmen an diesem Tag" }
                        EmptyDescription { "Alle Medikamente sind pausiert oder wurden später angelegt." }
                    }
                }
                for (slot, items) in groups {
                    Section { key: "{slot}", title: "{slot} · {slot_label(&slot)}",
                        Card { class: "divide-y overflow-hidden",
                            for (m, log) in items {
                                CheckRow { key: "{m.id}-{slot}", med: m, slot: slot.clone(), log }
                            }
                        }
                    }
                }
            }
            div { class: "flex min-w-0 flex-col gap-4",
                Card {
                    CardHeader { title: "Meine Medikamente", subtitle: "{data.meds.iter().filter(|m| m.active).count()} aktiv" }
                    div { class: "divide-y border-t",
                        for m in data.meds.iter().cloned() {
                            MedRow { key: "{m.id}", med: m, onedit }
                        }
                    }
                    div { class: "border-t p-3",
                        Button { variant: crate::components::ui::button::ButtonVariant::Ghost, class: "h-11 w-full", onclick: move |_| onedit.call(None),
                            Plus { class: "size-4" }
                            "Medikament hinzufügen"
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn AdherenceStat(label: String, value: Option<f64>) -> Element {
    let color = match value {
        Some(v) if v >= 90.0 => "bg-good",
        Some(v) if v >= 60.0 => "bg-warn",
        Some(_) => "bg-bad",
        None => "bg-muted",
    };
    let txt = value.map(|v| format!("{} %", num(v, 0))).unwrap_or_else(|| "–".into());
    rsx! {
        div { class: "flex min-w-0 flex-col gap-1",
            div { class: "flex items-baseline justify-between gap-1",
                span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "{label}" }
                span { class: "text-sm font-semibold tabular-nums", "{txt}" }
            }
            Bar { value: value.unwrap_or(0.0) / 100.0, color: color }
        }
    }
}

#[component]
fn CheckRow(med: Medication, slot: String, log: Option<MedLog>) -> Element {
    let mut app = use_app();
    let mut busy = use_signal(|| false);
    let done = log.is_some();
    let id = med.id.clone();
    let s = slot.clone();
    let toggle = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        let pid = app.pid();
        let date = (app.date)();
        let (id, s) = (id.clone(), s.clone());
        spawn(async move {
            match toggle_med_log(pid, id, date, s, now()).await {
                Ok(_) => app.bump(),
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };
    rsx! {
        button {
            r#type: "button",
            class: if done { "flex w-full items-center gap-3 bg-good/10 px-4 py-3 text-left transition-colors active:bg-good/20" } else { "flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-accent/40 active:bg-accent/60" },
            disabled: busy(),
            onclick: toggle,
            span {
                class: if done { "flex size-8 shrink-0 items-center justify-center rounded-full bg-good text-background transition-all" } else { "flex size-8 shrink-0 items-center justify-center rounded-full border-2 border-muted-foreground/40 transition-all" },
                if done { Check { class: "size-4" } }
            }
            div { class: "min-w-0 flex-1",
                p { class: if done { "truncate font-medium text-muted-foreground line-through decoration-1" } else { "truncate font-medium" }, "{med.name}" }
                if let Some(d) = med.dose.as_ref() {
                    p { class: "truncate text-xs text-muted-foreground", "{d}" }
                }
            }
            if let Some(l) = log {
                span { class: "shrink-0 text-xs text-good tabular-nums", "um {fmt_time(l.taken_at)}" }
            }
        }
    }
}

#[component]
fn MedRow(med: Medication, onedit: EventHandler<Option<Medication>>) -> Element {
    let mut app = use_app();
    let m2 = med.clone();
    let id = med.id.clone();
    let active = med.active;
    let times = med.slots().join(", ");
    rsx! {
        div { class: "flex items-center gap-3 px-4 py-3",
            button {
                r#type: "button",
                class: "flex min-w-0 flex-1 items-center gap-3 text-left",
                onclick: move |_| onedit.call(Some(m2.clone())),
                div { class: if active { "flex size-10 shrink-0 items-center justify-center rounded-full bg-protein/15 text-protein" } else { "flex size-10 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground" },
                    Pill { class: "size-5" }
                }
                div { class: if active { "min-w-0 flex-1" } else { "min-w-0 flex-1 opacity-60" },
                    p { class: "truncate font-medium",
                        "{med.name}"
                        if !active {
                            span { class: "ml-2 rounded bg-muted px-1.5 py-px text-[10px] font-medium text-muted-foreground", "pausiert" }
                        }
                    }
                    p { class: "truncate text-xs text-muted-foreground",
                        if let Some(d) = med.dose.as_ref() { "{d} · " }
                        "{times}"
                    }
                }
            }
            Switch {
                checked: active,
                onchange: move |v: bool| {
                    let pid = app.pid();
                    let id = id.clone();
                    spawn(async move {
                        match set_medication_active(pid, id, v).await {
                            Ok(()) => {
                                app.toast(if v { "Aktiviert" } else { "Pausiert" });
                                app.bump();
                            }
                            Err(e) => app.error(e),
                        }
                    });
                },
            }
        }
    }
}

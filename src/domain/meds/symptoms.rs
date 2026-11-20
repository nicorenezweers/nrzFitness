//! Symptome erfassen (Schnell-Chips, Stärke 1–5, Notiz) und Verlauf.

use std::collections::HashMap;

use chrono::Duration;
use dioxus::prelude::*;
use icons::{Stethoscope, Trash2};

use crate::api::meds::{add_symptom, delete_symptom, symptoms_data};
use crate::components::layout::page::Section;
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::field::{Field, INPUT, Scale5};
use crate::components::ui::skeleton::Skeleton;
use crate::model::{SYMPTOM_SUGGESTIONS, Symptom, SymptomsData, severity_label};
use crate::state::use_app;
use crate::util::{fmt_relative, fmt_short, fmt_time, today};

fn severity_class(s: i64) -> &'static str {
    match s {
        1 | 2 => "bg-good/15 text-good",
        3 => "bg-warn/15 text-warn",
        _ => "bg-bad/15 text-bad",
    }
}

#[component]
pub fn SymptomsTab() -> Element {
    let mut app = use_app();
    let res = use_resource(move || async move {
        let _ = (app.refresh)();
        symptoms_data(app.pid_tracked()).await.map_err(|e| e.to_string())
    });
    let mut name = use_signal(String::new);
    let mut severity = use_signal(|| Some(3i64));
    let mut notes = use_signal(String::new);
    let mut busy = use_signal(|| false);

    let data: SymptomsData = match &*res.read() {
        Some(Ok(d)) => d.clone(),
        Some(Err(e)) => return rsx! { p { class: "text-sm text-bad", "{e}" } },
        None => return rsx! { Skeleton { class: "h-64 rounded-2xl" } },
    };
    let date = (app.date)();

    let mut chips: Vec<String> = data.recent_names.clone();
    for s in SYMPTOM_SUGGESTIONS {
        if chips.len() >= 12 {
            break;
        }
        if !chips.iter().any(|c| c.eq_ignore_ascii_case(s)) {
            chips.push(s.to_string());
        }
    }

    let can_save = !name().trim().is_empty() && !busy();
    let mut save = move || {
        if !can_save {
            return;
        }
        busy.set(true);
        let pid = app.pid();
        let (n, sev, no) = (name(), severity().unwrap_or(3), Some(notes()));
        spawn(async move {
            match add_symptom(pid, date, n.clone(), sev, no).await {
                Ok(()) => {
                    app.toast(format!("{} erfasst", n.trim()));
                    name.set(String::new());
                    notes.set(String::new());
                    severity.set(Some(3));
                    app.bump();
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };

    // Häufigste Symptome der letzten 30 Tage.
    let since = today() - Duration::days(29);
    let mut counts: HashMap<String, (usize, i64)> = HashMap::new();
    for s in data.list.iter().filter(|s| s.date >= since) {
        let e = counts.entry(s.name.clone()).or_insert((0, 0));
        e.0 += 1;
        e.1 += s.severity;
    }
    let mut top: Vec<(String, usize, f64)> =
        counts.into_iter().map(|(k, (n, sum))| (k, n, sum as f64 / n as f64)).collect();
    top.sort_by(|a, b| b.1.cmp(&a.1).then(a.0.cmp(&b.0)));
    top.truncate(5);
    let max = top.first().map(|t| t.1).unwrap_or(1).max(1);

    // Verlauf nach Tagen gruppiert.
    let mut groups: Vec<(chrono::NaiveDate, Vec<Symptom>)> = Vec::new();
    for s in data.list.iter().take(80).cloned() {
        match groups.last_mut() {
            Some((d, v)) if *d == s.date => v.push(s),
            _ => groups.push((s.date, vec![s])),
        }
    }

    rsx! {
        div { class: "grid gap-4 lg:grid-cols-2 lg:items-start",
            div { class: "flex min-w-0 flex-col gap-4",
                Card {
                    CardHeader { title: "Symptom erfassen", subtitle: "für {fmt_relative(date)}", icon: rsx! { Stethoscope { class: "size-4" } } }
                    form {
                        class: "flex flex-col gap-4 px-4 pb-4",
                        onsubmit: move |e| {
                            e.prevent_default();
                            save();
                        },
                        div { class: "no-scrollbar -mx-4 flex gap-2 overflow-x-auto px-4 sm:flex-wrap",
                            for c in chips {
                                {
                                    let active = name().trim().eq_ignore_ascii_case(&c);
                                    let c2 = c.clone();
                                    rsx! {
                                        button {
                                            r#type: "button",
                                            key: "{c}",
                                            class: if active { "h-9 shrink-0 rounded-full bg-primary px-3.5 text-sm font-medium text-primary-foreground" } else { "h-9 shrink-0 rounded-full border px-3.5 text-sm text-muted-foreground hover:text-foreground" },
                                            onclick: move |_| name.set(c2.clone()),
                                            "{c}"
                                        }
                                    }
                                }
                            }
                        }
                        Field { label: "Symptom",
                            input { class: INPUT, placeholder: "z.B. Kopfschmerzen", value: "{name}", autocomplete: "off", oninput: move |e| name.set(e.value()) }
                        }
                        Field { label: "Stärke · {severity_label(severity().unwrap_or(3))}",
                            Scale5 {
                                value: severity(),
                                onchange: move |v: Option<i64>| severity.set(v.or(Some(3))),
                            }
                        }
                        Field { label: "Notiz",
                            input { class: INPUT, placeholder: "optional", value: "{notes}", oninput: move |e| notes.set(e.value()) }
                        }
                        Button { size: ButtonSize::Lg, class: "w-full", submit: true, disabled: !can_save,
                            if busy() { "Speichern …" } else { "Symptom speichern" }
                        }
                    }
                }
                if !top.is_empty() {
                    Card {
                        CardHeader { title: "Häufigste Symptome", subtitle: "Letzte 30 Tage" }
                        div { class: "flex flex-col gap-2.5 px-4 pb-4",
                            for (n, c, avg) in top {
                                div { key: "{n}", class: "flex flex-col gap-1",
                                    div { class: "flex items-baseline justify-between gap-2 text-sm",
                                        span { class: "truncate font-medium", "{n}" }
                                        span { class: "shrink-0 text-xs text-muted-foreground", "{c}× · Ø {crate::util::num(avg, 1)}" }
                                    }
                                    div { class: "h-1.5 w-full overflow-hidden rounded-full bg-muted",
                                        div { class: "h-full rounded-full bg-protein", style: "width: {(c as f64 / max as f64 * 100.0).round()}%" }
                                    }
                                }
                            }
                        }
                    }
                }
            }
            div { class: "flex min-w-0 flex-col gap-4",
                if groups.is_empty() {
                    Empty {
                        EmptyTitle { "Noch keine Symptome" }
                        EmptyDescription { "Erfasste Symptome erscheinen hier im Verlauf." }
                    }
                }
                for (d, items) in groups {
                    Section { key: "{d}", title: "{fmt_relative(d)} · {fmt_short(d)}", count: items.len(),
                        Card { class: "divide-y overflow-hidden",
                            for s in items {
                                SymptomRow { key: "{s.id}", s }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn SymptomRow(s: Symptom) -> Element {
    let mut app = use_app();
    let mut confirm = use_signal(|| false);
    let id = s.id.clone();
    rsx! {
        div { class: "flex items-center gap-3 px-4 py-3",
            span { class: "flex size-9 shrink-0 items-center justify-center rounded-full text-sm font-bold tabular-nums {severity_class(s.severity)}", "{s.severity}" }
            div { class: "min-w-0 flex-1",
                p { class: "truncate font-medium", "{s.name}" }
                p { class: "truncate text-xs text-muted-foreground",
                    "{severity_label(s.severity)} · {fmt_time(s.created_at)}"
                    if let Some(n) = s.notes.as_ref() { " · {n}" }
                }
            }
            button {
                r#type: "button",
                class: if confirm() { "flex h-10 shrink-0 items-center gap-1 rounded-full bg-destructive px-3 text-xs font-medium text-white" } else { "flex size-10 shrink-0 items-center justify-center rounded-full text-muted-foreground hover:bg-accent" },
                onclick: move |_| {
                    if !confirm() {
                        confirm.set(true);
                        return;
                    }
                    let pid = app.pid();
                    let id = id.clone();
                    spawn(async move {
                        match delete_symptom(pid, id).await {
                            Ok(()) => app.bump(),
                            Err(e) => app.error(e),
                        }
                    });
                },
                Trash2 { class: "size-4" }
                if confirm() { "Löschen" }
            }
        }
    }
}

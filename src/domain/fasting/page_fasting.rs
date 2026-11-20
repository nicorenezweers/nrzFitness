use chrono::NaiveDateTime;
use dioxus::prelude::*;
use icons::{Check, Circle, CircleCheck, Pencil, Play, Square};

use super::fast_sheet::FastSheet;
use crate::api::fasting::{end_fast, fasting_data, start_fast};
use crate::components::charts::BarChart;
use crate::components::layout::page::{PageBody, PageHeader, Section};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::{Card, CardHeader, Stat};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::field::INPUT;
use crate::components::ui::ring::Ring;
use crate::components::ui::segmented::Segmented;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{Fast, FastingData, PHASES, PRESETS, phase_index, preset_label, stats};
use crate::state::use_app;
use crate::util::{
    datetime_input, fmt_dm, fmt_duration_min, fmt_hms, fmt_relative, fmt_time, now, num, parse_datetime, today,
};

/// "Heute 20:15"
fn fmt_dt(dt: NaiveDateTime) -> String {
    format!("{} {}", fmt_relative(dt.date()), fmt_time(dt))
}

/// Aktuelle Uhrzeit, jede Sekunde aktualisiert (nur Komponenten, die das Signal lesen, rendern neu).
fn use_now() -> Signal<NaiveDateTime> {
    let mut t = use_signal(now);
    use_future(move || async move {
        loop {
            if document::eval("await new Promise(r => setTimeout(r, 1000)); return 1;").await.is_err() {
                break;
            }
            t.set(now());
        }
    });
    t
}

#[component]
pub fn FastingPage() -> Element {
    let mut app = use_app();
    let clock = use_now();
    let data = use_resource(move || async move {
        let _ = (app.refresh)();
        fasting_data(app.pid_tracked()).await.map_err(|e| e.to_string())
    });
    let mut target = use_signal(|| None::<f64>);
    let mut start_mode = use_signal(|| "now".to_string());
    let mut start_at = use_signal(|| datetime_input(now()));
    let mut edit = use_signal(|| None::<Fast>);
    let mut busy = use_signal(|| false);
    let mut confirm_end = use_signal(|| false);

    let d: FastingData = match &*data.read() {
        Some(Ok(d)) => d.clone(),
        Some(Err(e)) => {
            return rsx! {
                PageHeader { title: "Fasten" }
                PageBody { p { class: "text-sm text-bad", "{e}" } }
            };
        }
        None => {
            return rsx! {
                PageHeader { title: "Fasten" }
                PageBody {
                    Skeleton { class: "h-96 rounded-2xl" }
                }
            };
        }
    };
    let goal = target().unwrap_or(d.default_hours);
    let st = stats(&d.history, today());
    let last_end = d.history.first().and_then(|f| f.end_at);

    let start = move |_| {
        if busy() {
            return;
        }
        let at = if start_mode() == "now" { Some(now()) } else { parse_datetime(&start_at()) };
        let Some(at) = at else {
            app.error("Startzeit ungültig");
            return;
        };
        if at > now() {
            app.error("Startzeit liegt in der Zukunft");
            return;
        }
        busy.set(true);
        let pid = app.pid();
        spawn(async move {
            match start_fast(pid, at, goal).await {
                Ok(_) => {
                    app.toast(format!("Fasten gestartet · Ziel {} h", num(goal, 1)));
                    start_mode.set("now".into());
                    app.bump();
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };

    let active = d.active.clone();
    let end = {
        let active = active.clone();
        move |_| {
            let Some(f) = active.clone() else { return };
            if !f.reached(now()) && !confirm_end() {
                confirm_end.set(true);
                return;
            }
            busy.set(true);
            let pid = app.pid();
            spawn(async move {
                match end_fast(pid, f.id.clone(), now()).await {
                    Ok(f) => {
                        let h = f.hours(now());
                        app.toast(if f.reached(now()) {
                            format!("Geschafft! {} h gefastet 🎉", num(h, 1))
                        } else {
                            format!("Fasten beendet · {} h", num(h, 1))
                        });
                        confirm_end.set(false);
                        app.bump();
                    }
                    Err(e) => app.error(e),
                }
                busy.set(false);
            });
        }
    };

    let bars: Vec<(String, f64)> = d
        .history
        .iter()
        .take(14)
        .rev()
        .map(|f| (fmt_dm(f.start_at.date()), (f.hours(now()) * 10.0).round() / 10.0))
        .collect();

    rsx! {
        PageHeader { title: "Fasten", subtitle: "Intervallfasten mit Live-Timer" }
        PageBody {
            div { class: "grid gap-4 lg:grid-cols-2 lg:items-start",
                div { class: "flex min-w-0 flex-col gap-4",
                    Card { class: "flex flex-col items-center gap-5 px-4 pb-5 pt-6",
                        TimerRing { clock, active: active.clone(), last_end, goal }
                        if let Some(f) = active.clone() {
                            {
                                let f2 = f.clone();
                                let f3 = f.clone();
                                rsx! {
                                    div { class: "grid w-full grid-cols-2 gap-2",
                                        button {
                                            r#type: "button",
                                            class: "flex flex-col items-start rounded-xl bg-muted/70 px-3 py-2.5 text-left active:scale-[0.98]",
                                            onclick: move |_| edit.set(Some(f2.clone())),
                                            span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Gestartet" }
                                            span { class: "flex items-center gap-1.5 font-semibold", "{fmt_dt(f.start_at)}" Pencil { class: "size-3 text-muted-foreground" } }
                                        }
                                        div { class: "flex flex-col items-start rounded-xl bg-muted/70 px-3 py-2.5",
                                            span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Ziel ({preset_label(f.target_hours)})" }
                                            span { class: "font-semibold", "{fmt_dt(f.target_end())}" }
                                        }
                                    }
                                    div { class: "flex w-full gap-2",
                                        Button {
                                            size: ButtonSize::Lg,
                                            variant: if confirm_end() { ButtonVariant::Destructive } else { ButtonVariant::Default },
                                            class: "flex-1",
                                            disabled: busy(),
                                            onclick: end,
                                            Square { class: "size-4" }
                                            if confirm_end() { "Ziel nicht erreicht – trotzdem beenden?" } else { "Fasten beenden" }
                                        }
                                        Button {
                                            size: ButtonSize::Lg,
                                            variant: ButtonVariant::Outline,
                                            class: "w-12 px-0",
                                            onclick: move |_| edit.set(Some(f3.clone())),
                                            Pencil { class: "size-4" }
                                        }
                                    }
                                }
                            }
                        } else {
                            div { class: "flex w-full flex-col gap-2",
                                span { class: "px-1 text-xs font-medium text-muted-foreground", "Ziel" }
                                div { class: "grid grid-cols-4 gap-2 sm:grid-cols-7",
                                    for h in PRESETS {
                                        button {
                                            r#type: "button",
                                            key: "{h}",
                                            class: if (goal - h).abs() < 0.01 { "flex h-14 flex-col items-center justify-center rounded-xl bg-primary text-primary-foreground transition-transform active:scale-95" } else { "flex h-14 flex-col items-center justify-center rounded-xl bg-muted/70 transition-transform hover:bg-accent active:scale-95" },
                                            onclick: move |_| target.set(Some(h)),
                                            span { class: "text-sm font-semibold", "{preset_label(h)}" }
                                            span { class: "text-[10px] opacity-70", "{num(h, 0)} h" }
                                        }
                                    }
                                }
                                if !PRESETS.iter().any(|h| (goal - h).abs() < 0.01) {
                                    p { class: "px-1 text-xs text-muted-foreground", "Eigenes Ziel aus den Zielen: {num(goal, 1)} h" }
                                }
                            }
                            div { class: "flex w-full flex-col gap-2",
                                Segmented {
                                    options: vec![("now".into(), "Jetzt starten".into()), ("custom".into(), "Früher begonnen".into())],
                                    value: start_mode(),
                                    onchange: move |v: String| {
                                        if v == "custom" {
                                            start_at.set(datetime_input(now()));
                                        }
                                        start_mode.set(v);
                                    },
                                }
                                if start_mode() == "custom" {
                                    input {
                                        class: INPUT,
                                        r#type: "datetime-local",
                                        value: "{start_at}",
                                        max: "{datetime_input(now())}",
                                        oninput: move |e| start_at.set(e.value()),
                                    }
                                }
                            }
                            Button { size: ButtonSize::Lg, class: "w-full", disabled: busy(), onclick: start,
                                Play { class: "size-4" }
                                "Fasten starten"
                            }
                        }
                    }
                    PhasesCard { clock, active: active.clone() }
                }
                div { class: "flex min-w-0 flex-col gap-4",
                    Card {
                        CardHeader { title: "Statistik", subtitle: if st.count > 0 { format!("{} % mit erreichtem Ziel", num(st.success_rate, 0)) } else { "Noch keine abgeschlossene Fastenzeit".to_string() } }
                        div { class: "grid grid-cols-2 gap-4 px-4 pb-4 sm:grid-cols-4",
                            Stat { label: "Anzahl", value: "{st.count}" }
                            Stat { label: "Schnitt", value: num(st.avg_hours, 1), unit: "h" }
                            Stat { label: "Längstes", value: num(st.longest_hours, 1), unit: "h" }
                            Stat { label: "Serie", value: "{st.streak}", unit: if st.streak == 1 { "Tag" } else { "Tage" } }
                        }
                    }
                    Card {
                        CardHeader { title: "Letzte Fastenzeiten", subtitle: "Stunden, Linie = Ziel" }
                        div { class: "px-3 pb-4",
                            BarChart { bars, goal: Some(d.default_hours), color: "text-carbs", over_color: "text-good" }
                        }
                    }
                    Section { title: "Verlauf", count: d.history.len(),
                        if d.history.is_empty() {
                            Empty {
                                EmptyTitle { "Noch kein Verlauf" }
                                EmptyDescription { "Abgeschlossene Fastenzeiten erscheinen hier." }
                            }
                        } else {
                            Card { class: "divide-y overflow-hidden",
                                for f in d.history.iter().take(30).cloned() {
                                    HistoryRow { key: "{f.id}", f: f.clone(), onclick: move |f| edit.set(Some(f)) }
                                }
                            }
                        }
                    }
                }
            }
        }
        if let Some(f) = edit() {
            FastSheet { key: "{f.id}", fast: f, onclose: move |_| edit.set(None) }
        }
    }
}

#[component]
fn TimerRing(
    clock: Signal<NaiveDateTime>,
    active: Option<Fast>,
    last_end: Option<NaiveDateTime>,
    goal: f64,
) -> Element {
    let t = clock();
    match active {
        Some(f) => {
            let secs = f.secs(t);
            let h = secs as f64 / 3600.0;
            let target_secs = (f.target_hours * 3600.0).round() as i64;
            let p = &PHASES[phase_index(h)];
            let rest = target_secs - secs;
            rsx! {
                Ring { value: h / f.target_hours, size: 252.0, stroke: 16.0, color: "text-carbs", over_color: "text-good",
                    span { class: "text-2xl", "{p.emoji}" }
                    span { class: "text-xs font-medium text-muted-foreground", "{p.name}" }
                    span { class: "mt-1 text-[2.6rem] font-bold leading-none tabular-nums tracking-tight", "{fmt_hms(secs)}" }
                    if rest > 0 {
                        span { class: "mt-2 text-xs text-muted-foreground", "noch {fmt_hms(rest)}" }
                        span { class: "text-xs font-medium tabular-nums", "{num((h / f.target_hours * 100.0).min(100.0), 0)} %" }
                    } else {
                        span { class: "mt-2 flex items-center gap-1 text-sm font-semibold text-good", Check { class: "size-4" } "Ziel erreicht" }
                        span { class: "text-xs text-muted-foreground tabular-nums", "+{fmt_hms(-rest)}" }
                    }
                }
            }
        }
        None => {
            let since = last_end.map(|e| (t - e).num_minutes());
            rsx! {
                Ring { value: 0.0, size: 252.0, stroke: 16.0, color: "text-carbs",
                    span { class: "text-2xl", "🍽️" }
                    span { class: "mt-1 text-xl font-semibold", "Bereit" }
                    span { class: "text-sm text-muted-foreground", "Ziel {num(goal, 1)} h" }
                    if let Some(m) = since.filter(|m| *m >= 0) {
                        span { class: "mt-2 max-w-[11rem] text-xs text-muted-foreground", "Letztes Fasten endete vor {fmt_duration_min(m)}" }
                    }
                }
            }
        }
    }
}

#[component]
fn PhasesCard(clock: Signal<NaiveDateTime>, active: Option<Fast>) -> Element {
    let t = clock();
    let hours = active.as_ref().map(|f| f.hours(t));
    let cur = hours.map(phase_index);
    rsx! {
        Card {
            CardHeader { title: "Fastenphasen", subtitle: if active.is_some() { "Wo du gerade stehst" } else { "Was im Körper passiert" } }
            div { class: "flex flex-col gap-1 px-2 pb-3",
                for (i, p) in PHASES.iter().enumerate() {
                    {
                        let state = match cur { Some(c) if i < c => 0, Some(c) if i == c => 1, _ => 2 };
                        let at = active.as_ref().map(|f| f.start_at + chrono::Duration::minutes((p.from_h * 60.0) as i64));
                        let row = if state == 1 { "flex items-start gap-3 rounded-xl bg-carbs/12 px-2 py-2.5 ring-1 ring-carbs/40" } else { "flex items-start gap-3 rounded-xl px-2 py-2.5" };
                        rsx! {
                            div { key: "{i}", class: "{row}",
                                div { class: if state == 2 && active.is_some() { "flex size-9 shrink-0 items-center justify-center rounded-full bg-muted text-lg opacity-60" } else { "flex size-9 shrink-0 items-center justify-center rounded-full bg-muted text-lg" },
                                    "{p.emoji}"
                                }
                                div { class: "min-w-0 flex-1",
                                    div { class: "flex items-baseline justify-between gap-2",
                                        span { class: if state == 1 { "text-sm font-semibold" } else { "text-sm font-medium" }, "{p.name}" }
                                        span { class: "shrink-0 text-[11px] text-muted-foreground tabular-nums",
                                            if p.from_h == 0.0 { "Start" } else { "ab {num(p.from_h, 0)} h" }
                                        }
                                    }
                                    p { class: "text-xs text-muted-foreground", "{p.text}" }
                                    if let Some(at) = at.filter(|_| i > 0) {
                                        if state == 0 {
                                            p { class: "mt-0.5 flex items-center gap-1 text-[11px] text-good", CircleCheck { class: "size-3" } "erreicht {fmt_dt(at)}" }
                                        } else if state == 2 {
                                            p { class: "mt-0.5 text-[11px] text-muted-foreground", "um {fmt_dt(at)}" }
                                        }
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn HistoryRow(f: Fast, onclick: EventHandler<Fast>) -> Element {
    let t = now();
    let ok = f.reached(t);
    let end = f.end_at.unwrap_or(t);
    let range = if end.date() == f.start_at.date() {
        format!("{} – {}", fmt_time(f.start_at), fmt_time(end))
    } else {
        format!("{} – {} {}", fmt_time(f.start_at), fmt_dm(end.date()), fmt_time(end))
    };
    let f2 = f.clone();
    rsx! {
        button {
            r#type: "button",
            class: "flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-accent/40 active:bg-accent/60",
            onclick: move |_| onclick.call(f2.clone()),
            if ok {
                CircleCheck { class: "size-6 shrink-0 text-good" }
            } else {
                Circle { class: "size-6 shrink-0 text-muted-foreground" }
            }
            div { class: "min-w-0 flex-1",
                p { class: "text-sm font-medium", "{fmt_relative(f.start_at.date())}" }
                p { class: "truncate text-xs text-muted-foreground tabular-nums", "{range}" }
            }
            div { class: "flex flex-col items-end",
                span { class: "text-sm font-semibold tabular-nums", "{fmt_duration_min(f.secs(t) / 60)}" }
                span { class: if ok { "text-[11px] text-good" } else { "text-[11px] text-muted-foreground" },
                    if ok { "Ziel {num(f.target_hours, 0)} h ✓" } else { "Ziel {num(f.target_hours, 0)} h" }
                }
            }
        }
    }
}

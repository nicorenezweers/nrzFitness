use std::collections::HashMap;

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use icons::{ChevronLeft, ChevronRight, Droplet, Flower2, Settings};

use crate::api::cycle::{cycle_days, set_cycle_day, start_period};
use crate::app::Route;
use crate::components::layout::page::{PageBody, PageHeader, Section};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::{Card, CardHeader, Stat};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::ring::Ring;
use crate::components::ui::sheet::Sheet;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{CycleDay, CycleInfo, DayMark, FLOWS, analyze, flow_label, predicted_marks};
use crate::state::use_app;
use crate::util::{MONTHS, WEEKDAYS_SHORT, add_months, fmt_long, fmt_short, month_grid, today};

/// "in 6 Tagen" / "heute" / "morgen"
fn in_days(d: NaiveDate) -> String {
    match (d - today()).num_days() {
        0 => "heute".into(),
        1 => "morgen".into(),
        n if n > 1 => format!("in {n} Tagen"),
        -1 => "gestern".into(),
        n => format!("vor {} Tagen", -n),
    }
}

#[component]
pub fn CyclePage() -> Element {
    let app = use_app();
    let enabled = app.profile().is_some_and(|p| p.cycle_tracking);
    if !enabled {
        return rsx! {
            PageHeader { title: "Zyklus" }
            PageBody {
                Card { class: "flex flex-col items-center gap-4 p-8 text-center",
                    div { class: "flex size-14 items-center justify-center rounded-full bg-fat/15 text-fat", Flower2 { class: "size-7" } }
                    div { class: "flex flex-col gap-1",
                        p { class: "text-lg font-semibold", "Zyklus-Tracking ist aus" }
                        p { class: "max-w-sm text-sm text-muted-foreground",
                            "Aktiviere das Zyklus-Tracking in deinem Profil, um Periode, Eisprung und fruchtbare Tage zu verfolgen."
                        }
                    }
                    Link { to: Route::Settings {},
                        class: "inline-flex h-12 items-center gap-2 rounded-2xl bg-primary px-6 text-sm font-medium text-primary-foreground transition-transform active:scale-[0.97]",
                        Settings { class: "size-4" }
                        "Zu den Einstellungen"
                    }
                }
            }
        };
    }
    rsx! { CycleTracker {} }
}

#[component]
fn CycleTracker() -> Element {
    let mut app = use_app();
    let t = today();
    let mut month = use_signal(|| (t.year(), t.month()));
    let mut pick = use_signal(|| None::<NaiveDate>);
    let mut busy = use_signal(|| false);
    let res = use_resource(move || async move {
        let _ = (app.refresh)();
        cycle_days(app.pid_tracked()).await.map_err(|e| e.to_string())
    });

    let days: Vec<CycleDay> = match &*res.read() {
        Some(Ok(d)) => d.clone(),
        Some(Err(e)) => {
            return rsx! {
                PageHeader { title: "Zyklus" }
                PageBody { p { class: "text-sm text-bad", "{e}" } }
            };
        }
        None => {
            return rsx! {
                PageHeader { title: "Zyklus" }
                PageBody { Skeleton { class: "h-96 rounded-2xl" } }
            };
        }
    };
    let info = analyze(&days, t);
    let flows: HashMap<NaiveDate, i64> = days.iter().map(|d| (d.date, d.flow)).collect();
    let mut marks: HashMap<NaiveDate, DayMark> = HashMap::new();
    for (d, m) in predicted_marks(&info, 4) {
        // Periode hat Vorrang vor fruchtbaren Tagen.
        let e = marks.entry(d).or_insert(DayMark::None);
        if *e == DayMark::None || m == DayMark::PredictedPeriod {
            *e = m;
        }
    }
    // Fruchtbares Fenster des laufenden Zyklus (kann vor heute liegen).
    if let (Some((fs, fe)), Some(ov)) = (info.fertile, info.ovulation) {
        let mut d = fs;
        while d <= fe {
            marks.entry(d).or_insert(if d == ov { DayMark::Ovulation } else { DayMark::Fertile });
            d += Duration::days(1);
        }
    }

    let n = info.avg_period;
    let start_now = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        let pid = app.pid();
        spawn(async move {
            match start_period(pid, today(), n).await {
                Ok(()) => {
                    app.toast("Periode eingetragen – Tage kannst du im Kalender anpassen");
                    app.bump();
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };
    let (y, m) = month();
    let grid = month_grid(y, m);

    rsx! {
        PageHeader { title: "Zyklus", subtitle: "Periode, Eisprung & fruchtbare Tage" }
        PageBody {
            div { class: "grid gap-4 lg:grid-cols-2 lg:items-start",
                div { class: "flex min-w-0 flex-col gap-4",
                    Summary { info: info.clone(), busy: busy(), onstart: start_now }
                    Card {
                        div { class: "flex items-center gap-1 px-2 pt-3 pb-1",
                            button {
                                r#type: "button",
                                class: "flex size-11 items-center justify-center rounded-full text-muted-foreground hover:bg-accent active:scale-90",
                                onclick: move |_| month.set(add_months(y, m, -1)),
                                ChevronLeft { class: "size-5" }
                            }
                            button {
                                r#type: "button",
                                class: "flex-1 text-center text-[15px] font-semibold",
                                onclick: move |_| month.set((today().year(), today().month())),
                                "{MONTHS[m as usize - 1]} {y}"
                            }
                            button {
                                r#type: "button",
                                class: "flex size-11 items-center justify-center rounded-full text-muted-foreground hover:bg-accent active:scale-90",
                                onclick: move |_| month.set(add_months(y, m, 1)),
                                ChevronRight { class: "size-5" }
                            }
                        }
                        div { class: "grid grid-cols-7 gap-1 px-2 pb-1",
                            for w in WEEKDAYS_SHORT {
                                div { key: "{w}", class: "py-1 text-center text-[11px] font-medium text-muted-foreground", "{w}" }
                            }
                        }
                        div { class: "grid grid-cols-7 gap-1 px-2 pb-3",
                            for d in grid {
                                DayCell {
                                    key: "{d}",
                                    date: d,
                                    in_month: d.month() == m,
                                    flow: flows.get(&d).copied(),
                                    mark: marks.get(&d).copied().unwrap_or(DayMark::None),
                                    onclick: move |d| pick.set(Some(d)),
                                }
                            }
                        }
                        Legend {}
                    }
                }
                div { class: "flex min-w-0 flex-col gap-4",
                    Card {
                        CardHeader { title: "Durchschnitt", subtitle: if info.from_history { "Aus deinen Daten" } else { "Standardwerte – mehr Daten verbessern die Vorhersage" } }
                        div { class: "grid grid-cols-3 gap-3 px-4 pb-4",
                            Stat { label: "Zyklus", value: "{info.avg_cycle}", unit: "Tage" }
                            Stat { label: "Periode", value: "{info.avg_period}", unit: "Tage" }
                            Stat { label: "Erfasst", value: "{info.periods.len()}", unit: if info.periods.len() == 1 { "Periode" } else { "Perioden" } }
                        }
                    }
                    Section { title: "Vergangene Zyklen", count: info.periods.len(),
                        if info.periods.is_empty() {
                            Empty {
                                EmptyTitle { "Noch keine Periode erfasst" }
                                EmptyDescription { "Tippe im Kalender auf einen Tag, um die Stärke einzutragen." }
                            }
                        } else {
                            Card { class: "divide-y overflow-hidden",
                                for p in info.periods.iter().rev().take(12).cloned() {
                                    {
                                        let len = p.cycle_len;
                                        let scale = len.unwrap_or(info.avg_cycle).max(p.days()).max(1) as f64;
                                        let pw = (p.days() as f64 / scale * 100.0).round();
                                        rsx! {
                                            div { key: "{p.start}", class: "flex flex-col gap-2 px-4 py-3",
                                                div { class: "flex items-baseline justify-between gap-2",
                                                    span { class: "text-sm font-medium", "{fmt_short(p.start)} {p.start.year()}" }
                                                    span { class: "text-xs text-muted-foreground",
                                                        if let Some(l) = len { "Zyklus {l} Tage" } else { "laufender Zyklus" }
                                                    }
                                                }
                                                div { class: "flex h-2 w-full overflow-hidden rounded-full bg-muted",
                                                    div { class: "h-full rounded-full bg-fat", style: "width: {pw}%" }
                                                }
                                                span { class: "text-[11px] text-muted-foreground", "Periode {p.days()} Tage · bis {fmt_short(p.end)}" }
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
        if let Some(d) = pick() {
            DaySheet { key: "{d}", date: d, flow: flows.get(&d).copied(), info: info.clone(), mark: marks.get(&d).copied().unwrap_or(DayMark::None), onclose: move |_| pick.set(None) }
        }
    }
}

#[component]
fn Summary(info: CycleInfo, busy: bool, onstart: EventHandler<()>) -> Element {
    let ring = info.cycle_day.map(|c| c as f64 / info.avg_cycle.max(1) as f64).unwrap_or(0.0);
    let headline = if info.in_period {
        format!("Periode · Tag {}", info.cycle_day.unwrap_or(1))
    } else if info.late_days > 0 {
        format!("{} {} überfällig", info.late_days, if info.late_days == 1 { "Tag" } else { "Tage" })
    } else if let Some(n) = info.next_period {
        format!("Periode {}", in_days(n))
    } else {
        "Noch keine Daten".into()
    };
    let t = today();
    let fertile_now = info.fertile.is_some_and(|(a, b)| t >= a && t <= b);
    rsx! {
        Card { class: "flex flex-col gap-4 p-4",
            div { class: "flex items-center gap-4",
                Ring { value: ring.min(1.0), size: 112.0, stroke: 10.0, color: "text-fat", over_color: "text-fat",
                    if let Some(c) = info.cycle_day {
                        span { class: "text-[10px] font-medium uppercase tracking-wide text-muted-foreground", "Tag" }
                        span { class: "text-3xl font-bold leading-none tabular-nums", "{c}" }
                        span { class: "text-[10px] text-muted-foreground", "von ~{info.avg_cycle}" }
                    } else {
                        Flower2 { class: "size-8 text-fat" }
                    }
                }
                div { class: "flex min-w-0 flex-1 flex-col gap-1",
                    p { class: "text-lg font-semibold leading-tight", "{headline}" }
                    if let Some(n) = info.next_period.filter(|_| !info.in_period) {
                        p { class: "text-sm text-muted-foreground", "Nächste: {fmt_short(n)}" }
                    }
                    if fertile_now {
                        span { class: "w-fit rounded-full bg-water/20 px-2.5 py-0.5 text-xs font-medium text-water", "Fruchtbare Tage" }
                    }
                }
            }
            if info.next_period.is_some() {
                div { class: "grid grid-cols-2 gap-2",
                    if let Some(ov) = info.ovulation {
                        div { class: "flex flex-col rounded-xl bg-muted/70 px-3 py-2.5",
                            span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Eisprung" }
                            span { class: "font-semibold", "{fmt_short(ov)}" }
                            span { class: "text-[11px] text-muted-foreground", "{in_days(ov)}" }
                        }
                    }
                    if let Some((a, b)) = info.fertile {
                        div { class: "flex flex-col rounded-xl bg-muted/70 px-3 py-2.5",
                            span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Fruchtbar" }
                            span { class: "font-semibold", "{fmt_short(a)} – {fmt_short(b)}" }
                            span { class: "text-[11px] text-muted-foreground", "geschätzt" }
                        }
                    }
                }
            }
            if !info.in_period {
                Button { size: ButtonSize::Lg, variant: ButtonVariant::Secondary, class: "w-full", disabled: busy, onclick: move |_| onstart.call(()),
                    Droplet { class: "size-4 text-fat" }
                    "Periode beginnt heute"
                }
            }
        }
    }
}

#[component]
fn DayCell(
    date: NaiveDate,
    in_month: bool,
    flow: Option<i64>,
    mark: DayMark,
    onclick: EventHandler<NaiveDate>,
) -> Element {
    let is_today = date == today();
    let base = "relative flex aspect-square min-h-11 flex-col items-center justify-center rounded-xl text-sm tabular-nums transition-transform active:scale-90";
    let fill = match (flow, mark) {
        (Some(1), _) => "bg-fat/30 font-semibold",
        (Some(2), _) => "bg-fat/55 font-semibold",
        (Some(_), _) => "bg-fat/80 font-semibold",
        (None, DayMark::PredictedPeriod) => "border-2 border-dashed border-fat/70",
        (None, DayMark::Ovulation) => "bg-water/35 font-semibold",
        (None, DayMark::Fertile) => "bg-water/15",
        _ => "hover:bg-accent",
    };
    let dim = if in_month { "" } else { "opacity-35" };
    let ring = if is_today { "ring-2 ring-foreground ring-offset-1 ring-offset-card" } else { "" };
    rsx! {
        button {
            r#type: "button",
            class: "{base} {fill} {dim} {ring}",
            onclick: move |_| onclick.call(date),
            "{date.day()}"
            if let Some(f) = flow {
                span { class: "absolute bottom-1 flex gap-0.5",
                    for i in 0..f {
                        span { key: "{i}", class: "size-1 rounded-full bg-foreground/70" }
                    }
                }
            } else if mark == DayMark::Ovulation {
                span { class: "absolute bottom-1 size-1 rounded-full bg-water" }
            }
        }
    }
}

#[component]
fn Legend() -> Element {
    rsx! {
        div { class: "flex flex-wrap gap-x-4 gap-y-1.5 border-t px-4 py-3 text-[11px] text-muted-foreground",
            span { class: "flex items-center gap-1.5", span { class: "size-3 rounded bg-fat/60" } "Periode" }
            span { class: "flex items-center gap-1.5", span { class: "size-3 rounded border-2 border-dashed border-fat/70" } "Vorhersage" }
            span { class: "flex items-center gap-1.5", span { class: "size-3 rounded bg-water/20" } "Fruchtbar" }
            span { class: "flex items-center gap-1.5", span { class: "size-3 rounded bg-water/45" } "Eisprung" }
        }
    }
}

#[component]
fn DaySheet(date: NaiveDate, flow: Option<i64>, info: CycleInfo, mark: DayMark, onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let mut busy = use_signal(|| false);
    let mut set = move |f: Option<i64>| {
        if busy() {
            return;
        }
        busy.set(true);
        let pid = app.pid();
        spawn(async move {
            match set_cycle_day(pid, date, f).await {
                Ok(()) => {
                    app.toast(match f {
                        Some(f) => format!("{} eingetragen", flow_label(f)),
                        None => "Periodentag entfernt".into(),
                    });
                    app.bump();
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };
    let last_start = info.periods.iter().rev().find(|p| p.start <= date).map(|p| p.start);
    let cycle_day = last_start.map(|s| (date - s).num_days() + 1).filter(|d| *d <= 60);
    let hint = match mark {
        DayMark::PredictedPeriod => Some("Vorhergesagte Periode"),
        DayMark::Ovulation => Some("Voraussichtlicher Eisprung"),
        DayMark::Fertile => Some("Voraussichtlich fruchtbar"),
        DayMark::None => None,
    };
    rsx! {
        Sheet { title: fmt_long(date), onclose: move |_| onclose.call(()),
            div { class: "flex flex-col gap-4 px-5 pb-5 pt-1",
                div { class: "flex flex-wrap gap-2 text-xs",
                    if let Some(c) = cycle_day {
                        span { class: "rounded-full bg-muted px-2.5 py-1 font-medium", "Zyklustag {c}" }
                    }
                    if let Some(h) = hint {
                        span { class: "rounded-full bg-muted px-2.5 py-1 text-muted-foreground", "{h}" }
                    }
                }
                if date > today() {
                    p { class: "text-sm text-muted-foreground", "Für zukünftige Tage kann noch nichts eingetragen werden." }
                } else {
                    p { class: "text-sm font-medium", "Periodenstärke" }
                    div { class: "grid grid-cols-3 gap-2",
                        for (f, label) in FLOWS {
                            button {
                                r#type: "button",
                                key: "{f}",
                                disabled: busy(),
                                class: if flow == Some(f) { "flex h-20 flex-col items-center justify-center gap-1.5 rounded-2xl bg-fat/25 ring-2 ring-fat transition-transform active:scale-95" } else { "flex h-20 flex-col items-center justify-center gap-1.5 rounded-2xl bg-muted/70 transition-transform hover:bg-accent active:scale-95" },
                                onclick: move |_| set(Some(f)),
                                span { class: "flex gap-0.5 text-fat",
                                    for i in 0..f {
                                        Droplet { key: "{i}", class: "size-4 fill-current" }
                                    }
                                }
                                span { class: "text-sm font-medium", "{label}" }
                            }
                        }
                    }
                    if flow.is_some() {
                        Button { size: ButtonSize::Lg, variant: ButtonVariant::Outline, class: "w-full", disabled: busy(), onclick: move |_| set(None),
                            "Kein Periodentag"
                        }
                    }
                }
            }
        }
    }
}

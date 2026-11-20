use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use icons::{ChevronRight, Copy, Dumbbell, Footprints, GlassWater, Minus, Moon, Plus, Scale, Smile, Timer};

use crate::api::diary::{add_water, copy_day, get_day};
use crate::api::profile::get_goals;
use crate::app::Route;
use crate::components::layout::page::PageHeader;
use crate::components::ui::card::Card;
use crate::components::ui::ring::{Bar, Ring};
use crate::components::ui::skeleton::Skeleton;
use crate::domain::foods::amount_sheet::EditEntrySheet;
use crate::model::{DayData, FoodEntry, Goals, MEALS};
use crate::state::use_app;
use crate::util::{WEEKDAYS_SHORT, fmt_duration_min, fmt_long, fmt_relative, greeting, int, num, today, week_start};

#[component]
pub fn TodayPage() -> Element {
    let mut app = use_app();
    let date = (app.date)();

    let mut day = use_resource(move || async move {
        let pid = app.pid_tracked();
        let d = (app.date)();
        let _ = (app.refresh)();
        get_day(pid, d).await
    });
    let goals = use_resource(move || async move {
        let pid = app.pid_tracked();
        let _ = (app.refresh)();
        get_goals(pid).await.unwrap_or_default()
    });
    let mut editing = use_signal(|| None::<FoodEntry>);
    let mut touch_x = use_signal(|| None::<(f64, f64)>);

    let name = app.profile().map(|p| p.name.split_whitespace().next().unwrap_or("").to_string()).unwrap_or_default();
    let title = if date == today() { format!("{}, {name}", greeting()) } else { fmt_relative(date) };
    let data: Option<DayData> = match &*day.read() {
        Some(Ok(d)) => Some(d.clone()),
        _ => None,
    };
    let err = match &*day.read() {
        Some(Err(e)) => Some(e.to_string()),
        _ => None,
    };
    let g: Goals = goals.read().clone().unwrap_or_default();

    rsx! {
        div {
            class: "flex flex-col",
            // Wischen links/rechts = Tag wechseln
            ontouchstart: move |e| {
                if let Some(t) = e.data().touches().first() {
                    let p = t.client_coordinates();
                    touch_x.set(Some((p.x, p.y)));
                }
            },
            ontouchend: move |e| {
                if let (Some((x0, y0)), Some(t)) = (touch_x(), e.data().touches_changed().first()) {
                    let p = t.client_coordinates();
                    let (dx, dy) = (p.x - x0, p.y - y0);
                    if dx.abs() > 70.0 && dx.abs() > dy.abs() * 1.8 {
                        let d = (app.date)();
                        if dx < 0.0 && d < today() { app.date.set(d + Duration::days(1)); }
                        if dx > 0.0 { app.date.set(d - Duration::days(1)); }
                    }
                }
                touch_x.set(None);
            },
            PageHeader { title, subtitle: fmt_long(date) }
            WeekStrip { date, onchange: move |d| app.date.set(d) }
            div { class: "grid gap-4 px-4 pb-8 pt-2 sm:px-8 lg:grid-cols-[minmax(0,1fr)_minmax(0,1fr)] lg:items-start",
                if let Some(e) = err {
                    Card { class: "p-4 text-sm text-destructive lg:col-span-2", "Fehler: {e}" }
                }
                match data {
                    None => rsx! {
                        Skeleton { class: "h-64 rounded-2xl" }
                        Skeleton { class: "h-40 rounded-2xl" }
                    },
                    Some(d) => rsx! {
                        div { class: "flex flex-col gap-4",
                            SummaryCard { d: d.clone(), g: g.clone() }
                            ActivityGrid { d: d.clone(), g: g.clone() }
                            WaterCard { d: d.clone(), g: g.clone(), date, onchange: move |_| day.restart() }
                        }
                        div { class: "flex flex-col gap-3",
                            for (k, l) in MEALS {
                                MealCard {
                                    key: "{k}",
                                    meal: k,
                                    label: l,
                                    entries: d.entries.iter().filter(|e| e.meal == k).cloned().collect::<Vec<_>>(),
                                    date,
                                    onedit: move |e| editing.set(Some(e)),
                                }
                            }
                            NutrientDetails { d: d.clone(), g: g.clone() }
                        }
                    },
                }
            }
        }
        if let Some(e) = editing() {
            EditEntrySheet { key: "{e.id}", entry: e, onclose: move |_| editing.set(None) }
        }
    }
}

/// Wochenleiste Mo–So, Tag antippen. Pfeile blättern wochenweise über die DateNav-Logik.
#[component]
fn WeekStrip(date: NaiveDate, onchange: EventHandler<NaiveDate>) -> Element {
    let start = week_start(date);
    let t = today();
    rsx! {
        div { class: "flex items-center gap-1 px-2 pb-1 sm:px-6",
            button {
                class: "flex size-9 shrink-0 items-center justify-center rounded-full text-muted-foreground active:scale-90",
                onclick: move |_| onchange.call(date - Duration::days(7)),
                "‹"
            }
            div { class: "grid flex-1 grid-cols-7 gap-1",
                for i in 0..7 {
                    {
                        let d = start + Duration::days(i);
                        let sel = d == date;
                        let future = d > t;
                        let is_today = d == t;
                        rsx! {
                            button {
                                key: "{d}",
                                disabled: future,
                                class: "flex flex-col items-center gap-0.5 rounded-2xl py-1.5 transition-all active:scale-95 disabled:opacity-35",
                                class: if sel { "bg-primary text-primary-foreground" } else { "hover:bg-accent/60" },
                                onclick: move |_| onchange.call(d),
                                span { class: if sel { "text-[10px] font-medium opacity-80" } else { "text-[10px] font-medium text-muted-foreground" }, "{WEEKDAYS_SHORT[i as usize]}" }
                                span { class: "text-base font-semibold tabular-nums", "{d.day()}" }
                                span { class: if is_today && !sel { "size-1 rounded-full bg-foreground" } else { "size-1" } }
                            }
                        }
                    }
                }
            }
            button {
                class: "flex size-9 shrink-0 items-center justify-center rounded-full text-muted-foreground active:scale-90 disabled:opacity-30",
                disabled: start + Duration::days(7) > t,
                onclick: move |_| onchange.call((date + Duration::days(7)).min(t)),
                "›"
            }
        }
    }
}

#[component]
fn SummaryCard(d: DayData, g: Goals) -> Element {
    let n = d.totals();
    let budget = g.calories + d.burned_kcal;
    let remaining = budget - n.kcal;
    let over = remaining < 0.0;
    let progress = if budget > 0.0 { n.kcal / budget } else { 0.0 };
    rsx! {
        Card { class: "p-5",
            div { class: "flex items-center justify-between gap-2",
                div { class: "flex w-20 flex-col items-center text-center",
                    span { class: "text-xl font-semibold tabular-nums", "{int(n.kcal)}" }
                    span { class: "text-[11px] text-muted-foreground", "gegessen" }
                }
                Ring { value: progress, size: 156.0, stroke: 13.0,
                    span { class: "text-[2rem] font-bold leading-none tabular-nums tracking-tight", class: if over { "text-bad" }, "{int(remaining.abs())}" }
                    span { class: "mt-1 text-xs text-muted-foreground", if over { "kcal zu viel" } else { "kcal übrig" } }
                }
                div { class: "flex w-20 flex-col items-center text-center",
                    span { class: "text-xl font-semibold tabular-nums", "{int(d.burned_kcal)}" }
                    span { class: "text-[11px] text-muted-foreground", "verbrannt" }
                }
            }
            div { class: "mt-5 grid grid-cols-3 gap-4",
                MacroBar { label: "Protein", value: n.protein, goal: g.protein_g, color: "bg-protein" }
                MacroBar { label: "Kohlenhydrate", value: n.carbs, goal: g.carbs_g, color: "bg-carbs" }
                MacroBar { label: "Fett", value: n.fat, goal: g.fat_g, color: "bg-fat" }
            }
        }
    }
}

#[component]
fn MacroBar(label: &'static str, value: f64, goal: f64, color: &'static str) -> Element {
    let v = if goal > 0.0 { value / goal } else { 0.0 };
    rsx! {
        div { class: "flex min-w-0 flex-col gap-1.5",
            span { class: "truncate text-xs text-muted-foreground", "{label}" }
            Bar { value: v, color }
            span { class: "text-xs tabular-nums",
                span { class: "font-semibold", "{value.round()}" }
                span { class: "text-muted-foreground", " / {goal.round()} g" }
            }
        }
    }
}

#[component]
fn WaterCard(d: DayData, g: Goals, date: NaiveDate, onchange: EventHandler<()>) -> Element {
    let mut app = use_app();
    let glasses = ((g.water_ml / 250.0).round() as usize).clamp(4, 16);
    let filled = (d.water_ml / 250.0).floor() as usize;
    let pct = if g.water_ml > 0.0 { d.water_ml / g.water_ml } else { 0.0 };
    let add = move |ml: f64| {
        let pid = app.pid();
        spawn(async move {
            match add_water(pid, date, ml).await {
                Ok(()) => onchange.call(()),
                Err(e) => app.error(e),
            }
        });
    };
    rsx! {
        Card { class: "p-4",
            div { class: "flex items-center gap-3",
                div { class: "flex size-10 items-center justify-center rounded-xl bg-water/15 text-water", GlassWater { class: "size-5" } }
                div { class: "flex min-w-0 flex-1 flex-col",
                    span { class: "text-[15px] font-semibold", "Wasser" }
                    span { class: "text-xs text-muted-foreground tabular-nums", "{num(d.water_ml / 1000.0, 2)} / {num(g.water_ml / 1000.0, 1)} l · {(pct * 100.0).round()} %" }
                }
                button {
                    class: "flex size-10 items-center justify-center rounded-full bg-muted transition-transform active:scale-90 disabled:opacity-40",
                    disabled: d.water_ml <= 0.0,
                    onclick: move |_| add(-1.0),
                    Minus { class: "size-4" }
                }
                button {
                    class: "flex h-10 items-center gap-1 rounded-full bg-water/20 px-3.5 text-sm font-semibold text-water transition-transform active:scale-90",
                    onclick: move |_| add(250.0),
                    Plus { class: "size-4" } "250"
                }
            }
            div { class: "mt-3 flex flex-wrap gap-1.5",
                for i in 0..glasses.max(filled) {
                    button {
                        key: "{i}",
                        class: "h-8 flex-1 basis-[calc(12.5%-0.4rem)] rounded-lg transition-colors active:scale-90",
                        class: if i < filled { "bg-water" } else { "bg-muted" },
                        "aria-label": "Glas {i + 1}",
                        onclick: move |_| { if i >= filled { add(250.0 * (i + 1 - filled) as f64) } },
                    }
                }
            }
        }
    }
}

#[component]
fn MealCard(meal: &'static str, label: &'static str, entries: Vec<FoodEntry>, date: NaiveDate, onedit: EventHandler<FoodEntry>) -> Element {
    let mut app = use_app();
    let kcal: f64 = entries.iter().map(|e| e.kcal).sum();
    let p: f64 = entries.iter().map(|e| e.protein).sum();
    let c: f64 = entries.iter().map(|e| e.carbs).sum();
    let f: f64 = entries.iter().map(|e| e.fat).sum();
    let emoji = match meal {
        "breakfast" => "🥐",
        "lunch" => "🥗",
        "dinner" => "🍝",
        _ => "🍎",
    };
    let empty = entries.is_empty();
    rsx! {
        Card { class: "overflow-hidden",
            div { class: "flex items-center gap-3 px-4 py-3",
                span { class: "flex size-10 shrink-0 items-center justify-center rounded-xl bg-muted text-xl", "{emoji}" }
                div { class: "flex min-w-0 flex-1 flex-col",
                    span { class: "text-[15px] font-semibold", "{label}" }
                    if empty {
                        span { class: "text-xs text-muted-foreground", "Noch nichts eingetragen" }
                    } else {
                        span { class: "truncate text-xs text-muted-foreground tabular-nums", "{kcal.round()} kcal · P {p.round()} · K {c.round()} · F {f.round()}" }
                    }
                }
                button {
                    "aria-label": "{label} hinzufügen",
                    class: "flex size-10 shrink-0 items-center justify-center rounded-full bg-primary text-primary-foreground transition-transform active:scale-90",
                    onclick: move |_| app.open_food(date, meal),
                    Plus { class: "size-5" }
                }
            }
            if !empty {
                div { class: "border-t",
                    for e in entries.clone() {
                        {
                            let ec = e.clone();
                            let amount = if e.amount > 0.0 { format!("{} {}", num(e.amount, 1), e.unit) } else { "Schnelleintrag".to_string() };
                            rsx! {
                                button {
                                    key: "{e.id}",
                                    class: "flex w-full items-center gap-3 px-4 py-2.5 text-left transition-colors active:bg-accent hover:bg-accent/40",
                                    onclick: move |_| onedit.call(ec.clone()),
                                    span { class: "flex min-w-0 flex-1 flex-col",
                                        span { class: "truncate text-sm font-medium", "{e.name}" }
                                        span { class: "truncate text-xs text-muted-foreground",
                                            "{amount}"
                                            if let Some(b) = e.brand.as_ref().filter(|b| !b.is_empty()) { " · {b}" }
                                        }
                                    }
                                    span { class: "shrink-0 text-sm tabular-nums", "{e.kcal.round()}" }
                                }
                            }
                        }
                    }
                }
            } else {
                div { class: "flex border-t",
                    button {
                        class: "flex flex-1 items-center justify-center gap-1.5 py-2.5 text-xs text-muted-foreground active:bg-accent hover:text-foreground",
                        onclick: move |_| {
                            let pid = app.pid();
                            spawn(async move {
                                match copy_day(pid, date - Duration::days(1), date, Some(meal.to_string())).await {
                                    Ok(0) => app.toast("Gestern war hier nichts eingetragen"),
                                    Ok(n) => { app.bump(); app.toast(format!("{n} Einträge von gestern übernommen")); }
                                    Err(e) => app.error(e),
                                }
                            });
                        },
                        Copy { class: "size-3.5" } "Wie gestern"
                    }
                }
            }
        }
    }
}

#[component]
fn ActivityGrid(d: DayData, g: Goals) -> Element {
    let navigator = use_navigator();
    let fasting = d.fasting_since.map(|s| (crate::util::now() - s).num_minutes());
    let mood = d.mood.map(|m| ["😞", "🙁", "😐", "🙂", "😄"][(m.clamp(1, 5) - 1) as usize]);
    rsx! {
        div { class: "grid grid-cols-2 gap-3",
            Tile {
                icon: rsx! { Dumbbell { class: "size-4" } }, tint: "bg-protein/15 text-protein",
                label: "Training",
                value: if d.exercise_min > 0.0 { fmt_duration_min(d.exercise_min.round() as i64) } else { "—".to_string() },
                sub: if d.exercise_count > 0 { format!("{} kcal · Ziel {} min", d.burned_kcal.round(), g.exercise_min) } else { format!("Ziel {} min", g.exercise_min) },
                progress: if g.exercise_min > 0 { Some(d.exercise_min / g.exercise_min as f64) } else { None },
                bar: "bg-protein",
                onclick: move |_| { navigator.push(Route::Training {}); },
            }
            Tile {
                icon: rsx! { Footprints { class: "size-4" } }, tint: "bg-kcal/15 text-kcal",
                label: "Schritte",
                value: d.steps.map(|s| int(s as f64)).unwrap_or_else(|| "—".into()),
                sub: format!("Ziel {}", int(g.steps as f64)),
                progress: d.steps.map(|s| s as f64 / g.steps.max(1) as f64),
                bar: "bg-kcal",
                onclick: move |_| { navigator.push(Route::Body {}); },
            }
            Tile {
                icon: rsx! { Moon { class: "size-4" } }, tint: "bg-water/15 text-water",
                label: "Schlaf",
                value: d.sleep_hours.map(|h| format!("{} h", num(h, 1))).unwrap_or_else(|| "—".into()),
                sub: format!("Ziel {} h", num(g.sleep_hours, 1)),
                progress: d.sleep_hours.map(|h| h / g.sleep_hours.max(1.0)),
                bar: "bg-water",
                onclick: move |_| { navigator.push(Route::Body {}); },
            }
            Tile {
                icon: rsx! { Scale { class: "size-4" } }, tint: "bg-carbs/15 text-carbs",
                label: "Gewicht",
                value: d.weight.map(|w| format!("{} kg", num(w, 1))).unwrap_or_else(|| "—".into()),
                sub: g.target_weight.map(|t| format!("Ziel {} kg", num(t, 1))).unwrap_or_else(|| "Antippen zum Erfassen".into()),
                onclick: move |_| { navigator.push(Route::Body {}); },
            }
            Tile {
                icon: rsx! { Timer { class: "size-4" } }, tint: "bg-fat/15 text-fat",
                label: "Fasten",
                value: fasting.map(|m| fmt_duration_min(m)).unwrap_or_else(|| "—".into()),
                sub: match (fasting, d.fasting_target) {
                    (Some(_), Some(t)) => format!("Ziel {} h", num(t, 0)),
                    _ => "Nicht aktiv".to_string(),
                },
                progress: match (fasting, d.fasting_target) { (Some(m), Some(t)) if t > 0.0 => Some(m as f64 / 60.0 / t), _ => None },
                bar: "bg-fat",
                onclick: move |_| { navigator.push(Route::Fasting {}); },
            }
            Tile {
                icon: rsx! { Smile { class: "size-4" } }, tint: "bg-muted text-foreground",
                label: "Stimmung",
                value: mood.map(String::from).unwrap_or_else(|| "—".into()),
                sub: "Check-in".to_string(),
                onclick: move |_| { navigator.push(Route::Body {}); },
            }
        }
    }
}

#[component]
fn Tile(
    icon: Element,
    tint: &'static str,
    label: &'static str,
    value: String,
    sub: String,
    #[props(default)] progress: Option<f64>,
    #[props(default = "bg-kcal")] bar: &'static str,
    onclick: EventHandler<MouseEvent>,
) -> Element {
    rsx! {
        Card { class: "flex flex-col gap-2 p-3.5", onclick,
            div { class: "flex items-center gap-2",
                span { class: "flex size-7 items-center justify-center rounded-lg {tint}", {icon} }
                span { class: "flex-1 text-xs font-medium text-muted-foreground", "{label}" }
                ChevronRight { class: "size-4 text-muted-foreground/60" }
            }
            span { class: "truncate text-xl font-semibold tabular-nums", "{value}" }
            if let Some(p) = progress {
                Bar { value: p, color: bar, height: "h-1" }
            }
            span { class: "truncate text-[11px] text-muted-foreground", "{sub}" }
        }
    }
}

#[component]
fn NutrientDetails(d: DayData, g: Goals) -> Element {
    let mut open = use_signal(|| false);
    let n = d.totals();
    rsx! {
        Card { class: "overflow-hidden",
            button { class: "flex w-full items-center gap-2 px-4 py-3 text-left", onclick: move |_| open.toggle(),
                span { class: "flex-1 text-sm font-semibold", "Nährstoffe" }
                span { class: "text-xs text-muted-foreground", if open() { "weniger" } else { "Details" } }
            }
            if open() {
                div { class: "flex flex-col gap-3 border-t px-4 py-4",
                    Row { label: "Ballaststoffe", value: n.fiber, goal: Some(g.fiber_g), unit: "g", color: "bg-kcal" }
                    Row { label: "Zucker", value: n.sugar, goal: Some(g.sugar_g), unit: "g", color: "bg-carbs", limit: true }
                    Row { label: "Natrium", value: n.sodium_mg, goal: Some(2300.0), unit: "mg", color: "bg-fat", limit: true }
                    div { class: "grid grid-cols-3 gap-2 pt-1 text-center text-xs text-muted-foreground",
                        {
                            let t = n.protein * 4.0 + n.carbs * 4.0 + n.fat * 9.0;
                            let s = |v: f64| if t > 0.0 { (v / t * 100.0).round() } else { 0.0 };
                            rsx! {
                                span { "Protein {s(n.protein * 4.0)} %" }
                                span { "Kohlenh. {s(n.carbs * 4.0)} %" }
                                span { "Fett {s(n.fat * 9.0)} %" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn Row(label: &'static str, value: f64, goal: Option<f64>, unit: &'static str, color: &'static str, #[props(default = false)] limit: bool) -> Element {
    let g = goal.unwrap_or(0.0);
    rsx! {
        div { class: "flex flex-col gap-1.5",
            div { class: "flex justify-between text-xs",
                span { "{label}" }
                span { class: "tabular-nums text-muted-foreground",
                    "{num(value, 0)} / {num(g, 0)} {unit}"
                    if limit { " (max.)" }
                }
            }
            Bar { value: if g > 0.0 { value / g } else { 0.0 }, color }
        }
    }
}

use chrono::Duration;
use dioxus::prelude::*;
use icons::{Plus, Scale, TrendingDown, TrendingUp};

use super::checkin_form::CheckinForm;
use super::quick_weight::{QuickWeightSheet, signed};
use super::weight_chart::WeightChart;
use crate::api::body::{body_overview, get_checkin};
use crate::api::profile::get_goals;
use crate::components::layout::page::{PageBody, PageHeader, Section};
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::date_nav::DateNav;
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::ring::Bar;
use crate::components::ui::segmented::Segmented;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{BodyOverview, Checkin, Goals, MOOD_EMOJI, target_progress, weight_change};
use crate::state::use_app;
use crate::util::{bmi, bmi_label, fmt_relative, fmt_short, int, num, today};

#[component]
pub fn BodyPage() -> Element {
    let mut app = use_app();
    let mut quick = use_signal(|| false);
    let mut range = use_signal(|| "90".to_string());

    let overview = use_resource(move || async move {
        let _ = (app.refresh)();
        body_overview(app.pid_tracked()).await.map_err(|e| e.to_string())
    });
    let goals = use_resource(move || async move {
        let _ = (app.refresh)();
        get_goals(app.pid_tracked()).await.map_err(|e| e.to_string())
    });
    let checkin = use_resource(move || async move {
        let _ = (app.refresh)();
        let d = (app.date)();
        get_checkin(app.pid_tracked(), d).await.map(|c| (d, c)).map_err(|e| e.to_string())
    });

    let date = (app.date)();
    let ov: Option<BodyOverview> = overview.read().as_ref().and_then(|r| r.as_ref().ok().cloned());
    let g: Goals = goals.read().as_ref().and_then(|r| r.as_ref().ok().cloned()).unwrap_or_default();
    let last_weight = ov.as_ref().and_then(|o| o.weights.last().map(|w| w.weight));
    let height = app.profile().and_then(|p| p.height_cm);

    let form = match &*checkin.read() {
        Some(Ok((d, c))) if *d == date => {
            let initial = c.clone().unwrap_or_else(|| Checkin::empty(date));
            // Neu aufbauen, wenn sich Tag oder gespeicherte Daten ändern.
            let key = serde_json::to_string(&initial).unwrap_or_default();
            rsx! { CheckinForm { key: "{key}", date, initial, steps_goal: g.steps, sleep_goal: g.sleep_hours } }
        }
        Some(Err(e)) => rsx! { p { class: "text-sm text-bad", "{e}" } },
        _ => rsx! { Skeleton { class: "h-96 rounded-2xl" } },
    };

    let days: i64 = range().parse().unwrap_or(90);
    let to = today();
    let from = to - Duration::days(days - 1);

    rsx! {
        PageHeader {
            title: "Körper",
            subtitle: "Check-in & Gewicht",
            actions: rsx! {
                Button { size: ButtonSize::Sm, class: "h-10 rounded-full px-4", onclick: move |_| quick.set(true),
                    Plus { class: "size-4" }
                    "Gewicht"
                }
            },
        }
        PageBody {
            div { class: "grid gap-4 lg:grid-cols-2 lg:items-start",
                div { class: "flex min-w-0 flex-col gap-4",
                    match ov.as_ref() {
                        Some(o) => rsx! { Overview { o: o.clone(), goal: g.target_weight, height, onquick: move |_| quick.set(true) } },
                        None => match &*overview.read() {
                            Some(Err(e)) => rsx! { p { class: "text-sm text-bad", "{e}" } },
                            _ => rsx! { Skeleton { class: "h-44 rounded-2xl" } },
                        },
                    }
                    Card {
                        CardHeader { title: "Gewichtsverlauf",
                            action: rsx! {
                                Segmented {
                                    class: "w-auto",
                                    options: vec![("30".into(), "30T".into()), ("90".into(), "90T".into()), ("365".into(), "1J".into())],
                                    value: range(),
                                    onchange: move |v| range.set(v),
                                }
                            },
                        }
                        div { class: "px-3 pb-4",
                            WeightChart { weights: ov.as_ref().map(|o| o.weights.clone()).unwrap_or_default(), from, to, goal: g.target_weight }
                        }
                    }
                    div { class: "hidden lg:block", History { ov: ov.clone() } }
                }
                div { id: "checkin", class: "flex min-w-0 flex-col gap-3 scroll-mt-4",
                    div { class: "flex items-center justify-between gap-2 px-1",
                        h2 { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground", "Check-in" }
                        DateNav { date, onchange: move |d| app.date.set(d) }
                    }
                    {form}
                }
                div { class: "lg:hidden", History { ov: ov.clone() } }
            }
        }
        if quick() {
            QuickWeightSheet { date, initial: last_weight, onclose: move |_| quick.set(false) }
        }
    }
}

#[component]
fn Overview(o: BodyOverview, goal: Option<f64>, height: Option<f64>, onquick: EventHandler<()>) -> Element {
    let Some(last) = o.weights.last().copied() else {
        return rsx! {
            Card { class: "flex flex-col items-center gap-3 p-6 text-center",
                div { class: "flex size-12 items-center justify-center rounded-full bg-kcal/15 text-kcal", Scale { class: "size-6" } }
                div {
                    p { class: "font-semibold", "Noch kein Gewicht erfasst" }
                    p { class: "text-sm text-muted-foreground", "Trag dein aktuelles Gewicht ein, um den Verlauf zu sehen." }
                }
                Button { size: ButtonSize::Lg, class: "w-full sm:w-auto", onclick: move |_| onquick.call(()), "Gewicht eintragen" }
            }
        };
    };
    let c7 = weight_change(&o.weights, 7);
    let c30 = weight_change(&o.weights, 30);
    let b = height.and_then(|h| bmi(last.weight, h));
    // Gut = Richtung Ziel (ohne Ziel: abnehmen gilt als neutral).
    let tone = |c: Option<f64>| -> &'static str {
        match (c, goal) {
            (Some(c), _) if c.abs() < 0.05 => "text-muted-foreground",
            (Some(c), Some(g)) => {
                if (g - last.weight).signum() == c.signum() || (g - last.weight).abs() < 0.3 {
                    "text-good"
                } else {
                    "text-warn"
                }
            }
            _ => "text-foreground",
        }
    };
    let change = |c: Option<f64>| c.map(|c| format!("{} kg", signed(c))).unwrap_or_else(|| "–".into());
    let progress = goal.and_then(|g| target_progress(&o.weights, g));
    let start = o.weights.first().map(|w| w.weight).unwrap_or(last.weight);
    rsx! {
        Card { class: "overflow-hidden",
            button {
                r#type: "button",
                class: "flex w-full items-end gap-3 px-4 pt-4 text-left",
                onclick: move |_| onquick.call(()),
                div { class: "min-w-0 flex-1",
                    p { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Aktuelles Gewicht" }
                    p { class: "text-4xl font-bold tabular-nums tracking-tight",
                        "{num(last.weight, 1)}"
                        span { class: "ml-1 text-lg font-medium text-muted-foreground", "kg" }
                    }
                    p { class: "text-xs text-muted-foreground", "{fmt_relative(last.date)}" }
                }
                if let Some(c) = c7.filter(|c| c.abs() >= 0.05) {
                    span { class: "mb-1 flex items-center gap-1 rounded-full bg-muted px-2.5 py-1 text-xs font-medium {tone(Some(c))}",
                        if c < 0.0 { TrendingDown { class: "size-3.5" } } else { TrendingUp { class: "size-3.5" } }
                        "{signed(c)} kg"
                    }
                }
            }
            div { class: "grid grid-cols-3 gap-3 px-4 py-4",
                div { class: "flex min-w-0 flex-col",
                    span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "7 Tage" }
                    span { class: "text-base font-semibold tabular-nums {tone(c7)}", "{change(c7)}" }
                }
                div { class: "flex min-w-0 flex-col",
                    span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "30 Tage" }
                    span { class: "text-base font-semibold tabular-nums {tone(c30)}", "{change(c30)}" }
                }
                div { class: "flex min-w-0 flex-col",
                    span { class: "text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "BMI" }
                    if let Some(b) = b {
                        span { class: "text-base font-semibold tabular-nums", "{num(b, 1)}" }
                        span { class: "truncate text-[11px] text-muted-foreground", "{bmi_label(b)}" }
                    } else {
                        span { class: "text-base font-semibold", "–" }
                        span { class: "truncate text-[11px] text-muted-foreground", "Grösse fehlt" }
                    }
                }
            }
            if let (Some(g), Some(p)) = (goal, progress) {
                div { class: "flex flex-col gap-1.5 border-t px-4 py-3",
                    div { class: "flex items-baseline justify-between text-xs",
                        span { class: "text-muted-foreground", "Start {num(start, 1)} kg" }
                        span { class: "font-medium",
                            if (last.weight - g).abs() < 0.05 { "Ziel erreicht 🎉" } else { "noch {num((last.weight - g).abs(), 1)} kg bis {num(g, 1)} kg" }
                        }
                    }
                    Bar { value: p, color: "bg-good", height: "h-2" }
                }
            }
        }
    }
}

#[component]
fn History(ov: Option<BodyOverview>) -> Element {
    rsx! {
        Section { title: "Letzte Check-ins",
            if let Some(o) = ov.as_ref() {
                if o.recent.is_empty() {
                    Empty {
                        EmptyTitle { "Noch keine Check-ins" }
                        EmptyDescription { "Trage dein Gewicht oder deine Stimmung ein – der Verlauf erscheint hier." }
                    }
                } else {
                    Card { class: "divide-y overflow-hidden",
                        for c in o.recent.iter().take(14).cloned() {
                            HistoryRow { key: "{c.date}", c }
                        }
                    }
                }
            } else {
                Skeleton { class: "h-40 rounded-2xl" }
            }
        }
    }
}

#[component]
fn HistoryRow(c: Checkin) -> Element {
    let mut app = use_app();
    let date = c.date;
    let mood = c.mood.and_then(|m| MOOD_EMOJI.get((m - 1).clamp(0, 4) as usize)).copied();
    rsx! {
        button {
            r#type: "button",
            class: "flex w-full items-center gap-3 px-4 py-3 text-left transition-colors hover:bg-accent/40 active:bg-accent/60",
            onclick: move |_| {
                app.date.set(date);
                let _ = document::eval("document.getElementById('checkin')?.scrollIntoView({behavior:'smooth', block:'start'})");
            },
            div { class: "w-20 shrink-0",
                p { class: "text-sm font-medium", "{fmt_relative(date)}" }
                p { class: "text-[11px] text-muted-foreground", "{fmt_short(date)}" }
            }
            div { class: "flex min-w-0 flex-1 flex-wrap items-center gap-x-3 gap-y-0.5 text-xs text-muted-foreground",
                if let Some(w) = c.weight {
                    span { class: "text-sm font-semibold tabular-nums text-foreground", "{num(w, 1)} kg" }
                }
                if let Some(s) = c.steps {
                    span { class: "tabular-nums", "👣 {int(s as f64)}" }
                }
                if let Some(h) = c.sleep_hours {
                    span { class: "tabular-nums", "🌙 {num(h, 1)} h" }
                }
                if let Some(bf) = c.body_fat {
                    span { class: "tabular-nums", "{num(bf, 1)} % KFA" }
                }
            }
            if let Some(m) = mood {
                span { class: "text-xl", "{m}" }
            }
        }
    }
}

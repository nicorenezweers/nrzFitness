//! Berichte: Zeitraum wählen, Kennzahlen mit Trend, Diagramme je Bereich, Konsistenz-Kalender.

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;
use icons::{ChevronLeft, ChevronRight, Flame};

use super::charts_extra::{HeatLegend, Heatmap, Legend, MultiLine, RankList, StackedBars};
use crate::api::reports::get_report;
use crate::components::charts::{BarChart, Donut, LineChart};
use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::card::{Card, CardHeader, Stat};
use crate::components::ui::ring::Bar;
use crate::components::ui::segmented::Segmented;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{DayRow, REPORT_RANGES, Report};
use crate::state::use_app;
use crate::util::{WEEKDAYS_SHORT, fmt_dm, fmt_short, int, num, today, week_start};

/// Abschnitte (Anker-ID, Beschriftung) für die Sprung-Chips.
const SECTIONS: [(&str, &str); 9] = [
    ("r-uebersicht", "Überblick"),
    ("r-konsistenz", "Konsistenz"),
    ("r-ernaehrung", "Ernährung"),
    ("r-wasser", "Wasser"),
    ("r-koerper", "Körper"),
    ("r-training", "Training"),
    ("r-schlaf", "Schlaf"),
    ("r-schritte", "Schritte"),
    ("r-fasten", "Fasten"),
];

fn range_days(key: &str) -> i64 {
    REPORT_RANGES.iter().find(|r| r.0 == key).map(|r| r.2).unwrap_or(30)
}

fn opts(v: &[(&str, &str)]) -> Vec<(String, String)> {
    v.iter().map(|(a, b)| (a.to_string(), b.to_string())).collect()
}

#[component]
pub fn ReportsPage() -> Element {
    let app = use_app();
    let mut range = use_signal(|| "30".to_string());
    let mut offset = use_signal(|| 0i64);
    let mut active = use_signal(|| SECTIONS[0].0.to_string());
    let period = use_memo(move || {
        let len = range_days(&range());
        let to = today() - Duration::days(len * offset());
        (to - Duration::days(len - 1), to)
    });
    let res = use_resource(move || {
        let _ = (app.refresh)();
        let pid = app.pid_tracked();
        let (from, to) = period();
        async move { get_report(pid, from, to).await }
    });

    let (from, to) = period();
    let subtitle = if from.year() == to.year() {
        format!("{} – {} {}", fmt_short(from), fmt_short(to), to.year())
    } else {
        format!("{} {} – {} {}", fmt_short(from), from.year(), fmt_short(to), to.year())
    };
    let range_opts: Vec<(String, String)> = REPORT_RANGES.iter().map(|r| (r.0.to_string(), r.1.to_string())).collect();

    rsx! {
        PageHeader { title: "Berichte", subtitle }
        // Sticky: Zeitraum + Sprung-Chips (bleibt beim Scrollen unter dem Kopf)
        div { class: "sticky top-0 z-20 -mt-1 flex flex-col gap-2 border-b bg-background/90 px-4 pb-2 pt-2 backdrop-blur-md sm:px-8",
            div { class: "flex items-center gap-2",
                button {
                    r#type: "button",
                    class: "flex size-11 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground transition-transform hover:text-foreground active:scale-95 sm:size-10",
                    "aria-label": "Vorheriger Zeitraum",
                    onclick: move |_| offset += 1,
                    ChevronLeft { class: "size-5" }
                }
                Segmented {
                    class: "h-11 sm:h-10",
                    options: range_opts,
                    value: range(),
                    onchange: move |v| {
                        range.set(v);
                        offset.set(0);
                    },
                }
                button {
                    r#type: "button",
                    class: "flex size-11 shrink-0 items-center justify-center rounded-xl bg-muted text-muted-foreground transition-transform hover:text-foreground active:scale-95 disabled:opacity-40 sm:size-10",
                    "aria-label": "Nächster Zeitraum",
                    disabled: offset() == 0,
                    onclick: move |_| if offset() > 0 { offset -= 1 },
                    ChevronRight { class: "size-5" }
                }
            }
            nav { class: "no-scrollbar -mx-4 flex gap-2 overflow-x-auto px-4 sm:-mx-8 sm:px-8",
                for (id, label) in SECTIONS {
                    button {
                        r#type: "button",
                        key: "{id}",
                        class: if active() == id { "h-9 shrink-0 rounded-full bg-primary px-4 text-sm font-medium text-primary-foreground" } else { "h-9 shrink-0 rounded-full border px-4 text-sm text-muted-foreground hover:text-foreground" },
                        onclick: move |_| {
                            active.set(id.to_string());
                            document::eval(&format!(
                                "document.getElementById('{id}')?.scrollIntoView({{behavior: 'smooth', block: 'start'}});"
                            ));
                        },
                        "{label}"
                    }
                }
            }
        }
        PageBody { class: "pt-4",
            match &*res.read() {
                Some(Ok(r)) => rsx! { ReportView { report: r.clone() } },
                Some(Err(e)) => rsx! {
                    Card { class: "p-4 text-sm text-bad", "Bericht konnte nicht geladen werden: {e}" }
                },
                None => rsx! { ReportSkeleton {} },
            }
        }
    }
}

#[component]
fn ReportSkeleton() -> Element {
    rsx! {
        div { class: "grid grid-cols-2 gap-3 sm:grid-cols-3",
            for i in 0..6 {
                Skeleton { key: "{i}", class: "h-28 rounded-2xl" }
            }
        }
        div { class: "grid gap-4 lg:grid-cols-2",
            for i in 0..4 {
                Skeleton { key: "{i}", class: "h-64 rounded-2xl" }
            }
        }
    }
}

// ---------------------------------------------------------------------------------------------
// Zeitreihen: Tage bzw. Wochen (ab 1 Jahr)
// ---------------------------------------------------------------------------------------------

struct Buckets<'a> {
    labels: Vec<String>,
    groups: Vec<&'a [DayRow]>,
    weekly: bool,
}

impl<'a> Buckets<'a> {
    fn new(days: &'a [DayRow]) -> Self {
        let weekly = days.len() > 120;
        let mut labels = Vec::new();
        let mut groups = Vec::new();
        if weekly {
            let mut start = 0;
            for i in 1..=days.len() {
                if i == days.len() || week_start(days[i].date) != week_start(days[start].date) {
                    labels.push(fmt_dm(week_start(days[start].date)));
                    groups.push(&days[start..i]);
                    start = i;
                }
            }
        } else {
            for i in 0..days.len() {
                let d = days[i].date;
                labels.push(if days.len() <= 7 {
                    WEEKDAYS_SHORT[d.weekday().num_days_from_monday() as usize].to_string()
                } else {
                    fmt_dm(d)
                });
                groups.push(&days[i..=i]);
            }
        }
        Self { labels, groups, weekly }
    }

    /// Mittelwert der vorhandenen Werte je Gruppe.
    fn avg(&self, f: impl Fn(&DayRow) -> Option<f64>) -> Vec<Option<f64>> {
        self.groups
            .iter()
            .map(|g| {
                let v: Vec<f64> = g.iter().filter_map(&f).collect();
                (!v.is_empty()).then(|| v.iter().sum::<f64>() / v.len() as f64)
            })
            .collect()
    }

    fn sum(&self, f: impl Fn(&DayRow) -> f64) -> Vec<f64> {
        self.groups.iter().map(|g| g.iter().map(&f).sum()).collect()
    }

    fn bars(&self, v: Vec<Option<f64>>) -> Vec<(String, f64)> {
        self.labels.iter().cloned().zip(v.into_iter().map(|x| x.unwrap_or(0.0))).collect()
    }

    fn bars_f(&self, v: Vec<f64>) -> Vec<(String, f64)> {
        self.labels.iter().cloned().zip(v).collect()
    }

    fn line(&self, v: Vec<Option<f64>>) -> Vec<(String, Option<f64>)> {
        self.labels.iter().cloned().zip(v).collect()
    }

    fn per(&self) -> &'static str {
        if self.weekly { "pro Woche" } else { "pro Tag" }
    }

    fn avg_per(&self) -> &'static str {
        if self.weekly { "Ø pro Tag, nach Wochen" } else { "pro Tag" }
    }
}

fn opt_num(v: Option<f64>, dec: usize) -> String {
    v.map(|v| num(v, dec)).unwrap_or_else(|| "–".into())
}

fn opt_int(v: Option<f64>) -> String {
    v.map(int).unwrap_or_else(|| "–".into())
}

fn signed(v: f64, dec: usize) -> String {
    if v > 0.0 { format!("+{}", num(v, dec)) } else { num(v, dec).replace('-', "−") }
}

// ---------------------------------------------------------------------------------------------
// Ansicht
// ---------------------------------------------------------------------------------------------

#[component]
fn ReportView(report: Report) -> Element {
    let r = &report;
    let s = &r.summary;
    let p = &r.prev;
    let g = &r.goals;
    let b = Buckets::new(&r.days);

    // Wochenziel bei Wochen-Summen
    let ex_goal = if b.weekly { g.exercise_min as f64 * 7.0 } else { g.exercise_min as f64 };

    let kcal_bars = b.bars(b.avg(|d| (d.food_count > 0).then_some(d.kcal)));
    let macro_bars: Vec<(String, Vec<f64>)> = {
        let pr = b.avg(|d| (d.food_count > 0).then_some(d.protein));
        let ca = b.avg(|d| (d.food_count > 0).then_some(d.carbs));
        let fa = b.avg(|d| (d.food_count > 0).then_some(d.fat));
        b.labels
            .iter()
            .enumerate()
            .map(|(i, l)| (l.clone(), vec![pr[i].unwrap_or(0.0), ca[i].unwrap_or(0.0), fa[i].unwrap_or(0.0)]))
            .collect()
    };
    let water_bars = b.bars(b.avg(|d| (d.water_ml > 0.0).then_some(d.water_ml)));
    let water_goal_days = r.days.iter().filter(|d| g.water_ml > 0.0 && d.water_ml >= g.water_ml).count();
    let weight_line = b.line(b.avg(|d| d.weight));
    let fat_line = b.line(b.avg(|d| d.body_fat));
    let waist_line = b.line(b.avg(|d| d.waist));
    let ex_min_bars = b.bars_f(b.sum(|d| d.exercise_min));
    let ex_kcal_bars = b.bars_f(b.sum(|d| d.burned_kcal));
    let active_days = r.days.iter().filter(|d| d.workouts > 0).count();
    let sleep_line = b.line(b.avg(|d| d.sleep_h));
    let mood = b.avg(|d| d.mood);
    let energy = b.avg(|d| d.energy);
    let steps_bars = b.bars(b.avg(|d| d.steps.map(|s| s as f64)));
    let fast_bars = b.bars(b.avg(|d| d.fast_hours));

    // Makro-Verteilung (Anteil an kcal)
    let (pk, ck, fk) =
        (s.avg_protein.unwrap_or(0.0) * 4.0, s.avg_carbs.unwrap_or(0.0) * 4.0, s.avg_fat.unwrap_or(0.0) * 9.0);
    let mk = (pk + ck + fk).max(1e-9);
    let macro_rows = [
        ("bg-protein", "Eiweiss", s.avg_protein, pk / mk, g.protein_g),
        ("bg-carbs", "Kohlenhydrate", s.avg_carbs, ck / mk, g.carbs_g),
        ("bg-fat", "Fett", s.avg_fat, fk / mk, g.fat_g),
    ];

    let logged_days = r.activity.iter().filter(|a| a.date >= r.from && a.date <= r.to).count();
    let heat: Vec<(NaiveDate, u8)> = r.activity.iter().map(|a| (a.date, a.level)).collect();

    let weight_goal_dir = if g.weekly_change < 0.0 {
        Some(false)
    } else if g.weekly_change > 0.0 {
        Some(true)
    } else {
        None
    };

    let top_foods_count: Vec<(String, f64, String)> = r
        .top_foods_count
        .iter()
        .map(|t| (t.name.clone(), t.count as f64, format!("{}× · {} kcal", t.count, int(t.kcal))))
        .collect();
    let top_foods_kcal: Vec<(String, f64, String)> = r
        .top_foods_kcal
        .iter()
        .map(|t| (t.name.clone(), t.kcal, format!("{} kcal · {}×", int(t.kcal), t.count)))
        .collect();
    let top_ex: Vec<(String, f64, String)> = r
        .top_exercises
        .iter()
        .map(|t| {
            (t.name.clone(), t.count as f64, format!("{}× · {} min · {} kcal", t.count, int(t.minutes), int(t.kcal)))
        })
        .collect();

    rsx! {
           // Überblick
           div { id: "r-uebersicht", class: "grid scroll-mt-32 grid-cols-2 gap-3 sm:grid-cols-3",
               Tile {
                   label: "Ø Kalorien",
                   value: opt_int(s.avg_kcal),
                   unit: "kcal",
                   sub: format!("Ziel {}", int(g.calories)),
                   progress: s.avg_kcal.map(|v| v / g.calories.max(1.0)),
                   color: "bg-kcal",
                   trend: rsx! { Trend { cur: s.avg_kcal, prev: p.avg_kcal, target: g.calories, unit: "kcal" } },
               }
               Tile {
                   label: "Ø Eiweiss",
                   value: opt_int(s.avg_protein),
                   unit: "g",
                   sub: format!("Ziel {} g", int(g.protein_g)),
                   progress: s.avg_protein.map(|v| v / g.protein_g.max(1.0)),
                   color: "bg-protein",
                   trend: rsx! { Trend { cur: s.avg_protein, prev: p.avg_protein, good_up: true, unit: "g" } },
               }
               Tile {
                   label: "Ø Wasser",
                   value: opt_num(s.avg_water.map(|v| v / 1000.0), 1),
                   unit: "l",
                   sub: format!("Ziel {} l", num(g.water_ml / 1000.0, 1)),
                   progress: s.avg_water.map(|v| v / g.water_ml.max(1.0)),
                   color: "bg-water",
                   trend: rsx! { Trend { cur: s.avg_water.map(|v| v / 1000.0), prev: p.avg_water.map(|v| v / 1000.0), good_up: true, decimals: 1, unit: "l" } },
               }
               Tile {
                   label: "Gewicht",
                   value: s.weight_change().map(|v| signed(v, 1)).unwrap_or_else(|| "–".into()),
                   unit: "kg",
                   sub: match s.weight_last {
                       Some(w) => format!("aktuell {} kg", num(w, 1)),
                       None => "keine Messung".into(),
                   },
                   trend: rsx! {
                       Trend { cur: s.weight_last, prev: p.weight_last, target: g.target_weight, good_up: if g.target_weight.is_none() { weight_goal_dir } else { None }, decimals: 1, unit: "kg" }
                   },
               }
               Tile {
                   label: "Trainings",
                   value: s.workouts.to_string(),
                   unit: "×",
                   sub: format!("{} · {} kcal", crate::util::fmt_duration_min(s.exercise_min.round() as i64), int(s.burned_kcal)),
                   trend: rsx! { Trend { cur: Some(s.workouts as f64), prev: Some(p.workouts as f64), good_up: true } },
               }
               Tile {
                   label: "Ø Schlaf",
                   value: opt_num(s.avg_sleep, 1),
                   unit: "h",
                   sub: format!("Ziel {} h", num(g.sleep_hours, 1)),
                   progress: s.avg_sleep.map(|v| v / g.sleep_hours.max(1.0)),
                   color: "bg-protein",
                   trend: rsx! { Trend { cur: s.avg_sleep, prev: p.avg_sleep, target: g.sleep_hours, decimals: 1, unit: "h" } },
               }
           }

           div { class: "grid gap-4 lg:grid-cols-2",
               // Konsistenz
               Anchor { id: "r-konsistenz",
    Card { class: "h-full",
                   CardHeader { title: "Konsistenz", subtitle: "Tage, an denen du etwas erfasst hast" }
                   div { class: "flex flex-col gap-4 px-4 pb-4",
                       div { class: "flex items-center gap-4",
                           div { class: "flex size-14 shrink-0 items-center justify-center rounded-2xl bg-muted text-carbs",
                               Flame { class: "size-7" }
                           }
                           div { class: "min-w-0 flex-1",
                               p { class: "text-2xl font-bold tabular-nums leading-tight",
                                   "{r.streak}"
                                   span { class: "ml-1 text-sm font-normal text-muted-foreground", if r.streak == 1 { "Tag in Folge" } else { "Tage in Folge" } }
                               }
                               p { class: "text-xs text-muted-foreground", "Rekord {r.best_streak} Tage · {logged_days} von {s.days} Tagen erfasst" }
                           }
                       }
                       Heatmap { from: r.heat_from, to: r.to, levels: heat }
                       HeatLegend {}
                   }
               }
    }

               // Ernährung: Kalorien
               Anchor { id: "r-ernaehrung",
    Card { class: "h-full",
                   CardHeader { title: "Kalorien", subtitle: format!("Aufgenommen {} · Ziellinie {} kcal", b.avg_per(), int(g.calories)) }
                   div { class: "flex flex-col gap-4 px-4 pb-4",
                       BarChart { bars: kcal_bars, goal: g.calories, color: "text-kcal", over_color: "text-warn" }
                       div { class: "grid grid-cols-2 gap-3 sm:grid-cols-4",
                           Stat { label: "Ø kcal", value: opt_int(s.avg_kcal) }
                           Stat { label: "Erfasst", value: format!("{}/{}", s.food_days, s.days), unit: "Tage" }
                           Stat { label: "Ø Ballastst.", value: opt_num(s.avg_fiber, 0), unit: "g" }
                           Stat { label: "Ø Zucker", value: opt_num(s.avg_sugar, 0), unit: "g" }
                       }
                   }
               }
    }

               // Ernährung: Makros
               Card {
                   CardHeader { title: "Makronährstoffe", subtitle: "Verteilung nach Kalorien" }
                   div { class: "flex flex-col gap-4 px-4 pb-4",
                       div { class: "flex items-center gap-4",
                           Donut { parts: vec![("text-protein".to_string(), pk), ("text-carbs".to_string(), ck), ("text-fat".to_string(), fk)], size: 104.0, stroke: 14.0,
                               span { class: "text-lg font-semibold tabular-nums leading-none", "{opt_int(s.avg_kcal)}" }
                               span { class: "text-[10px] text-muted-foreground", "Ø kcal" }
                           }
                           div { class: "flex min-w-0 flex-1 flex-col gap-2.5",
                               for (bg, label, grams, share, goal) in macro_rows {
                                   div { key: "{label}", class: "flex flex-col gap-1",
                                       div { class: "flex items-baseline gap-2 text-sm",
                                           span { class: "size-2.5 shrink-0 rounded-full {bg}" }
                                           span { class: "min-w-0 flex-1 truncate", "{label}" }
                                           span { class: "shrink-0 tabular-nums font-medium", "{(share * 100.0).round()}%" }
                                       }
                                       p { class: "pl-4.5 text-xs tabular-nums text-muted-foreground", "Ø {opt_num(grams, 0)} g von {int(goal)} g" }
                                   }
                               }
                           }
                       }
                       div { class: "flex flex-col gap-2",
                           p { class: "text-xs font-medium text-muted-foreground", "Verlauf in g ({b.avg_per()})" }
                           StackedBars { bars: macro_bars, colors: vec!["text-protein".to_string(), "text-carbs".to_string(), "text-fat".to_string()] }
                           Legend { items: vec![("bg-protein".to_string(), "Eiweiss".to_string()), ("bg-carbs".to_string(), "Kohlenhydrate".to_string()), ("bg-fat".to_string(), "Fett".to_string())] }
                       }
                   }
               }

               // Ernährung: Top-Lebensmittel
               TopFoodsCard { by_count: top_foods_count, by_kcal: top_foods_kcal }

               // Wasser
               Anchor { id: "r-wasser",
    Card { class: "h-full",
                   CardHeader { title: "Wasser", subtitle: format!("ml {} · Ziel {} ml", b.avg_per(), int(g.water_ml)) }
                   div { class: "flex flex-col gap-4 px-4 pb-4",
                       BarChart { bars: water_bars, goal: g.water_ml, color: "text-water" }
                       div { class: "grid grid-cols-3 gap-3",
                           Stat { label: "Ø Menge", value: opt_num(s.avg_water.map(|v| v / 1000.0), 2), unit: "l" }
                           Stat { label: "Ziel erreicht", value: format!("{water_goal_days}/{}", s.days), unit: "Tage" }
                           Stat { label: "Erfasst", value: s.water_days.to_string(), unit: "Tage" }
                       }
                   }
               }
    }

               // Gewicht & Körper
               BodyCard {
                   weight: weight_line,
                   body_fat: fat_line,
                   waist: waist_line,
                   target: g.target_weight,
                   first: s.weight_first,
                   last: s.weight_last,
                   min: s.weight_min,
                   max: s.weight_max,
                   avg_fat: s.avg_body_fat,
                   good_up: weight_goal_dir,
               }

               // Training
               TrainingCard {
                   minutes: ex_min_bars,
                   kcal: ex_kcal_bars,
                   goal: ex_goal,
                   per: b.per(),
                   workouts: s.workouts,
                   total_min: s.exercise_min,
                   total_kcal: s.burned_kcal,
                   active_days,
                   days: s.days,
                   top: top_ex,
               }

               // Schlaf & Stimmung
               Anchor { id: "r-schlaf",
    Card { class: "h-full",
                   CardHeader { title: "Schlaf & Stimmung", subtitle: format!("Schlaf in h · Ziel {} h", num(g.sleep_hours, 1)) }
                   div { class: "flex flex-col gap-4 px-4 pb-4",
                       LineChart { points: sleep_line, goal: g.sleep_hours, color: "text-protein", decimals: 1 }
                       div { class: "flex flex-col gap-2",
                           p { class: "text-xs font-medium text-muted-foreground", "Stimmung & Energie (1–5)" }
                           MultiLine { labels: b.labels.clone(), series: vec![("text-carbs".to_string(), mood), ("text-kcal".to_string(), energy)], min: 1.0, max: 5.0 }
                           Legend { items: vec![("bg-carbs".to_string(), "Stimmung".to_string()), ("bg-kcal".to_string(), "Energie".to_string())] }
                       }
                       div { class: "grid grid-cols-2 gap-3 sm:grid-cols-4",
                           Stat { label: "Ø Schlaf", value: opt_num(s.avg_sleep, 1), unit: "h" }
                           Stat { label: "Ø Qualität", value: opt_num(s.avg_sleep_quality, 1), unit: "/5" }
                           Stat { label: "Ø Stimmung", value: opt_num(s.avg_mood, 1), unit: "/5" }
                           Stat { label: "Ø Energie", value: opt_num(s.avg_energy, 1), unit: "/5" }
                       }
                   }
               }
    }

               // Schritte
               Anchor { id: "r-schritte",
    Card { class: "h-full",
                   CardHeader { title: "Schritte", subtitle: format!("{} · Ziel {}", b.avg_per(), int(g.steps as f64)) }
                   div { class: "flex flex-col gap-4 px-4 pb-4",
                       BarChart { bars: steps_bars, goal: g.steps as f64, color: "text-kcal" }
                       div { class: "grid grid-cols-3 gap-3",
                           Stat { label: "Ø Schritte", value: opt_int(s.avg_steps) }
                           Stat { label: "Ziel erreicht", value: format!("{}/{}", s.steps_goal_days, s.step_days), unit: "Tage" }
                           Stat { label: "Trend", value: trend_text(s.avg_steps, p.avg_steps, 0) }
                       }
                   }
               }
    }

               // Fasten
               Anchor { id: "r-fasten",
    Card { class: "h-full",
                   CardHeader { title: "Fasten", subtitle: format!("Beendete Fasten in h · Ziel {} h", num(g.fasting_hours, 0)) }
                   div { class: "flex flex-col gap-4 px-4 pb-4",
                       div { class: "grid grid-cols-2 gap-3 sm:grid-cols-4",
                           Stat { label: "Fasten", value: s.fasts.to_string() }
                           Stat { label: "Ziel erreicht", value: format!("{}/{}", s.fasts_reached, s.fasts) }
                           Stat { label: "Ø Dauer", value: opt_num(s.avg_fast_h, 1), unit: "h" }
                           Stat { label: "Längstes", value: opt_num(s.longest_fast_h, 1), unit: "h" }
                       }
                       BarChart { bars: fast_bars, goal: g.fasting_hours, color: "text-carbs", decimals: 0 }
                   }
               }
    }
           }
       }
}

fn trend_text(cur: Option<f64>, prev: Option<f64>, dec: usize) -> String {
    match (cur, prev) {
        (Some(c), Some(p)) if (c - p).abs() > 1e-9 => {
            format!(
                "{} {}",
                if c > p { "▲" } else { "▼" },
                if dec == 0 { int((c - p).abs()) } else { num((c - p).abs(), dec) }
            )
        }
        (Some(_), Some(_)) => "=".into(),
        _ => "–".into(),
    }
}

/// Überblick-Kachel.
#[component]
fn Tile(
    #[props(into)] label: String,
    #[props(into)] value: String,
    #[props(into)] unit: String,
    #[props(into)] sub: String,
    #[props(default)] progress: Option<f64>,
    #[props(into, default)] color: String,
    trend: Element,
) -> Element {
    rsx! {
        Card { class: "flex min-w-0 flex-col gap-1 p-3.5",
            span { class: "truncate text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "{label}" }
            p { class: "truncate text-2xl font-bold tabular-nums leading-tight",
                "{value}"
                span { class: "ml-1 text-xs font-normal text-muted-foreground", "{unit}" }
            }
            if let Some(v) = progress {
                Bar { value: v, color: color.clone() }
            }
            span { class: "truncate text-xs text-muted-foreground", "{sub}" }
            {trend}
        }
    }
}

/// Veränderung gegenüber dem vorherigen Zeitraum (▲/▼). Mit `target` gilt „näher am Ziel“ als gut,
/// sonst bestimmt `good_up`, ob steigend gut ist (None = neutral).
#[component]
fn Trend(
    cur: Option<f64>,
    prev: Option<f64>,
    #[props(default)] target: Option<f64>,
    #[props(default)] good_up: Option<bool>,
    #[props(default = 0)] decimals: usize,
    #[props(into, default)] unit: String,
) -> Element {
    let (Some(c), Some(p)) = (cur, prev) else {
        return rsx! { span { class: "truncate text-[11px] text-muted-foreground", "kein Vergleich" } };
    };
    let d = c - p;
    let shown = if decimals == 0 { int(d.abs()) } else { num(d.abs(), decimals) };
    if shown == "0" || d.abs() < 1e-9 {
        return rsx! { span { class: "truncate text-[11px] text-muted-foreground", "= wie zuvor" } };
    }
    let good = match (target, good_up) {
        (Some(t), _) => Some((c - t).abs() < (p - t).abs()),
        (None, Some(up)) => Some((d > 0.0) == up),
        _ => None,
    };
    let cls = match good {
        Some(true) => "text-good",
        Some(false) => "text-bad",
        None => "text-muted-foreground",
    };
    let arrow = if d > 0.0 { "▲" } else { "▼" };
    rsx! {
        span { class: "truncate text-[11px] font-medium tabular-nums {cls}",
            "{arrow} {shown}"
            if !unit.is_empty() { " {unit}" }
            span { class: "font-normal text-muted-foreground", " vs. vorher" }
        }
    }
}

#[component]
fn TopFoodsCard(by_count: Vec<(String, f64, String)>, by_kcal: Vec<(String, f64, String)>) -> Element {
    let mut mode = use_signal(|| "count".to_string());
    let rows = if mode() == "count" { by_count } else { by_kcal };
    rsx! {
        Card {
            CardHeader { title: "Top-Lebensmittel", subtitle: "Die 10 häufigsten bzw. kalorienreichsten" }
            div { class: "flex flex-col gap-4 px-4 pb-4",
                Segmented { options: opts(&[("count", "Häufigkeit"), ("kcal", "Kalorien")]), value: mode(), onchange: move |v| mode.set(v) }
                RankList { rows, color: if mode() == "count" { "bg-kcal" } else { "bg-carbs" } }
            }
        }
    }
}

#[component]
fn BodyCard(
    weight: Vec<(String, Option<f64>)>,
    body_fat: Vec<(String, Option<f64>)>,
    waist: Vec<(String, Option<f64>)>,
    target: Option<f64>,
    first: Option<f64>,
    last: Option<f64>,
    min: Option<f64>,
    max: Option<f64>,
    avg_fat: Option<f64>,
    good_up: Option<bool>,
) -> Element {
    let mut metric = use_signal(|| "weight".to_string());
    let change = match (first, last) {
        (Some(a), Some(b)) => Some(b - a),
        _ => None,
    };
    let change_cls = match (change, good_up) {
        (Some(c), Some(up)) if c.abs() > 0.05 => {
            if (c > 0.0) == up {
                "text-good"
            } else {
                "text-bad"
            }
        }
        _ => "",
    };
    rsx! {
           Anchor { id: "r-koerper",
    Card { class: "h-full",
               CardHeader { title: "Gewicht & Körper" }
               div { class: "flex flex-col gap-4 px-4 pb-4",
                   Segmented {
                       options: opts(&[("weight", "Gewicht"), ("fat", "Körperfett"), ("waist", "Taille")]),
                       value: metric(),
                       onchange: move |v| metric.set(v),
                   }
                   match metric().as_str() {
                       "fat" => rsx! { LineChart { points: body_fat.clone(), color: "text-fat", decimals: 1 } },
                       "waist" => rsx! { LineChart { points: waist.clone(), color: "text-carbs", decimals: 0 } },
                       _ => rsx! { LineChart { points: weight.clone(), goal: target, color: "text-kcal", decimals: 1 } },
                   }
                   div { class: "grid grid-cols-2 gap-3 sm:grid-cols-4",
                       Stat { label: "Aktuell", value: opt_num(last, 1), unit: "kg" }
                       div { class: "flex min-w-0 flex-col",
                           span { class: "truncate text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "Veränderung" }
                           span { class: "truncate text-lg font-semibold tabular-nums {change_cls}",
                               {change.map(|c| signed(c, 1)).unwrap_or_else(|| "–".into())}
                               span { class: "ml-0.5 text-xs font-normal text-muted-foreground", "kg" }
                           }
                       }
                       Stat { label: "Min – Max", value: match (min, max) { (Some(a), Some(b)) => format!("{}–{}", num(a, 1), num(b, 1)), _ => "–".into() } }
                       Stat { label: "Ø Körperfett", value: opt_num(avg_fat, 1), unit: "%" }
                   }
                   if let Some(t) = target {
                       p { class: "text-xs text-muted-foreground",
                           "Zielgewicht {num(t, 1)} kg"
                           if let Some(l) = last { " · noch {num((l - t).abs(), 1)} kg" }
                       }
                   }
               }
           }
    }
       }
}

#[component]
fn TrainingCard(
    minutes: Vec<(String, f64)>,
    kcal: Vec<(String, f64)>,
    goal: f64,
    per: &'static str,
    workouts: i64,
    total_min: f64,
    total_kcal: f64,
    active_days: usize,
    days: i64,
    top: Vec<(String, f64, String)>,
) -> Element {
    let mut mode = use_signal(|| "min".to_string());
    rsx! {
           Anchor { id: "r-training",
    Card { class: "h-full",
               CardHeader { title: "Training", subtitle: format!("Minuten bzw. verbrannte kcal {per}") }
               div { class: "flex flex-col gap-4 px-4 pb-4",
                   Segmented { options: opts(&[("min", "Minuten"), ("kcal", "kcal")]), value: mode(), onchange: move |v| mode.set(v) }
                   if mode() == "min" {
                       BarChart { bars: minutes.clone(), goal: if goal > 0.0 { Some(goal) } else { None }, color: "text-protein" }
                   } else {
                       BarChart { bars: kcal.clone(), color: "text-fat" }
                   }
                   div { class: "grid grid-cols-2 gap-3 sm:grid-cols-4",
                       Stat { label: "Einheiten", value: workouts.to_string() }
                       Stat { label: "Aktive Tage", value: format!("{active_days}/{days}") }
                       Stat { label: "Minuten", value: int(total_min) }
                       Stat { label: "kcal", value: int(total_kcal) }
                   }
                   div { class: "flex flex-col gap-2",
                       p { class: "text-xs font-medium text-muted-foreground", "Top-Übungen" }
                       RankList { rows: top.clone(), color: "bg-protein" }
                   }
               }
           }
    }
       }
}

/// Sprungziel für die Abschnitts-Chips (Abstand für den Sticky-Kopf).
#[component]
fn Anchor(id: &'static str, children: Element) -> Element {
    rsx! {
        div { id, class: "min-w-0 scroll-mt-32", {children} }
    }
}

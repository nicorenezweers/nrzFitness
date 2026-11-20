//! Zusätzliche Diagramme für Berichte: Konsistenz-Kalender, gestapelte Säulen, mehrere Linien, Rangliste.

use chrono::{Datelike, Duration, NaiveDate};
use dioxus::prelude::*;

use crate::components::charts::ChartEmpty;
use crate::util::{MONTHS, num};

const W: f64 = 320.0;
const H: f64 = 140.0;
const PAD_L: f64 = 30.0;
const PAD_R: f64 = 8.0;
const PAD_T: f64 = 10.0;
const PAD_B: f64 = 20.0;

fn label_every(n: usize) -> usize {
    (n / 6).max(1)
}

/// Kalender-Heatmap (Spalten = Wochen ab Montag `from`, Zeilen = Mo..So). `levels`: Tag → 1..5.
#[component]
pub fn Heatmap(
    from: NaiveDate,
    to: NaiveDate,
    levels: Vec<(NaiveDate, u8)>,
    #[props(default = 5)] max_level: u8,
) -> Element {
    const CELL: f64 = 10.0;
    const GAP: f64 = 2.5;
    const LEFT: f64 = 16.0;
    const TOP: f64 = 12.0;
    let step = CELL + GAP;
    let days = (to - from).num_days().max(0) + 1;
    let weeks = ((days + 6) / 7).max(1);
    let w = LEFT + weeks as f64 * step;
    let h = TOP + 7.0 * step;
    let map: std::collections::HashMap<NaiveDate, u8> = levels.into_iter().collect();
    let max = max_level.max(1) as f64;
    let mut cells = Vec::new();
    let mut months = Vec::new();
    for wk in 0..weeks {
        let col_start = from + Duration::days(wk * 7);
        if col_start.day() <= 7 || wk == 0 {
            let x = LEFT + wk as f64 * step;
            // Erste Spalte nur beschriften, wenn genug Platz bis zum nächsten Monatsanfang bleibt.
            if wk > 0 || col_start.day() <= 14 {
                months.push((x, MONTHS[col_start.month0() as usize].chars().take(3).collect::<String>()));
            }
        }
        for d in 0..7 {
            let date = col_start + Duration::days(d);
            if date > to {
                continue;
            }
            let lvl = map.get(&date).copied().unwrap_or(0);
            let (cls, op) =
                if lvl == 0 { ("text-muted", 1.0) } else { ("text-good", 0.3 + 0.7 * (lvl as f64 / max).min(1.0)) };
            cells.push((LEFT + wk as f64 * step, TOP + d as f64 * step, cls, op, date));
        }
    }
    rsx! {
        svg { view_box: "0 0 {w} {h}", class: "h-auto w-full", style: "max-width: {(w * 2.2).max(320.0)}px",
            for (x, label) in months {
                text { x: "{x}", y: "8", class: "fill-muted-foreground", font_size: "7.5", "{label}" }
            }
            for (i, l) in [(0usize, "Mo"), (2, "Mi"), (4, "Fr")] {
                text { x: "0", y: "{TOP + i as f64 * step + CELL - 2.0}", class: "fill-muted-foreground", font_size: "7", "{l}" }
            }
            for (x, y, cls, op, date) in cells {
                rect {
                    key: "{date}",
                    x: "{x:.1}", y: "{y:.1}", width: "{CELL}", height: "{CELL}", rx: "2.5",
                    fill: "currentColor", fill_opacity: "{op:.2}", class: "{cls}",
                }
            }
        }
    }
}

/// Legende zur Heatmap („Weniger … Mehr“).
#[component]
pub fn HeatLegend() -> Element {
    rsx! {
        div { class: "flex items-center justify-end gap-1 text-[11px] text-muted-foreground",
            span { class: "mr-1", "Weniger" }
            span { class: "size-2.5 rounded-[3px] bg-muted" }
            for op in [30, 50, 70, 100] {
                span { class: "size-2.5 rounded-[3px] bg-good", style: "opacity: {op}%" }
            }
            span { class: "ml-1", "Mehr" }
        }
    }
}

/// Gestapelte Säulen. `bars`: (Beschriftung, Werte je Serie), `colors`: Textklasse je Serie.
#[component]
pub fn StackedBars(
    bars: Vec<(String, Vec<f64>)>,
    colors: Vec<String>,
    #[props(default = 0)] decimals: usize,
) -> Element {
    let totals: Vec<f64> = bars.iter().map(|b| b.1.iter().map(|v| v.max(0.0)).sum()).collect();
    let max = totals.iter().cloned().fold(0.0, f64::max);
    if max <= 0.0 {
        return rsx! { ChartEmpty {} };
    }
    let hi = max * 1.08;
    let n = bars.len().max(1);
    let slot = (W - PAD_L - PAD_R) / n as f64;
    let bw = (slot * 0.62).max(1.5);
    let y = |v: f64| PAD_T + (H - PAD_T - PAD_B) * (1.0 - v / hi);
    let every = label_every(bars.len());
    let ticks = [hi * 0.25, hi * 0.5, hi * 0.75];
    let mut rects = Vec::new();
    for (i, (_, vals)) in bars.iter().enumerate() {
        let x0 = PAD_L + slot * i as f64 + (slot - bw) / 2.0;
        let mut acc = 0.0;
        for (s, v) in vals.iter().enumerate() {
            let v = v.max(0.0);
            if v <= 0.0 {
                continue;
            }
            let top = y(acc + v);
            let bottom = y(acc);
            acc += v;
            rects.push((x0, top, bw, (bottom - top).max(0.5), colors.get(s).cloned().unwrap_or_default()));
        }
    }
    rsx! {
        svg { view_box: "0 0 {W} {H}", class: "h-auto w-full overflow-visible",
            for t in ticks {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(t):.1}", y2: "{y(t):.1}", stroke: "currentColor", class: "text-border", stroke_width: "1" }
                text { x: "{PAD_L - 4.0}", y: "{y(t) + 3.0:.1}", text_anchor: "end", class: "fill-muted-foreground", font_size: "8", "{num(t, decimals)}" }
            }
            for (x, top, w, h, cls) in rects {
                rect { x: "{x:.1}", y: "{top:.1}", width: "{w:.1}", height: "{h:.1}", fill: "currentColor", class: "{cls}" }
            }
            for (i, (label, _)) in bars.iter().enumerate() {
                if i % every == 0 || i == bars.len() - 1 {
                    text { x: "{PAD_L + slot * i as f64 + slot / 2.0:.1}", y: "{H - 6.0}", text_anchor: "middle", class: "fill-muted-foreground", font_size: "8", "{label}" }
                }
            }
        }
    }
}

/// Mehrere Linien auf gemeinsamer Achse. `series`: (Textfarbe, Werte); `labels`: x-Beschriftungen.
#[component]
pub fn MultiLine(
    labels: Vec<String>,
    series: Vec<(String, Vec<Option<f64>>)>,
    #[props(default)] min: Option<f64>,
    #[props(default)] max: Option<f64>,
    #[props(default = 0)] decimals: usize,
) -> Element {
    let vals: Vec<f64> = series.iter().flat_map(|s| s.1.iter().flatten().copied()).collect();
    if vals.is_empty() {
        return rsx! { ChartEmpty {} };
    }
    let lo = min.unwrap_or_else(|| vals.iter().cloned().fold(f64::INFINITY, f64::min));
    let mut hi = max.unwrap_or_else(|| vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max));
    if (hi - lo).abs() < 1e-9 {
        hi = lo + 1.0;
    }
    let n = labels.len().max(2);
    let x = |i: usize| PAD_L + (W - PAD_L - PAD_R) * i as f64 / (n - 1) as f64;
    let y = |v: f64| PAD_T + (H - PAD_T - PAD_B) * (1.0 - (v - lo) / (hi - lo));
    #[allow(clippy::type_complexity)]
    let paths: Vec<(String, String, Vec<(f64, f64)>)> = series
        .iter()
        .map(|(col, v)| {
            let pts: Vec<(f64, f64)> = v.iter().enumerate().filter_map(|(i, p)| p.map(|p| (x(i), y(p)))).collect();
            let d = pts
                .iter()
                .enumerate()
                .map(|(i, (a, b))| format!("{}{a:.1},{b:.1}", if i == 0 { "M" } else { "L" }))
                .collect::<String>();
            (col.clone(), d, pts)
        })
        .collect();
    let ticks = [lo, (lo + hi) / 2.0, hi];
    let every = label_every(labels.len());
    rsx! {
        svg { view_box: "0 0 {W} {H}", class: "h-auto w-full overflow-visible",
            for t in ticks {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(t):.1}", y2: "{y(t):.1}", stroke: "currentColor", class: "text-border", stroke_width: "1" }
                text { x: "{PAD_L - 4.0}", y: "{y(t) + 3.0:.1}", text_anchor: "end", class: "fill-muted-foreground", font_size: "8", "{num(t, decimals)}" }
            }
            for (col, d, pts) in paths {
                g { class: "{col}",
                    path { d: "{d}", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linejoin: "round", stroke_linecap: "round" }
                    if pts.len() <= 40 {
                        for (a, b) in pts {
                            circle { cx: "{a:.1}", cy: "{b:.1}", r: "2", fill: "currentColor" }
                        }
                    }
                }
            }
            for (i, l) in labels.iter().enumerate() {
                if i % every == 0 || i == labels.len() - 1 {
                    text { x: "{x(i):.1}", y: "{H - 6.0}", text_anchor: "middle", class: "fill-muted-foreground", font_size: "8", "{l}" }
                }
            }
        }
    }
}

/// Legende: farbiger Punkt (Hintergrundklasse, z.B. `bg-protein`) + Text.
#[component]
pub fn Legend(items: Vec<(String, String)>) -> Element {
    rsx! {
        div { class: "flex flex-wrap gap-x-4 gap-y-1 text-xs text-muted-foreground",
            for (col, label) in items {
                span { class: "inline-flex items-center gap-1.5",
                    span { class: "size-2.5 rounded-full {col}" }
                    "{label}"
                }
            }
        }
    }
}

/// Rangliste mit horizontalen Balken. `rows`: (Name, Wert, Anzeige-Text).
#[component]
pub fn RankList(
    rows: Vec<(String, f64, String)>,
    #[props(into, default = "bg-kcal".to_string())] color: String,
) -> Element {
    if rows.is_empty() {
        return rsx! { ChartEmpty {} };
    }
    let max = rows.iter().map(|r| r.1).fold(0.0, f64::max).max(1e-9);
    rsx! {
        ol { class: "flex flex-col gap-2.5",
            for (i, (name, v, text)) in rows.into_iter().enumerate() {
                li { key: "{i}-{name}", class: "flex flex-col gap-1",
                    div { class: "flex items-baseline gap-2 text-sm",
                        span { class: "w-5 shrink-0 text-right text-xs tabular-nums text-muted-foreground", "{i + 1}." }
                        span { class: "min-w-0 flex-1 truncate", "{name}" }
                        span { class: "shrink-0 text-xs tabular-nums text-muted-foreground", "{text}" }
                    }
                    div { class: "ml-7 h-1.5 overflow-hidden rounded-full bg-muted",
                        div { class: "h-full rounded-full {color}", style: "width: {(v / max * 100.0).clamp(2.0, 100.0):.0}%" }
                    }
                }
            }
        }
    }
}

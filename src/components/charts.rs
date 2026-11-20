//! Schlanke SVG-Diagramme (responsiv über viewBox, Farben über Tailwind-Textklassen = currentColor).
#![allow(dead_code)]

use dioxus::prelude::*;

use crate::util::num;

const W: f64 = 320.0;
const H: f64 = 140.0;
const PAD_L: f64 = 30.0;
const PAD_R: f64 = 8.0;
const PAD_T: f64 = 10.0;
const PAD_B: f64 = 20.0;

fn nice_range(min: f64, max: f64, zero: bool) -> (f64, f64) {
    let (mut lo, mut hi) = (min, max);
    if zero {
        lo = lo.min(0.0);
    }
    if (hi - lo).abs() < 1e-9 {
        hi += 1.0;
        lo -= if zero { 0.0 } else { 1.0 };
    }
    let pad = (hi - lo) * 0.08;
    (if zero && lo >= 0.0 { lo } else { lo - pad }, hi + pad)
}

/// Beschriftungen der x-Achse: höchstens ~6, gleichmässig verteilt.
fn label_every(n: usize) -> usize {
    (n / 6).max(1)
}

/// Liniendiagramm mit optionaler Ziellinie. Lücken (`None`) werden übersprungen.
#[component]
pub fn LineChart(
    points: Vec<(String, Option<f64>)>,
    #[props(default)] goal: Option<f64>,
    #[props(into, default = "text-kcal".to_string())] color: String,
    #[props(default = 1)] decimals: usize,
    #[props(default = false)] zero: bool,
    #[props(default = true)] area: bool,
) -> Element {
    let vals: Vec<f64> = points.iter().filter_map(|p| p.1).collect();
    if vals.is_empty() {
        return rsx! { ChartEmpty {} };
    }
    let mut min = vals.iter().cloned().fold(f64::INFINITY, f64::min);
    let mut max = vals.iter().cloned().fold(f64::NEG_INFINITY, f64::max);
    if let Some(g) = goal {
        min = min.min(g);
        max = max.max(g);
    }
    let (lo, hi) = nice_range(min, max, zero);
    let n = points.len().max(2);
    let x = |i: usize| PAD_L + (W - PAD_L - PAD_R) * i as f64 / (n - 1) as f64;
    let y = |v: f64| PAD_T + (H - PAD_T - PAD_B) * (1.0 - (v - lo) / (hi - lo));
    let pts: Vec<(f64, f64)> = points.iter().enumerate().filter_map(|(i, p)| p.1.map(|v| (x(i), y(v)))).collect();
    let line = pts.iter().enumerate().map(|(i, (a, b))| format!("{}{a:.1},{b:.1}", if i == 0 { "M" } else { "L" })).collect::<String>();
    let area_d = if area && pts.len() > 1 {
        let base = H - PAD_B;
        format!("{line}L{:.1},{base:.1}L{:.1},{base:.1}Z", pts[pts.len() - 1].0, pts[0].0)
    } else {
        String::new()
    };
    let ticks = [lo + (hi - lo) * 0.1, (lo + hi) / 2.0, hi - (hi - lo) * 0.1];
    let every = label_every(points.len());
    let last = pts.last().cloned();
    rsx! {
        svg { view_box: "0 0 {W} {H}", class: "h-auto w-full overflow-visible {color}",
            for t in ticks {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(t):.1}", y2: "{y(t):.1}", stroke: "currentColor", class: "text-border", stroke_width: "1" }
                text { x: "{PAD_L - 4.0}", y: "{y(t) + 3.0:.1}", text_anchor: "end", class: "fill-muted-foreground", font_size: "8", "{num(t, decimals)}" }
            }
            if let Some(g) = goal {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(g):.1}", y2: "{y(g):.1}", stroke: "currentColor", stroke_width: "1", stroke_dasharray: "3 3", class: "text-muted-foreground" }
            }
            if !area_d.is_empty() {
                path { d: "{area_d}", fill: "currentColor", fill_opacity: "0.12" }
            }
            path { d: "{line}", fill: "none", stroke: "currentColor", stroke_width: "2", stroke_linejoin: "round", stroke_linecap: "round" }
            if pts.len() <= 40 {
                for (a, b) in pts.iter().cloned() {
                    circle { cx: "{a:.1}", cy: "{b:.1}", r: "2", fill: "currentColor" }
                }
            }
            if let Some((a, b)) = last {
                circle { cx: "{a:.1}", cy: "{b:.1}", r: "3.5", fill: "currentColor", stroke: "var(--card)", stroke_width: "1.5" }
            }
            for (i, p) in points.iter().enumerate() {
                if i % every == 0 || i == points.len() - 1 {
                    text { x: "{x(i):.1}", y: "{H - 6.0}", text_anchor: "middle", class: "fill-muted-foreground", font_size: "8", "{p.0}" }
                }
            }
        }
    }
}

/// Säulendiagramm; über dem Ziel wird die Säule `over_color`.
#[component]
pub fn BarChart(
    bars: Vec<(String, f64)>,
    #[props(default)] goal: Option<f64>,
    #[props(into, default = "text-kcal".to_string())] color: String,
    #[props(into, default = String::new())] over_color: String,
    #[props(default = 0)] decimals: usize,
) -> Element {
    if bars.iter().all(|b| b.1 <= 0.0) {
        return rsx! { ChartEmpty {} };
    }
    let max = bars.iter().map(|b| b.1).fold(0.0, f64::max).max(goal.unwrap_or(0.0));
    let (_, hi) = nice_range(0.0, max, true);
    let n = bars.len().max(1);
    let slot = (W - PAD_L - PAD_R) / n as f64;
    let bw = (slot * 0.62).max(1.5);
    let y = |v: f64| PAD_T + (H - PAD_T - PAD_B) * (1.0 - v / hi);
    let every = label_every(bars.len());
    let ticks = [hi * 0.25, hi * 0.5, hi * 0.75];
    rsx! {
        svg { view_box: "0 0 {W} {H}", class: "h-auto w-full overflow-visible",
            for t in ticks {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(t):.1}", y2: "{y(t):.1}", stroke: "currentColor", class: "text-border", stroke_width: "1" }
                text { x: "{PAD_L - 4.0}", y: "{y(t) + 3.0:.1}", text_anchor: "end", class: "fill-muted-foreground", font_size: "8", "{num(t, decimals)}" }
            }
            for (i, (label, v)) in bars.iter().enumerate() {
                {
                    let x0 = PAD_L + slot * i as f64 + (slot - bw) / 2.0;
                    let top = y(v.max(0.0));
                    let h = (H - PAD_B - top).max(if *v > 0.0 { 1.5 } else { 0.0 });
                    let over = goal.is_some_and(|g| *v > g * 1.05) && !over_color.is_empty();
                    let cls = if over { over_color.clone() } else { color.clone() };
                    rsx! {
                        rect { x: "{x0:.1}", y: "{H - PAD_B - h:.1}", width: "{bw:.1}", height: "{h:.1}", rx: "{(bw / 3.0).min(4.0):.1}", fill: "currentColor", class: "{cls}" }
                        if i % every == 0 || i == bars.len() - 1 {
                            text { x: "{x0 + bw / 2.0:.1}", y: "{H - 6.0}", text_anchor: "middle", class: "fill-muted-foreground", font_size: "8", "{label}" }
                        }
                    }
                }
            }
            if let Some(g) = goal {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(g):.1}", y2: "{y(g):.1}", stroke: "currentColor", stroke_width: "1", stroke_dasharray: "3 3", class: "text-muted-foreground" }
            }
        }
    }
}

/// Ringdiagramm (Anteile), z.B. Makro-Verteilung. `parts`: (Tailwind-Textfarbe, Wert).
#[component]
pub fn Donut(parts: Vec<(String, f64)>, #[props(default = 96.0)] size: f64, #[props(default = 12.0)] stroke: f64, children: Element) -> Element {
    let total: f64 = parts.iter().map(|p| p.1.max(0.0)).sum();
    let r = (size - stroke) / 2.0;
    let c = 2.0 * std::f64::consts::PI * r;
    let half = size / 2.0;
    let mut acc = 0.0;
    let segs: Vec<(String, f64, f64)> = parts
        .iter()
        .filter(|p| p.1 > 0.0 && total > 0.0)
        .map(|(col, v)| {
            let len = c * v / total;
            let s = (col.clone(), len, acc);
            acc += len;
            s
        })
        .collect();
    rsx! {
        div { class: "relative shrink-0", style: "width: {size}px; height: {size}px;",
            svg { width: "{size}", height: "{size}", view_box: "0 0 {size} {size}", class: "-rotate-90",
                circle { cx: "{half}", cy: "{half}", r: "{r}", fill: "none", stroke: "currentColor", stroke_width: "{stroke}", class: "text-muted" }
                for (col, len, off) in segs {
                    circle {
                        cx: "{half}", cy: "{half}", r: "{r}", fill: "none", stroke: "currentColor", stroke_width: "{stroke}",
                        stroke_dasharray: "{(len - 1.5).max(0.5):.2} {c:.2}", stroke_dashoffset: "{-off:.2}", class: "{col}",
                    }
                }
            }
            div { class: "absolute inset-0 flex flex-col items-center justify-center text-center", {children} }
        }
    }
}

#[component]
pub fn ChartEmpty() -> Element {
    rsx! {
        div { class: "flex h-28 items-center justify-center rounded-xl border border-dashed text-sm text-muted-foreground",
            "Noch keine Daten"
        }
    }
}

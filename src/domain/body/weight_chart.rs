//! Gewichtsverlauf: einzelne Messungen als Punkte, gleitender 7-Tage-Schnitt als Linie, Ziel gestrichelt.

use chrono::NaiveDate;
use dioxus::prelude::*;

use crate::components::charts::ChartEmpty;
use crate::model::{WeightPoint, moving_average};
use crate::util::{fmt_dm, num};

const W: f64 = 340.0;
const H: f64 = 170.0;
const PAD_L: f64 = 32.0;
const PAD_R: f64 = 10.0;
const PAD_T: f64 = 10.0;
const PAD_B: f64 = 22.0;

#[component]
pub fn WeightChart(weights: Vec<WeightPoint>, from: NaiveDate, to: NaiveDate, goal: Option<f64>) -> Element {
    let shown: Vec<WeightPoint> = weights.iter().filter(|w| w.date >= from && w.date <= to).copied().collect();
    if shown.is_empty() {
        return rsx! { ChartEmpty {} };
    }
    let avg = moving_average(&weights, from, to, 7);
    let mut lo = shown.iter().map(|w| w.weight).fold(f64::INFINITY, f64::min);
    let mut hi = shown.iter().map(|w| w.weight).fold(f64::NEG_INFINITY, f64::max);
    // Ziel nur einbeziehen, wenn es nicht zu weit weg ist (sonst wird der Verlauf flach).
    let goal_in = goal.filter(|g| (*g - lo).abs().min((*g - hi).abs()) <= (hi - lo).max(2.0) * 1.5);
    if let Some(g) = goal_in {
        lo = lo.min(g);
        hi = hi.max(g);
    }
    if hi - lo < 1.0 {
        let mid = (hi + lo) / 2.0;
        lo = mid - 0.5;
        hi = mid + 0.5;
    }
    let pad = (hi - lo) * 0.12;
    let (lo, hi) = (lo - pad, hi + pad);
    let span = (to - from).num_days().max(1) as f64;
    let x = |d: NaiveDate| PAD_L + (W - PAD_L - PAD_R) * (d - from).num_days() as f64 / span;
    let y = |v: f64| PAD_T + (H - PAD_T - PAD_B) * (1.0 - (v - lo) / (hi - lo));

    // Linie des Durchschnitts, Lücken unterbrechen sie.
    let mut line = String::new();
    let mut seg: Vec<(f64, f64)> = Vec::new();
    let mut segs: Vec<Vec<(f64, f64)>> = Vec::new();
    for (d, v) in &avg {
        match v {
            Some(v) => seg.push((x(*d), y(*v))),
            None if !seg.is_empty() => segs.push(std::mem::take(&mut seg)),
            None => {}
        }
    }
    if !seg.is_empty() {
        segs.push(seg);
    }
    let mut area = String::new();
    let base = H - PAD_B;
    for s in &segs {
        for (i, (a, b)) in s.iter().enumerate() {
            line.push_str(&format!("{}{a:.1},{b:.1}", if i == 0 { "M" } else { "L" }));
        }
        if s.len() > 1 {
            let pts: String = s.iter().map(|(a, b)| format!("L{a:.1},{b:.1}")).collect();
            area.push_str(&format!("M{:.1},{base:.1}{pts}L{:.1},{base:.1}Z", s[0].0, s[s.len() - 1].0));
        }
    }
    let ticks = [lo + (hi - lo) * 0.15, (lo + hi) / 2.0, hi - (hi - lo) * 0.15];
    let labels: Vec<NaiveDate> =
        (0..5).map(|i| from + chrono::Duration::days((span * i as f64 / 4.0).round() as i64)).collect();
    let last = shown.last().copied();
    let dot_r = if shown.len() > 60 { 1.6 } else { 2.4 };
    rsx! {
        svg { view_box: "0 0 {W} {H}", class: "h-auto w-full overflow-visible text-kcal",
            for t in ticks {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(t):.1}", y2: "{y(t):.1}", stroke: "currentColor", class: "text-border", stroke_width: "1" }
                text { x: "{PAD_L - 5.0}", y: "{y(t) + 3.0:.1}", text_anchor: "end", class: "fill-muted-foreground", font_size: "9", "{num(t, 1)}" }
            }
            if let Some(g) = goal_in {
                line { x1: "{PAD_L}", x2: "{W - PAD_R}", y1: "{y(g):.1}", y2: "{y(g):.1}", stroke: "currentColor", stroke_width: "1.2", stroke_dasharray: "4 4", class: "text-good" }
                text { x: "{W - PAD_R}", y: "{y(g) - 4.0:.1}", text_anchor: "end", class: "fill-good", font_size: "9", "Ziel {num(g, 1)}" }
            }
            path { d: "{area}", fill: "currentColor", fill_opacity: "0.10" }
            for w in shown.iter() {
                circle { cx: "{x(w.date):.1}", cy: "{y(w.weight):.1}", r: "{dot_r}", fill: "currentColor", fill_opacity: "0.45" }
            }
            path { d: "{line}", fill: "none", stroke: "currentColor", stroke_width: "2.5", stroke_linejoin: "round", stroke_linecap: "round" }
            if let Some(l) = last {
                circle { cx: "{x(l.date):.1}", cy: "{y(l.weight):.1}", r: "4", fill: "currentColor", stroke: "var(--card)", stroke_width: "2" }
            }
            for (i, d) in labels.iter().enumerate() {
                text {
                    x: "{x(*d):.1}", y: "{H - 6.0}",
                    text_anchor: if i == 0 { "start" } else if i == 4 { "end" } else { "middle" },
                    class: "fill-muted-foreground", font_size: "9", "{fmt_dm(*d)}"
                }
            }
        }
        div { class: "mt-1 flex items-center gap-4 px-1 text-[11px] text-muted-foreground",
            span { class: "flex items-center gap-1.5", span { class: "size-2 rounded-full bg-kcal/50" } "Messung" }
            span { class: "flex items-center gap-1.5", span { class: "h-0.5 w-4 rounded bg-kcal" } "7-Tage-Schnitt" }
            if goal_in.is_some() {
                span { class: "flex items-center gap-1.5", span { class: "h-0 w-4 border-t border-dashed border-good" } "Ziel" }
            }
        }
    }
}

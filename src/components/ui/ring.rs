use dioxus::prelude::*;

/// Fortschrittsring (SVG). `value` 0..1 (über 1 = überschritten, Ring wird `over_class`).
#[component]
pub fn Ring(
    value: f64,
    #[props(default = 120.0)] size: f64,
    #[props(default = 10.0)] stroke: f64,
    /// Tailwind-Textklasse für die Farbe (stroke = currentColor).
    #[props(into, default = "text-kcal".to_string())]
    color: String,
    #[props(into, default = "text-bad".to_string())] over_color: String,
    children: Element,
) -> Element {
    let r = (size - stroke) / 2.0;
    let c = 2.0 * std::f64::consts::PI * r;
    let v = if value.is_finite() { value.max(0.0) } else { 0.0 };
    let shown = v.min(1.0);
    let offset = c * (1.0 - shown);
    let col = if v > 1.0 { over_color.clone() } else { color.clone() };
    let half = size / 2.0;
    rsx! {
        div { class: "relative shrink-0", style: "width: {size}px; height: {size}px;",
            svg { width: "{size}", height: "{size}", view_box: "0 0 {size} {size}", class: "-rotate-90",
                circle { cx: "{half}", cy: "{half}", r: "{r}", fill: "none", stroke: "currentColor", stroke_width: "{stroke}", class: "text-muted" }
                circle {
                    cx: "{half}", cy: "{half}", r: "{r}", fill: "none", stroke: "currentColor", stroke_width: "{stroke}",
                    stroke_linecap: "round", stroke_dasharray: "{c}", stroke_dashoffset: "{offset}",
                    class: "ring-track {col}",
                }
            }
            div { class: "absolute inset-0 flex flex-col items-center justify-center text-center", {children} }
        }
    }
}

/// Dünner Fortschrittsbalken.
#[component]
pub fn Bar(
    value: f64,
    #[props(into, default = "bg-kcal".to_string())] color: String,
    #[props(into, default = "h-1.5".to_string())] height: String,
) -> Element {
    let v = if value.is_finite() { value.max(0.0) } else { 0.0 };
    let w = (v.min(1.0) * 100.0).round();
    let col = if v > 1.05 { "bg-bad".to_string() } else { color.clone() };
    rsx! {
        div { class: "w-full overflow-hidden rounded-full bg-muted {height}",
            div { class: "h-full rounded-full transition-all duration-500 {col}", style: "width: {w}%" }
        }
    }
}

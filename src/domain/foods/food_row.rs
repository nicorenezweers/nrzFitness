use dioxus::prelude::*;
use icons::{Plus, Star};

use crate::model::Food;
use crate::util::num;

/// Zeile in Lebensmittellisten: Bild/Initial, Name, Marke · kcal, rechts Schnell-Plus.
#[component]
pub fn FoodRow(
    food: Food,
    onclick: EventHandler<()>,
    #[props(optional)] onquick: Option<EventHandler<()>>,
    /// Rechts statt kcal/Portion anzeigen.
    #[props(into, optional)]
    trailing: Option<String>,
) -> Element {
    let portion = food.serving_size;
    let kcal_portion = food.kcal * portion / 100.0;
    let portion_label = food.serving_name.clone().unwrap_or_else(|| format!("{} {}", num(portion, 0), food.unit));
    let sub = {
        let mut p = Vec::new();
        if let Some(b) = food.brand.as_ref().filter(|b| !b.is_empty()) {
            p.push(b.clone());
        }
        p.push(portion_label);
        p.join(" · ")
    };
    let letter = food.name.chars().next().map(|c| c.to_uppercase().to_string()).unwrap_or_default();
    rsx! {
        div { class: "flex items-center gap-3 rounded-xl px-2 py-2 transition-colors active:bg-accent hover:bg-accent/40",
            button { r#type: "button", class: "flex min-w-0 flex-1 items-center gap-3 text-left", onclick: move |_| onclick.call(()),
                if let Some(img) = food.image_url.as_ref() {
                    img { src: "{img}", class: "size-11 shrink-0 rounded-xl bg-white object-contain p-0.5", loading: "lazy", alt: "" }
                } else {
                    span { class: "flex size-11 shrink-0 items-center justify-center rounded-xl bg-muted text-base font-semibold text-muted-foreground", "{letter}" }
                }
                span { class: "flex min-w-0 flex-1 flex-col",
                    span { class: "flex items-center gap-1 truncate text-[15px] font-medium",
                        span { class: "truncate", "{food.name}" }
                        if food.favorite { Star { class: "size-3.5 shrink-0 fill-current text-carbs" } }
                    }
                    span { class: "truncate text-xs text-muted-foreground", "{sub}" }
                }
                span { class: "shrink-0 text-right text-sm tabular-nums",
                    if let Some(t) = &trailing { "{t}" } else {
                        span { class: "font-semibold", "{kcal_portion.round()}" }
                        span { class: "ml-0.5 text-xs text-muted-foreground", "kcal" }
                    }
                }
            }
            if let Some(q) = onquick {
                button {
                    r#type: "button",
                    "aria-label": "Direkt hinzufügen",
                    class: "flex size-9 shrink-0 items-center justify-center rounded-full bg-muted text-foreground transition-transform active:scale-90 hover:bg-accent",
                    onclick: move |_| q.call(()),
                    Plus { class: "size-4" }
                }
            }
        }
    }
}

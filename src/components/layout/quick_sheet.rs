use chrono::Timelike;
use dioxus::prelude::*;
use icons::{Dumbbell, GlassWater, Scale, Smile, Timer, Utensils};

use crate::api::diary::add_water;
use crate::app::Route;
use crate::components::ui::sheet::Sheet;
use crate::model::meal_for_hour;
use crate::state::use_app;
use crate::util::today;

/// Schnellaktionen hinter dem (+) in der Tab-Leiste.
#[component]
pub fn QuickSheet() -> Element {
    let mut app = use_app();
    let navigator = use_navigator();
    if !(app.quick_open)() {
        return rsx! {};
    }
    let close = move || {
        let mut app = app;
        app.quick_open.set(false);
    };
    let tile = "flex flex-col items-start gap-3 rounded-2xl bg-muted/70 p-4 text-left transition-transform active:scale-95";
    let icon = "flex size-10 items-center justify-center rounded-full";
    rsx! {
        Sheet { title: "Hinzufügen", onclose: move |_| app.quick_open.set(false),
            div { class: "grid grid-cols-2 gap-2.5 p-4 pt-2",
                button { class: "{tile} col-span-2 flex-row items-center bg-primary text-primary-foreground",
                    onclick: move |_| {
                        close();
                        let d = (app.date)();
                        app.open_food(d, meal_for_hour(chrono::Local::now().hour()));
                    },
                    span { class: "{icon} bg-primary-foreground/15", Utensils { class: "size-5" } }
                    span { class: "flex flex-col",
                        span { class: "font-semibold", "Essen eintragen" }
                        span { class: "text-xs opacity-70", "Suchen, scannen oder schnell erfassen" }
                    }
                }
                button { class: tile,
                    onclick: move |_| {
                        close();
                        let pid = app.pid();
                        let d = (app.date)();
                        spawn(async move {
                            match add_water(pid, d, 250.0).await {
                                Ok(()) => { app.bump(); app.toast("+250 ml Wasser"); }
                                Err(e) => app.error(e),
                            }
                        });
                    },
                    span { class: "{icon} bg-water/20 text-water", GlassWater { class: "size-5" } }
                    span { class: "flex flex-col",
                        span { class: "font-semibold", "Wasser" }
                        span { class: "text-xs text-muted-foreground", "+250 ml" }
                    }
                }
                button { class: tile,
                    onclick: move |_| { close(); navigator.push(Route::Training {}); },
                    span { class: "{icon} bg-protein/20 text-protein", Dumbbell { class: "size-5" } }
                    span { class: "flex flex-col",
                        span { class: "font-semibold", "Training" }
                        span { class: "text-xs text-muted-foreground", "Übung erfassen" }
                    }
                }
                button { class: tile,
                    onclick: move |_| { close(); navigator.push(Route::Body {}); },
                    span { class: "{icon} bg-kcal/20 text-kcal", Scale { class: "size-5" } }
                    span { class: "flex flex-col",
                        span { class: "font-semibold", "Gewicht" }
                        span { class: "text-xs text-muted-foreground", "Check-in & Masse" }
                    }
                }
                button { class: tile,
                    onclick: move |_| { close(); navigator.push(Route::Fasting {}); },
                    span { class: "{icon} bg-carbs/20 text-carbs", Timer { class: "size-5" } }
                    span { class: "flex flex-col",
                        span { class: "font-semibold", "Fasten" }
                        span { class: "text-xs text-muted-foreground", "Timer starten/stoppen" }
                    }
                }
                button { class: "{tile} col-span-2 flex-row items-center",
                    onclick: move |_| { close(); navigator.push(Route::Body {}); },
                    span { class: "{icon} bg-fat/20 text-fat", Smile { class: "size-5" } }
                    span { class: "flex flex-col",
                        span { class: "font-semibold", "Stimmung, Schlaf & Schritte" }
                        span { class: "text-xs text-muted-foreground", "Täglicher Check-in für {crate::util::fmt_relative(today())}" }
                    }
                }
            }
        }
    }
}

use dioxus::prelude::*;
use icons::{ChartColumn, Dumbbell, LayoutGrid, Plus, Sun};

use crate::app::Route;
use crate::components::hooks::use_is_current_path::use_is_current_path;
use crate::components::layout::nav::NAV;
use crate::state::use_app;

/// Tab-Leiste unten (unter `lg`): Heute, Training, (+), Berichte, Mehr.
#[component]
pub fn AppBottomNav() -> Element {
    let is_current = use_is_current_path();
    let navigator = use_navigator();
    let mut app = use_app();
    let more_active = NAV.iter().filter(|i| !matches!(i.path, "/" | "/training" | "/berichte")).any(|i| is_current(i.path));
    let tab = |active: bool| {
        if active {
            "flex flex-col items-center justify-center gap-0.5 text-foreground transition-transform active:scale-90"
        } else {
            "flex flex-col items-center justify-center gap-0.5 text-muted-foreground transition-transform active:scale-90"
        }
    };
    rsx! {
        nav { class: "fixed inset-x-0 bottom-0 z-50 border-t bg-background/85 pb-[env(safe-area-inset-bottom)] backdrop-blur-xl lg:hidden",
            div { class: "mx-auto grid h-[3.75rem] max-w-lg grid-cols-5",
                button { class: tab(is_current("/")), onclick: move |_| { navigator.push(Route::Today {}); },
                    Sun { class: "size-[22px]" }
                    span { class: "text-[10px] font-medium", "Heute" }
                }
                button { class: tab(is_current("/training")), onclick: move |_| { navigator.push(Route::Training {}); },
                    Dumbbell { class: "size-[22px]" }
                    span { class: "text-[10px] font-medium", "Training" }
                }
                div { class: "flex items-center justify-center",
                    button {
                        "aria-label": "Hinzufügen",
                        class: "-mt-5 flex size-14 items-center justify-center rounded-full bg-primary text-primary-foreground shadow-lg shadow-black/30 ring-4 ring-background transition-transform active:scale-90",
                        onclick: move |_| app.quick_open.set(true),
                        Plus { class: "size-7" }
                    }
                }
                button { class: tab(is_current("/berichte")), onclick: move |_| { navigator.push(Route::Reports {}); },
                    ChartColumn { class: "size-[22px]" }
                    span { class: "text-[10px] font-medium", "Berichte" }
                }
                button { class: tab(more_active), onclick: move |_| app.more_open.set(true),
                    LayoutGrid { class: "size-[22px]" }
                    span { class: "text-[10px] font-medium", "Mehr" }
                }
            }
        }
    }
}

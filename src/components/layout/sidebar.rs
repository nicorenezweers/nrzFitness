use dioxus::prelude::*;
use icons::Plus;

use crate::components::hooks::use_is_current_path::use_is_current_path;
use crate::components::layout::nav::{NAV, NavIcon, route_for};
use crate::components::layout::profile_switcher::ProfileButton;
use crate::components::layout::theme_toggle::ThemeToggle;
use crate::state::use_app;
use crate::util::today;

/// Seitenleiste (ab `lg`): Logo, Profil, Navigation.
#[component]
pub fn Sidebar() -> Element {
    let is_current = use_is_current_path();
    let mut app = use_app();
    let cycle = app.profile().is_some_and(|p| p.cycle_tracking);
    rsx! {
        aside { class: "hidden w-60 shrink-0 flex-col border-r bg-sidebar/40 lg:flex",
            div { class: "flex items-center gap-2.5 px-5 pb-3 pt-5",
                div { class: "flex size-8 items-center justify-center rounded-xl bg-primary text-sm font-bold text-primary-foreground", "F" }
                span { class: "text-[15px] font-semibold tracking-tight", "nrzFitness" }
            }
            div { class: "px-3 pb-2", ProfileButton {} }
            div { class: "px-3 pb-3",
                button {
                    class: "flex h-10 w-full items-center justify-center gap-2 rounded-xl bg-primary text-sm font-medium text-primary-foreground transition-transform hover:bg-primary/90 active:scale-[0.98]",
                    onclick: move |_| {
                        let h = chrono::Timelike::hour(&chrono::Local::now());
                        app.open_food(today(), crate::model::meal_for_hour(h));
                    },
                    Plus { class: "size-4" }
                    "Essen eintragen"
                }
            }
            nav { class: "flex flex-1 flex-col gap-0.5 overflow-y-auto px-3",
                for item in NAV.iter().filter(|i| cycle || i.path != "/zyklus") {
                    Link {
                        key: "{item.path}",
                        to: route_for(item.path),
                        class: if is_current(item.path) { "flex items-center gap-3 rounded-xl bg-accent px-3 py-2 text-sm font-medium text-foreground" } else { "flex items-center gap-3 rounded-xl px-3 py-2 text-sm text-muted-foreground transition-colors hover:bg-accent/60 hover:text-foreground" },
                        NavIcon { name: item.icon, class: "size-[18px]" }
                        "{item.label}"
                    }
                }
            }
            div { class: "flex items-center justify-between border-t px-4 py-3",
                span { class: "text-xs text-muted-foreground", "v{env!(\"CARGO_PKG_VERSION\")}" }
                ThemeToggle {}
            }
        }
    }
}

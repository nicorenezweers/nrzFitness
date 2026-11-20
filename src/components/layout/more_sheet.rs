use dioxus::prelude::*;

use crate::components::layout::nav::{NAV, NavIcon, route_for};
use crate::components::layout::theme_toggle::ThemeToggle;
use crate::components::ui::sheet::Sheet;
use crate::state::use_app;

/// „Mehr“ (mobil): alle Bereiche als Kachel-Raster.
#[component]
pub fn MoreSheet() -> Element {
    let mut app = use_app();
    let navigator = use_navigator();
    if !(app.more_open)() {
        return rsx! {};
    }
    let cycle = app.profile().is_some_and(|p| p.cycle_tracking);
    rsx! {
        Sheet { title: "Mehr", onclose: move |_| app.more_open.set(false),
            header_action: rsx! { ThemeToggle {} },
            div { class: "grid grid-cols-3 gap-2.5 p-4 pt-2",
                for item in NAV.iter().filter(|i| i.path != "/" && (cycle || i.path != "/zyklus")) {
                    button {
                        key: "{item.path}",
                        class: "flex aspect-[1.1] flex-col items-center justify-center gap-2 rounded-2xl bg-muted/70 text-sm font-medium transition-transform active:scale-95",
                        onclick: move |_| {
                            app.more_open.set(false);
                            navigator.push(route_for(item.path));
                        },
                        span { class: "flex size-11 items-center justify-center rounded-full bg-background",
                            NavIcon { name: item.icon, class: "size-5" }
                        }
                        span { class: "text-[13px]", "{item.label}" }
                    }
                }
            }
        }
    }
}

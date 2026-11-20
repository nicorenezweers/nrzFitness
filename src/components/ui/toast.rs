use dioxus::prelude::*;

use crate::state::use_app;

/// Kurze Meldung unten (über der Tab-Leiste), verschwindet nach 2.5 s.
#[component]
pub fn Toast() -> Element {
    let mut app = use_app();
    let current = app.toast.read().clone();
    use_effect(move || {
        if app.toast.read().is_some() {
            spawn(async move {
                let msg = app.toast.peek().clone();
                let _ = document::eval("await new Promise(r => setTimeout(r, 2600));").await;
                if *app.toast.peek() == msg {
                    app.toast.set(None);
                }
            });
        }
    });
    let Some((msg, err)) = current else { return rsx! {} };
    rsx! {
        div {
            key: "{msg}",
            class: "anim-toast fixed left-1/2 z-[400] w-max max-w-[92vw] -translate-x-1/2 rounded-full px-4 py-2.5 text-sm font-medium shadow-xl bottom-[calc(env(safe-area-inset-bottom)+5.5rem)] lg:bottom-6",
            class: if err { "bg-destructive text-white" } else { "bg-foreground text-background" },
            onclick: move |_| app.toast.set(None),
            "{msg}"
        }
    }
}

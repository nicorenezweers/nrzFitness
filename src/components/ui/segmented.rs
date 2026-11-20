use dioxus::prelude::*;
use tw_merge::tw_merge;

/// Segment-Umschalter (iOS-Stil). `options`: (Wert, Beschriftung).
#[component]
pub fn Segmented(
    options: Vec<(String, String)>,
    value: String,
    onchange: EventHandler<String>,
    #[props(into, optional)] class: Option<String>,
) -> Element {
    let merged = tw_merge!("flex w-full gap-1 rounded-xl bg-muted p-1", class.as_deref().unwrap_or(""));
    rsx! {
        div { class: "{merged}",
            for (v, label) in options {
                {
                    let active = v == value;
                    let vv = v.clone();
                    rsx! {
                        button {
                            r#type: "button",
                            key: "{v}",
                            class: if active { "flex-1 truncate rounded-lg bg-background px-2 py-1.5 text-sm font-medium shadow-sm" } else { "flex-1 truncate rounded-lg px-2 py-1.5 text-sm text-muted-foreground hover:text-foreground" },
                            onclick: move |_| onchange.call(vv.clone()),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

/// Horizontale Chips (scrollbar auf dem Handy).
#[component]
pub fn Chips(options: Vec<(String, String)>, value: String, onchange: EventHandler<String>) -> Element {
    rsx! {
        div { class: "no-scrollbar -mx-4 flex gap-2 overflow-x-auto px-4 sm:mx-0 sm:flex-wrap sm:px-0",
            for (v, label) in options {
                {
                    let active = v == value;
                    let vv = v.clone();
                    rsx! {
                        button {
                            r#type: "button",
                            key: "{v}",
                            class: if active { "shrink-0 rounded-full bg-primary px-3.5 py-1.5 text-sm font-medium text-primary-foreground" } else { "shrink-0 rounded-full border px-3.5 py-1.5 text-sm text-muted-foreground hover:text-foreground" },
                            onclick: move |_| onchange.call(vv.clone()),
                            "{label}"
                        }
                    }
                }
            }
        }
    }
}

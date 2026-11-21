use dioxus::prelude::*;
use icons::{ChevronDown, Info};

/// Kleiner Info-Knopf bei sehr ehrgeizigen Abnehmzielen. Erlaubt das Ziel, weist aber auf ärztlichen Rat hin.
#[component]
pub fn GoalHint(reason: String, #[props(into, optional)] class: Option<String>) -> Element {
    let mut open = use_signal(|| false);
    rsx! {
        div { class: "flex flex-col gap-2 {class.clone().unwrap_or_default()}",
            button {
                r#type: "button",
                class: "flex items-center gap-1.5 self-start rounded-full bg-warn/15 px-3 py-1.5 text-xs font-medium text-warn transition-transform active:scale-95",
                "aria-expanded": "{open()}",
                onclick: move |_| open.toggle(),
                Info { class: "size-3.5" }
                "Mehr Abnahme als empfohlen"
                ChevronDown { class: if open() { "size-3.5 rotate-180 transition-transform" } else { "size-3.5 transition-transform" } }
            }
            if open() {
                div { class: "rounded-xl bg-warn/10 px-3 py-2.5 text-xs leading-relaxed text-foreground/90",
                    p { "Hey, du nimmst damit viel mehr ab als gewohnt empfohlen: {reason}." }
                    p { class: "mt-1.5 text-muted-foreground",
                        "Empfohlen sind etwa 0,5–1 % des Körpergewichts pro Woche. Sprich mit deiner Ärztin oder deinem Arzt darüber, bevor du so stark reduzierst – vor allem über längere Zeit."
                    }
                }
            }
        }
    }
}

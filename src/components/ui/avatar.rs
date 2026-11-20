use dioxus::prelude::*;

use crate::util::{initials, profile_bg};

#[component]
pub fn Avatar(#[props(into)] name: String, #[props(into)] color: String, #[props(default = "size-8 text-xs".to_string())] class: String) -> Element {
    rsx! {
        span { class: "inline-flex shrink-0 items-center justify-center rounded-full font-semibold text-white {profile_bg(&color)} {class}",
            "{initials(&name)}"
        }
    }
}

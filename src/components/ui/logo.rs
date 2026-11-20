use dioxus::prelude::*;

const LOGO: Asset = asset!("/public/icons/logo.png");

/// App-Logo (Ring mit Puls).
#[component]
pub fn Logo(#[props(default = "size-8".to_string())] class: String) -> Element {
    rsx! { img { src: LOGO, alt: "nrzFitness", class: "shrink-0 {class}" } }
}

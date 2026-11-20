use dioxus::prelude::*;

#[component]
pub fn AppWrapper(children: Element) -> Element {
    rsx! {
        // Leerer touchstart aktiviert CSS :active auf iOS
        div { class: "flex h-full flex-col", ontouchstart: |_| {}, {children} }
    }
}

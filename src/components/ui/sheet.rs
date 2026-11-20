use dioxus::prelude::*;
use icons::X;
use tw_merge::tw_merge;

/// Dialog: auf dem Handy ein Bottom-Sheet (von unten, Griff, Safe-Area), ab `sm` ein zentriertes Fenster.
/// Schliesst per Klick auf den Hintergrund, X oder Escape.
#[component]
pub fn Sheet(
    #[props(into, optional)] title: Option<String>,
    #[props(into, optional)] class: Option<String>,
    /// Feste, fast volle Höhe (z.B. Suche) statt Inhaltshöhe.
    #[props(default = false)]
    tall: bool,
    /// Optionaler fixer Fuss (Buttons), bleibt unten sichtbar.
    #[props(optional)]
    footer: Option<Element>,
    /// Zusätzliche Aktion rechts im Kopf.
    #[props(optional)]
    header_action: Option<Element>,
    onclose: EventHandler<()>,
    children: Element,
) -> Element {
    let height = if tall { "h-[94dvh] sm:h-[min(760px,88dvh)]" } else { "max-h-[92dvh] sm:max-h-[88dvh]" };
    let panel = tw_merge!(
        "anim-sheet sm:sm-anim-pop relative flex w-full flex-col overflow-hidden border bg-popover text-popover-foreground shadow-2xl",
        "rounded-t-[1.6rem] sm:rounded-2xl sm:max-w-lg",
        height,
        class.as_deref().unwrap_or("")
    );
    rsx! {
        div {
            class: "anim-fade fixed inset-0 z-[200] flex items-end justify-center bg-black/55 backdrop-blur-[2px] sm:items-center sm:p-6",
            tabindex: "-1",
            onmounted: move |e| async move { let _ = e.set_focus(true).await; },
            onkeydown: move |e| if e.key() == Key::Escape { onclose.call(()) },
            onclick: move |_| onclose.call(()),
            div { class: "{panel}", onclick: move |e| e.stop_propagation(),
                // Griff (nur mobil)
                div { class: "flex justify-center pt-2.5 sm:hidden",
                    div { class: "h-1.5 w-10 rounded-full bg-muted-foreground/30" }
                }
                if title.is_some() || header_action.is_some() {
                    div { class: "flex items-center gap-2 px-5 pb-2 pt-2 sm:border-b sm:py-3",
                        h2 { class: "min-w-0 flex-1 truncate text-lg font-semibold sm:text-base", "{title.clone().unwrap_or_default()}" }
                        {header_action}
                        button {
                            r#type: "button",
                            class: "-mr-1 flex size-9 items-center justify-center rounded-full bg-muted text-muted-foreground hover:text-foreground sm:size-8 sm:bg-transparent",
                            onclick: move |_| onclose.call(()),
                            X { class: "size-4" }
                        }
                    }
                }
                div { class: "min-h-0 flex-1 overflow-y-auto overscroll-contain", {children} }
                if let Some(f) = footer {
                    div { class: "border-t bg-popover px-5 pt-3 pb-[max(0.75rem,env(safe-area-inset-bottom))]", {f} }
                } else {
                    div { class: "pb-[env(safe-area-inset-bottom)] sm:hidden" }
                }
            }
        }
    }
}

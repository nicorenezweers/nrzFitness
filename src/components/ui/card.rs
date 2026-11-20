use dioxus::prelude::*;
use tw_merge::tw_merge;

#[component]
pub fn Card(
    #[props(into, optional)] class: Option<String>,
    #[props(optional)] onclick: Option<EventHandler<MouseEvent>>,
    children: Element,
) -> Element {
    let interactive = if onclick.is_some() { "cursor-pointer transition-transform active:scale-[0.985] hover:bg-accent/40" } else { "" };
    let merged = tw_merge!("rounded-2xl border bg-card text-card-foreground", interactive, class.as_deref().unwrap_or(""));
    rsx! {
        div {
            class: "{merged}",
            onclick: move |e| if let Some(h) = &onclick { h.call(e) },
            {children}
        }
    }
}

/// Kopfzeile einer Karte: Titel links, optional Aktion rechts.
#[component]
pub fn CardHeader(
    #[props(into)] title: String,
    #[props(into, optional)] subtitle: Option<String>,
    #[props(optional)] action: Option<Element>,
    #[props(optional)] icon: Option<Element>,
) -> Element {
    rsx! {
        div { class: "flex items-center gap-3 px-4 pt-4 pb-2",
            if let Some(i) = icon {
                div { class: "flex size-8 shrink-0 items-center justify-center rounded-xl bg-muted text-foreground", {i} }
            }
            div { class: "min-w-0 flex-1",
                h3 { class: "truncate text-[15px] font-semibold", "{title}" }
                if let Some(s) = &subtitle {
                    p { class: "truncate text-xs text-muted-foreground", "{s}" }
                }
            }
            {action}
        }
    }
}

/// Kennzahl mit Beschriftung.
#[component]
pub fn Stat(
    #[props(into)] label: String,
    #[props(into)] value: String,
    #[props(into, optional)] unit: Option<String>,
    #[props(into, optional)] class: Option<String>,
) -> Element {
    let merged = tw_merge!("flex min-w-0 flex-col", class.as_deref().unwrap_or(""));
    rsx! {
        div { class: "{merged}",
            span { class: "truncate text-[11px] font-medium uppercase tracking-wide text-muted-foreground", "{label}" }
            span { class: "truncate text-lg font-semibold tabular-nums",
                "{value}"
                if let Some(u) = &unit {
                    span { class: "ml-0.5 text-xs font-normal text-muted-foreground", "{u}" }
                }
            }
        }
    }
}

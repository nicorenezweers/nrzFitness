use dioxus::prelude::*;
use tw_merge::tw_merge;

use crate::components::layout::profile_switcher::ProfileAvatarButton;

/// Seitenkopf: grosser Titel (iOS-Stil), Aktionen rechts. Auf dem Handy mit Profil-Avatar.
#[component]
pub fn PageHeader(
    #[props(into)] title: String,
    #[props(into, optional)] subtitle: Option<String>,
    #[props(optional)] actions: Option<Element>,
) -> Element {
    rsx! {
        header { class: "flex items-center gap-3 px-4 pb-3 pt-[calc(env(safe-area-inset-top)+1rem)] sm:px-8 sm:pt-7",
            div { class: "min-w-0 flex-1",
                h1 { class: "truncate text-[1.75rem] font-bold leading-tight tracking-tight sm:text-2xl", "{title}" }
                if let Some(s) = &subtitle {
                    p { class: "mt-0.5 truncate text-sm text-muted-foreground", "{s}" }
                }
            }
            if let Some(a) = actions {
                div { class: "flex items-center gap-2", {a} }
            }
            div { class: "lg:hidden", ProfileAvatarButton {} }
        }
    }
}

/// Inhaltsbereich einer Seite (Breite, Abstände).
#[component]
pub fn PageBody(#[props(into, optional)] class: Option<String>, children: Element) -> Element {
    let merged = tw_merge!("flex flex-col gap-4 px-4 pb-8 sm:px-8", class.as_deref().unwrap_or(""));
    rsx! { div { class: "{merged}", {children} } }
}

/// Sektion mit Überschrift und optionalem Zähler/Aktion.
#[component]
pub fn Section(
    #[props(into)] title: String,
    #[props(optional)] count: Option<usize>,
    #[props(optional)] action: Option<Element>,
    #[props(into, optional)] class: Option<String>,
    children: Element,
) -> Element {
    let merged = tw_merge!("flex flex-col gap-2", class.as_deref().unwrap_or(""));
    rsx! {
        section { class: "{merged}",
            div { class: "flex items-center gap-2 px-1",
                h2 { class: "flex-1 text-xs font-semibold uppercase tracking-wide text-muted-foreground",
                    "{title}"
                    if let Some(c) = count {
                        span { class: "ml-2 rounded bg-muted px-1.5 py-px text-[10px] font-medium normal-case", "{c}" }
                    }
                }
                {action}
            }
            {children}
        }
    }
}

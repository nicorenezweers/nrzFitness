use dioxus::prelude::*;

/// Gemeinsame Klassen für native Formularelemente (im nrz-Stil, mobil gross genug zum Tippen).
pub const INPUT: &str = "flex h-11 w-full rounded-xl border border-input bg-transparent px-3 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 dark:[color-scheme:dark] sm:h-10";
pub const TEXTAREA: &str = "flex min-h-24 w-full rounded-xl border border-input bg-transparent px-3 py-2 text-sm shadow-sm placeholder:text-muted-foreground focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60";
pub const SELECT: &str = "flex h-11 w-full rounded-xl border border-input bg-background px-3 text-sm shadow-sm focus-visible:outline-none focus-visible:ring-2 focus-visible:ring-ring/60 sm:h-10";
/// Unsichtbares Input (z.B. Titel inline bearbeiten).
pub const GHOST_INPUT: &str = "w-full bg-transparent outline-none placeholder:text-muted-foreground";

#[component]
pub fn Field(#[props(into)] label: String, #[props(into, optional)] hint: Option<String>, children: Element) -> Element {
    rsx! {
        label { class: "flex min-w-0 flex-col gap-1.5",
            span { class: "text-xs font-medium text-muted-foreground", "{label}" }
            {children}
            if let Some(h) = &hint {
                span { class: "text-[11px] text-muted-foreground", "{h}" }
            }
        }
    }
}

/// Zahlenfeld mit Einheit rechts (Dezimaltastatur auf dem Handy). Wert als Text (Komma erlaubt).
#[component]
pub fn NumberInput(
    value: String,
    oninput: EventHandler<String>,
    #[props(into, optional)] unit: Option<String>,
    #[props(into, optional)] placeholder: Option<String>,
    #[props(default = false)] integer: bool,
    #[props(default = false)] autofocus: bool,
    #[props(into, optional)] class: Option<String>,
) -> Element {
    rsx! {
        div { class: "relative {class.clone().unwrap_or_default()}",
            input {
                class: "{INPUT} tabular-nums",
                class: if unit.is_some() { "pr-12" },
                r#type: "text",
                inputmode: if integer { "numeric" } else { "decimal" },
                autocomplete: "off",
                placeholder: placeholder.clone().unwrap_or_default(),
                value: "{value}",
                onmounted: move |e| async move { if autofocus { let _ = e.set_focus(true).await; } },
                oninput: move |e| oninput.call(e.value()),
            }
            if let Some(u) = &unit {
                span { class: "pointer-events-none absolute inset-y-0 right-3 flex items-center text-xs text-muted-foreground", "{u}" }
            }
        }
    }
}

/// Bewertung 1..5 als Emoji-/Zahlen-Knöpfe.
#[component]
pub fn Scale5(value: Option<i64>, onchange: EventHandler<Option<i64>>, #[props(default)] labels: Vec<String>) -> Element {
    let labels = if labels.len() == 5 { labels } else { ["1", "2", "3", "4", "5"].iter().map(|s| s.to_string()).collect() };
    rsx! {
        div { class: "grid grid-cols-5 gap-1.5",
            for (i, l) in labels.into_iter().enumerate() {
                {
                    let n = i as i64 + 1;
                    let active = value == Some(n);
                    rsx! {
                        button {
                            r#type: "button",
                            key: "{n}",
                            class: if active { "h-11 rounded-xl bg-primary text-lg text-primary-foreground transition-transform active:scale-95" } else { "h-11 rounded-xl bg-muted text-lg transition-transform hover:bg-accent active:scale-95" },
                            onclick: move |_| onchange.call(if active { None } else { Some(n) }),
                            "{l}"
                        }
                    }
                }
            }
        }
    }
}

/// Umschalter (Switch).
#[component]
pub fn Switch(checked: bool, onchange: EventHandler<bool>) -> Element {
    rsx! {
        button {
            r#type: "button",
            role: "switch",
            class: if checked { "relative h-7 w-12 shrink-0 rounded-full bg-primary transition-colors" } else { "relative h-7 w-12 shrink-0 rounded-full bg-muted transition-colors" },
            onclick: move |_| onchange.call(!checked),
            span {
                class: "absolute top-0.5 size-6 rounded-full bg-background shadow transition-all",
                class: if checked { "left-[22px]" } else { "left-0.5" },
            }
        }
    }
}

use chrono::{Duration, NaiveDate};
use dioxus::prelude::*;
use icons::{ChevronLeft, ChevronRight};

use crate::util::{fmt_relative, fmt_short, parse_date, today};

/// Tageswahl: ‹ Heute ›. Tippen auf das Datum öffnet den nativen Datumswähler.
#[component]
pub fn DateNav(date: NaiveDate, onchange: EventHandler<NaiveDate>) -> Element {
    let is_today = date == today();
    let label = fmt_relative(date);
    let sub = if (date - today()).num_days().abs() <= 6 && !is_today { fmt_short(date) } else if is_today { fmt_short(date) } else { String::new() };
    rsx! {
        div { class: "flex items-center gap-1",
            button {
                r#type: "button",
                class: "flex size-10 items-center justify-center rounded-full text-muted-foreground transition-transform hover:bg-accent active:scale-90",
                onclick: move |_| onchange.call(date - Duration::days(1)),
                ChevronLeft { class: "size-5" }
            }
            label { class: "relative flex min-w-[7.5rem] cursor-pointer flex-col items-center rounded-xl px-2 py-1 leading-tight hover:bg-accent/60",
                span { class: "text-sm font-semibold", "{label}" }
                if !sub.is_empty() {
                    span { class: "text-[11px] text-muted-foreground", "{sub}" }
                }
                input {
                    r#type: "date",
                    class: "absolute inset-0 cursor-pointer opacity-0",
                    value: "{date.format(\"%Y-%m-%d\")}",
                    onchange: move |e| if let Some(d) = parse_date(&e.value()) { onchange.call(d) },
                }
            }
            button {
                r#type: "button",
                class: "flex size-10 items-center justify-center rounded-full text-muted-foreground transition-transform hover:bg-accent active:scale-90 disabled:opacity-30",
                disabled: is_today,
                onclick: move |_| onchange.call(date + Duration::days(1)),
                ChevronRight { class: "size-5" }
            }
        }
    }
}

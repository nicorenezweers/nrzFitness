//! Schnelles Gewicht-Eintragen: Sheet mit vorausgefülltem letzten Wert, ±-Knöpfen und einem Tap zum Speichern.

use chrono::NaiveDate;
use dioxus::prelude::*;
use icons::{Minus, Plus};

use crate::api::body::quick_weight;
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::field::NumberInput;
use crate::components::ui::sheet::Sheet;
use crate::state::use_app;
use crate::util::{fmt_relative, num, parse_num};

/// "+0,2" / "-0,5"
pub fn signed(d: f64) -> String {
    if d > 0.0 { format!("+{}", num(d, 1)) } else { num(d, 1) }
}

#[component]
pub fn QuickWeightSheet(date: NaiveDate, initial: Option<f64>, onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let mut value = use_signal(|| initial.map(|w| num(w, 1)).unwrap_or_default());
    let mut busy = use_signal(|| false);
    let mut step = move |d: f64| {
        let cur = parse_num(&value()).or(initial).unwrap_or(70.0);
        value.set(num(((cur + d) * 10.0).round() / 10.0, 1));
    };
    let parsed = parse_num(&value());
    let valid = parsed.is_some_and(|w| (20.0..=400.0).contains(&w));
    let mut save = move || {
        let Some(w) = parse_num(&value()).filter(|w| (20.0..=400.0).contains(w)) else { return };
        if busy() {
            return;
        }
        busy.set(true);
        let pid = app.pid();
        spawn(async move {
            match quick_weight(pid, date, w).await {
                Ok(()) => {
                    app.toast(format!("{} kg gespeichert", num(w, 1)));
                    app.bump();
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };
    let round = "flex size-14 shrink-0 items-center justify-center rounded-full bg-muted text-foreground transition-transform active:scale-90";
    rsx! {
        Sheet {
            title: "Gewicht · {fmt_relative(date)}",
            onclose: move |_| onclose.call(()),
            footer: rsx! {
                Button { size: ButtonSize::Lg, class: "w-full", disabled: !valid || busy(), onclick: move |_| save(),
                    if busy() { "Speichern …" } else { "Speichern" }
                }
            },
            form {
                class: "flex flex-col items-center gap-4 px-5 pb-4 pt-2",
                onsubmit: move |e| {
                    e.prevent_default();
                    save();
                },
                div { class: "flex w-full items-center gap-3",
                    button { r#type: "button", class: round, onclick: move |_| step(-0.1), Minus { class: "size-6" } }
                    NumberInput {
                        value: value(),
                        oninput: move |v| value.set(v),
                        unit: "kg",
                        autofocus: true,
                        class: "flex-1 [&_input]:h-16 [&_input]:text-center [&_input]:text-3xl! [&_input]:font-bold",
                    }
                    button { r#type: "button", class: round, onclick: move |_| step(0.1), Plus { class: "size-6" } }
                }
                if let Some(prev) = initial {
                    p { class: "text-sm text-muted-foreground",
                        "Zuletzt {num(prev, 1)} kg"
                        if let Some(p) = parsed.filter(|_| valid) {
                            {
                                let d = p - prev;
                                let cls = if d.abs() < 0.05 { "text-muted-foreground" } else if d < 0.0 { "text-good" } else { "text-warn" };
                                let txt = signed(d);
                                rsx! { span { class: "ml-1.5 font-medium {cls}", "({txt} kg)" } }
                            }
                        }
                    }
                }
                div { class: "grid w-full grid-cols-4 gap-2",
                    for d in [-0.5, -0.2, 0.2, 0.5] {
                        button {
                            r#type: "button",
                            key: "{d}",
                            class: "h-11 rounded-xl border text-sm font-medium tabular-nums transition-transform active:scale-95",
                            onclick: move |_| step(d),
                            "{signed(d)}"
                        }
                    }
                }
            }
        }
    }
}

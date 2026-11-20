//! Ruhe-Timer: schwebende Pille über der Tab-Leiste nach einem abgehakten Satz.

use dioxus::prelude::*;
use icons::{Timer, X};

use super::page_training::{now_ms, use_training};

#[component]
pub fn RestTimer() -> Element {
    let t = use_training();
    if (t.rest_end)().is_none() {
        return rsx! {};
    }
    rsx! { RestPill {} }
}

#[component]
fn RestPill() -> Element {
    let mut t = use_training();
    let mut now = use_signal(now_ms);
    let mut buzzed = use_signal(|| false);
    use_future(move || async move {
        loop {
            let _ = document::eval("await new Promise(r => setTimeout(r, 1000));").await;
            let n = now_ms();
            now.set(n);
            let Some(end) = *t.rest_end.peek() else { break };
            if n >= end && !*buzzed.peek() {
                buzzed.set(true);
                let _ = document::eval("try { navigator.vibrate && navigator.vibrate([200, 100, 200]); } catch (e) {}");
            }
            if n >= end + 4000 {
                t.rest_end.set(None);
                break;
            }
        }
    });
    let end = (t.rest_end)().unwrap_or_default();
    let remaining = ((end - now()) as f64 / 1000.0).ceil().max(0.0) as i64;
    let total = (t.rest_len)().max(1);
    let pct = (remaining as f64 / total as f64).clamp(0.0, 1.0) * 100.0;
    let over = remaining == 0;
    let label = format!("{}:{:02}", remaining / 60, remaining % 60);
    let btn = "flex h-10 min-w-11 items-center justify-center rounded-full px-2 text-sm font-semibold tabular-nums transition-transform active:scale-90 hover:bg-background/15";
    rsx! {
        div { class: "anim-pop fixed left-1/2 z-[300] -translate-x-1/2 bottom-[calc(env(safe-area-inset-bottom)+5.25rem)] lg:bottom-6",
            div {
                class: "relative flex items-center gap-0.5 overflow-hidden rounded-full p-1 shadow-2xl shadow-black/40",
                class: if over { "bg-good text-background" } else { "bg-foreground text-background" },
                if !over {
                    button { r#type: "button", class: "{btn}", aria_label: "15 Sekunden weniger", onclick: move |_| t.adjust_rest(-15), "−15" }
                }
                div { class: "flex items-center gap-2 px-2",
                    Timer { class: "size-4 opacity-80" }
                    if over {
                        span { class: "text-sm font-semibold", "Pause vorbei – weiter!" }
                    } else {
                        span { class: "min-w-[3.2rem] text-center text-lg font-bold tabular-nums", "{label}" }
                    }
                }
                if !over {
                    button { r#type: "button", class: "{btn}", aria_label: "15 Sekunden mehr", onclick: move |_| t.adjust_rest(15), "+15" }
                }
                button {
                    r#type: "button",
                    class: "{btn}",
                    aria_label: "Pause beenden",
                    onclick: move |_| t.rest_end.set(None),
                    X { class: "size-4" }
                }
                if !over {
                    div { class: "absolute inset-x-3 bottom-0 h-0.5 rounded-full bg-background/20",
                        div { class: "h-full rounded-full bg-background/70 transition-all duration-1000 ease-linear", style: "width: {pct}%" }
                    }
                }
            }
        }
    }
}

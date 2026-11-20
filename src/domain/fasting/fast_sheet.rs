//! Fastenzeit nachträglich bearbeiten (Start, Ende, Ziel, Notiz) oder löschen.

use dioxus::prelude::*;
use icons::Trash2;

use crate::api::fasting::{delete_fast, update_fast};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, INPUT, NumberInput, TEXTAREA};
use crate::components::ui::sheet::Sheet;
use crate::model::{Fast, PRESETS, preset_label};
use crate::state::use_app;
use crate::util::{datetime_input, fmt_duration_min, num, parse_datetime, parse_num};

#[component]
pub fn FastSheet(fast: Fast, onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let running = fast.end_at.is_none();
    let mut start = use_signal(|| datetime_input(fast.start_at));
    let mut end = use_signal(|| fast.end_at.map(datetime_input).unwrap_or_default());
    let mut target = use_signal(|| num(fast.target_hours, 1));
    let mut notes = use_signal(|| fast.notes.clone().unwrap_or_default());
    let mut busy = use_signal(|| false);
    let mut confirm = use_signal(|| false);
    let id = fast.id.clone();

    let s = parse_datetime(&start());
    let e = parse_datetime(&end());
    let t = parse_num(&target());
    let err = if s.is_none() {
        Some("Startzeit fehlt")
    } else if !end().trim().is_empty() && e.is_none() {
        Some("Ende ungültig")
    } else if matches!((s, e), (Some(s), Some(e)) if e <= s) {
        Some("Ende muss nach dem Start liegen")
    } else if !t.is_some_and(|t| (1.0..=168.0).contains(&t)) {
        Some("Ziel zwischen 1 und 168 h")
    } else {
        None
    };
    let dur = match (s, e) {
        (Some(s), Some(e)) if e > s => Some(fmt_duration_min((e - s).num_minutes())),
        _ => None,
    };

    let id_save = id.clone();
    let save = move |_| {
        let (Some(s), Some(t)) = (parse_datetime(&start()), parse_num(&target())) else { return };
        let e = parse_datetime(&end());
        busy.set(true);
        let pid = app.pid();
        let id = id_save.clone();
        let n = Some(notes()).filter(|n| !n.trim().is_empty());
        spawn(async move {
            match update_fast(pid, id, s, e, t, n).await {
                Ok(_) => {
                    app.toast("Fastenzeit gespeichert");
                    app.bump();
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };
    let remove = move |_| {
        if !confirm() {
            confirm.set(true);
            return;
        }
        let pid = app.pid();
        let id = id.clone();
        spawn(async move {
            match delete_fast(pid, id).await {
                Ok(()) => {
                    app.toast("Fastenzeit gelöscht");
                    app.bump();
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
        });
    };

    rsx! {
        Sheet {
            title: if running { "Laufende Fastenzeit" } else { "Fastenzeit bearbeiten" },
            onclose: move |_| onclose.call(()),
            footer: rsx! {
                div { class: "flex gap-2",
                    Button {
                        variant: if confirm() { ButtonVariant::Destructive } else { ButtonVariant::Outline },
                        size: ButtonSize::Lg,
                        class: if confirm() { "px-4" } else { "w-12 px-0" },
                        onclick: remove,
                        Trash2 { class: "size-4" }
                        if confirm() { "Löschen?" }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", disabled: err.is_some() || busy(), onclick: save,
                        if busy() { "Speichern …" } else { "Speichern" }
                    }
                }
            },
            div { class: "flex flex-col gap-4 px-5 pb-4 pt-2",
                div { class: "grid grid-cols-1 gap-3 min-[400px]:grid-cols-2",
                    Field { label: "Start",
                        input { class: INPUT, r#type: "datetime-local", value: "{start}", oninput: move |e| start.set(e.value()) }
                    }
                    Field { label: if running { "Ende (leer = läuft noch)" } else { "Ende" },
                        input { class: INPUT, r#type: "datetime-local", value: "{end}", oninput: move |e| end.set(e.value()) }
                    }
                }
                if let Some(d) = dur {
                    p { class: "-mt-1 text-sm text-muted-foreground", "Dauer: " span { class: "font-medium text-foreground", "{d}" } }
                }
                Field { label: "Ziel",
                    NumberInput { value: target(), oninput: move |v| target.set(v), unit: "h" }
                }
                div { class: "no-scrollbar -mx-5 -mt-2 flex gap-2 overflow-x-auto px-5",
                    for h in PRESETS {
                        button {
                            r#type: "button",
                            key: "{h}",
                            class: if t == Some(h) { "shrink-0 rounded-full bg-primary px-3 py-1.5 text-sm font-medium text-primary-foreground" } else { "shrink-0 rounded-full border px-3 py-1.5 text-sm text-muted-foreground" },
                            onclick: move |_| target.set(num(h, 0)),
                            "{preset_label(h)}"
                        }
                    }
                }
                Field { label: "Notiz",
                    textarea { class: "{TEXTAREA} min-h-20", placeholder: "Wie lief es?", value: "{notes}", oninput: move |e| notes.set(e.value()) }
                }
                if let Some(e) = err {
                    p { class: "text-sm text-bad", "{e}" }
                }
            }
        }
    }
}

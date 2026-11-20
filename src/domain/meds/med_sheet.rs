//! Medikament anlegen/bearbeiten (Name, Dosis, mehrere Einnahmezeiten, Notizen, aktiv) oder löschen.

use dioxus::prelude::*;
use icons::{Plus, Trash2, X};

use crate::api::meds::{delete_medication, save_medication};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, INPUT, Switch, TEXTAREA};
use crate::components::ui::sheet::Sheet;
use crate::model::{Medication, MedicationInput, parse_times, slot_label};
use crate::state::use_app;

const PRESET_TIMES: [&str; 4] = ["08:00", "12:00", "18:00", "22:00"];

#[component]
pub fn MedSheet(med: Option<Medication>, onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let editing = med.clone();
    let is_new = med.is_none();
    let mut name = use_signal(|| med.as_ref().map(|m| m.name.clone()).unwrap_or_default());
    let mut dose = use_signal(|| med.as_ref().and_then(|m| m.dose.clone()).unwrap_or_default());
    let mut times = use_signal(|| med.as_ref().map(|m| m.slots()).unwrap_or_else(|| vec!["08:00".to_string()]));
    let mut notes = use_signal(|| med.as_ref().and_then(|m| m.notes.clone()).unwrap_or_default());
    let mut active = use_signal(|| med.as_ref().is_none_or(|m| m.active));
    let mut busy = use_signal(|| false);
    let mut confirm = use_signal(|| false);

    let valid_times = parse_times(&times().join(","));
    let can_save = !name().trim().is_empty() && !valid_times.is_empty() && !busy();
    let id = editing.as_ref().map(|m| m.id.clone());
    let id_save = id.clone();

    let save = move |_| {
        if !can_save {
            return;
        }
        busy.set(true);
        let pid = app.pid();
        let input = MedicationInput {
            name: name(),
            dose: Some(dose()),
            times: times(),
            notes: Some(notes()),
            active: active(),
        };
        let id = id_save.clone();
        spawn(async move {
            match save_medication(pid, id.clone(), input).await {
                Ok(m) => {
                    app.toast(if id.is_some() {
                        format!("{} gespeichert", m.name)
                    } else {
                        format!("{} hinzugefügt", m.name)
                    });
                    app.bump();
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };
    let remove = move |_| {
        let Some(id) = id.clone() else { return };
        if !confirm() {
            confirm.set(true);
            return;
        }
        let pid = app.pid();
        spawn(async move {
            match delete_medication(pid, id).await {
                Ok(()) => {
                    app.toast("Medikament gelöscht");
                    app.bump();
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
        });
    };

    rsx! {
        Sheet {
            title: if editing.is_some() { "Medikament bearbeiten" } else { "Neues Medikament" },
            onclose: move |_| onclose.call(()),
            footer: rsx! {
                div { class: "flex gap-2",
                    if editing.is_some() {
                        Button {
                            variant: if confirm() { ButtonVariant::Destructive } else { ButtonVariant::Outline },
                            size: ButtonSize::Lg,
                            class: if confirm() { "px-4" } else { "w-12 px-0" },
                            onclick: remove,
                            Trash2 { class: "size-4" }
                            if confirm() { "Löschen?" }
                        }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", disabled: !can_save, onclick: save,
                        if busy() { "Speichern …" } else { "Speichern" }
                    }
                }
            },
            div { class: "flex flex-col gap-4 px-5 pb-4 pt-2",
                Field { label: "Name",
                    input {
                        class: INPUT,
                        placeholder: "z.B. Vitamin D",
                        value: "{name}",
                        autocomplete: "off",
                        onmounted: move |e| async move { if is_new { let _ = e.set_focus(true).await; } },
                        oninput: move |e| name.set(e.value()),
                    }
                }
                Field { label: "Dosis",
                    input { class: INPUT, placeholder: "z.B. 1 Tablette, 1000 IE", value: "{dose}", oninput: move |e| dose.set(e.value()) }
                }
                div { class: "flex flex-col gap-2",
                    span { class: "text-xs font-medium text-muted-foreground", "Einnahmezeiten" }
                    for (i, t) in times().into_iter().enumerate() {
                        div { key: "{i}", class: "flex items-center gap-2",
                            input {
                                class: "{INPUT} flex-1",
                                r#type: "time",
                                value: "{t}",
                                oninput: move |e| times.with_mut(|v| if let Some(x) = v.get_mut(i) { *x = e.value() }),
                            }
                            span { class: "w-24 shrink-0 text-xs text-muted-foreground", "{slot_label(&t)}" }
                            button {
                                r#type: "button",
                                class: "flex size-11 shrink-0 items-center justify-center rounded-full text-muted-foreground hover:bg-accent disabled:opacity-30",
                                disabled: times().len() <= 1,
                                onclick: move |_| times.with_mut(|v| { if v.len() > 1 { v.remove(i); } }),
                                X { class: "size-4" }
                            }
                        }
                    }
                    div { class: "flex flex-wrap gap-2",
                        for p in PRESET_TIMES {
                            if !times().iter().any(|t| t == p) {
                                button {
                                    r#type: "button",
                                    key: "{p}",
                                    class: "flex h-9 items-center gap-1 rounded-full border px-3 text-sm text-muted-foreground hover:text-foreground",
                                    onclick: move |_| times.with_mut(|v| { v.push(p.to_string()); v.sort(); }),
                                    Plus { class: "size-3.5" }
                                    "{p}"
                                }
                            }
                        }
                    }
                }
                Field { label: "Notizen",
                    textarea { class: "{TEXTAREA} min-h-20", placeholder: "z.B. zum Essen einnehmen", value: "{notes}", oninput: move |e| notes.set(e.value()) }
                }
                div { class: "flex items-center justify-between gap-3 rounded-xl bg-muted/60 px-3 py-3",
                    div {
                        p { class: "text-sm font-medium", "Aktiv" }
                        p { class: "text-xs text-muted-foreground", "Pausierte Medikamente erscheinen nicht in der Checkliste" }
                    }
                    Switch { checked: active(), onchange: move |v| active.set(v) }
                }
            }
        }
    }
}

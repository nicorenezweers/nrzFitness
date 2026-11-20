//! Eigene Übung anlegen / bearbeiten / löschen.

use dioxus::prelude::*;
use icons::Trash2;

use crate::api::training::{create_exercise, delete_exercise, update_exercise};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, INPUT, NumberInput, SELECT};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::sheet::Sheet;
use crate::model::{EQUIPMENT, Exercise, ExerciseInput, MUSCLES};
use crate::state::use_app;
use crate::util::{num, parse_num};

fn default_met(cat: &str) -> f64 {
    match cat {
        "cardio" => 7.0,
        "flexibility" => 2.5,
        "sport" => 6.0,
        _ => 5.0,
    }
}

#[component]
pub fn ExerciseForm(
    #[props(!optional)] initial: Option<Exercise>,
    #[props(into, default)] preset_name: String,
    #[props(into, default = "strength".to_string())] preset_category: String,
    onclose: EventHandler<()>,
    onsaved: EventHandler<Exercise>,
    #[props(optional)] ondeleted: Option<EventHandler<()>>,
) -> Element {
    let mut app = use_app();
    let editing = initial.clone();
    let init = initial.clone();
    let mut name = use_signal(|| init.as_ref().map(|e| e.name.clone()).unwrap_or(preset_name.clone()));
    let mut category = use_signal(|| init.as_ref().map(|e| e.category.clone()).unwrap_or(preset_category.clone()));
    let mut muscle = use_signal(|| init.as_ref().and_then(|e| e.muscle.clone()).unwrap_or_default());
    let mut equipment = use_signal(|| init.as_ref().and_then(|e| e.equipment.clone()).unwrap_or_default());
    let mut met = use_signal(|| num(init.as_ref().map(|e| e.met).unwrap_or_else(|| default_met(&preset_category)), 1));
    let mut met_touched = use_signal(|| init.is_some());
    let mut busy = use_signal(|| false);
    let mut confirm = use_signal(|| false);

    let cats: Vec<(String, String)> = super::shared::category_options(false);
    let edit_id = editing.as_ref().map(|e| e.id.clone());
    let edit_id2 = edit_id.clone();

    let save = move |_| {
        let id = edit_id.clone();
        async move {
            if name().trim().is_empty() {
                app.error("Bitte einen Namen eingeben");
                return;
            }
            busy.set(true);
            let input = ExerciseInput {
                name: name().trim().to_string(),
                category: category(),
                muscle: Some(muscle()).filter(|m| !m.is_empty()),
                equipment: Some(equipment()).filter(|m| !m.is_empty()),
                met: parse_num(&met()).unwrap_or(5.0),
            };
            let pid = app.pid();
            let res = match id {
                Some(id) => update_exercise(pid, id, input).await,
                None => create_exercise(pid, input).await,
            };
            busy.set(false);
            match res {
                Ok(ex) => {
                    app.toast("Übung gespeichert");
                    onsaved.call(ex);
                }
                Err(e) => app.error(e),
            }
        }
    };

    let delete = move |_| {
        let id = edit_id2.clone();
        async move {
            let Some(id) = id else { return };
            if !confirm() {
                confirm.set(true);
                return;
            }
            match delete_exercise(app.pid(), id).await {
                Ok(()) => {
                    app.toast("Übung gelöscht");
                    app.bump();
                    if let Some(h) = &ondeleted {
                        h.call(());
                    }
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
        }
    };

    let title = if editing.is_some() { "Übung bearbeiten" } else { "Eigene Übung" };
    rsx! {
        Sheet {
            title,
            onclose: move |_| onclose.call(()),
            footer: rsx! {
                div { class: "flex gap-2",
                    if editing.is_some() {
                        Button {
                            variant: if confirm() { ButtonVariant::Destructive } else { ButtonVariant::Outline },
                            size: ButtonSize::Lg,
                            class: if confirm() { "" } else { "w-12 px-0" },
                            onclick: delete,
                            Trash2 { class: "size-5" }
                            if confirm() { "Löschen?" }
                        }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", disabled: busy(), onclick: save, "Speichern" }
                }
            },
            div { class: "flex flex-col gap-4 px-5 pb-5 pt-2",
                Field { label: "Name",
                    input {
                        class: "{INPUT}",
                        value: "{name}",
                        placeholder: "z.B. Kniebeugen an der Multipresse",
                        oninput: move |e| name.set(e.value()),
                    }
                }
                div { class: "flex flex-col gap-1.5",
                    span { class: "text-xs font-medium text-muted-foreground", "Kategorie" }
                    Segmented {
                        options: cats,
                        value: category(),
                        onchange: move |v: String| {
                            if !met_touched() {
                                met.set(num(default_met(&v), 1));
                            }
                            category.set(v);
                        },
                    }
                }
                div { class: "grid grid-cols-2 gap-3",
                    Field { label: "Muskelgruppe",
                        select { class: "{SELECT}", value: "{muscle}", onchange: move |e| muscle.set(e.value()),
                            option { value: "", selected: muscle().is_empty(), "–" }
                            for m in MUSCLES {
                                option { key: "{m}", value: "{m}", selected: muscle() == m, "{m}" }
                            }
                        }
                    }
                    Field { label: "Gerät",
                        select { class: "{SELECT}", value: "{equipment}", onchange: move |e| equipment.set(e.value()),
                            option { value: "", selected: equipment().is_empty(), "–" }
                            for m in EQUIPMENT {
                                option { key: "{m}", value: "{m}", selected: equipment() == m, "{m}" }
                            }
                        }
                    }
                }
                Field { label: "Intensität (MET)", hint: "Für die kcal-Schätzung: 2–3 leicht, 4–6 mittel, 7+ intensiv.",
                    NumberInput {
                        value: met(),
                        oninput: move |v| {
                            met_touched.set(true);
                            met.set(v);
                        },
                        unit: "MET",
                    }
                }
            }
        }
    }
}

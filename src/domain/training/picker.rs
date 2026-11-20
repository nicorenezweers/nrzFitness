//! Übung auswählen (Bottom-Sheet): Suche, Kategorie-Chips, „Zuletzt verwendet“.

use dioxus::prelude::*;
use icons::{CirclePlus, Search};

use super::exercise_form::ExerciseForm;
use super::page_training::use_training;
use super::shared::{ExerciseRow, category_options, matches};
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::field::INPUT;
use crate::components::ui::segmented::Chips;
use crate::components::ui::sheet::Sheet;
use crate::model::Exercise;

#[component]
pub fn ExercisePicker(
    #[props(into)] title: String,
    onpick: EventHandler<Exercise>,
    onclose: EventHandler<()>,
) -> Element {
    let mut t = use_training();
    let lib = t.lib();
    let loading = t.library.read().is_none();
    let mut q = use_signal(String::new);
    let mut cat = use_signal(|| "all".to_string());
    let mut creating = use_signal(|| false);

    let ql = q().trim().to_lowercase();
    let c = cat();
    let list: Vec<Exercise> = lib.exercises.iter().filter(|e| matches(e, &ql, &c)).cloned().collect();
    let recent: Vec<Exercise> =
        if ql.is_empty() { lib.recent.iter().filter(|e| matches(e, "", &c)).cloned().collect() } else { Vec::new() };
    let count = list.len();
    let head = "px-2 pb-1 pt-3 text-xs font-semibold uppercase tracking-wide text-muted-foreground";

    rsx! {
        Sheet { title, tall: true, onclose: move |_| onclose.call(()),
            div { class: "sticky top-0 z-10 flex flex-col gap-3 bg-popover px-4 pb-3 pt-1",
                div { class: "relative",
                    Search { class: "pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" }
                    input {
                        class: "{INPUT} pl-10",
                        r#type: "search",
                        placeholder: "Übung, Muskel oder Gerät suchen …",
                        autocomplete: "off",
                        enterkeyhint: "search",
                        value: "{q}",
                        oninput: move |e| q.set(e.value()),
                    }
                }
                Chips { options: category_options(true), value: c.clone(), onchange: move |v| cat.set(v) }
            }
            div { class: "px-2 pb-6",
                if loading {
                    div { class: "p-6 text-center text-sm text-muted-foreground", "Lädt …" }
                }
                if !recent.is_empty() {
                    p { class: "{head}", "Zuletzt verwendet" }
                    for ex in recent {
                        ExerciseRow { key: "r-{ex.id}", exercise: ex.clone(), onclick: move |_| onpick.call(ex.clone()) }
                    }
                    p { class: "{head}", "Alle Übungen" }
                } else if !ql.is_empty() && !loading {
                    p { class: "{head}", "{count} Treffer" }
                }
                for ex in list {
                    ExerciseRow { key: "{ex.id}", exercise: ex.clone(), onclick: move |_| onpick.call(ex.clone()) }
                }
                if count == 0 && !loading {
                    Empty { class: "mx-2 mt-2",
                        EmptyTitle { "Keine Übung gefunden" }
                        EmptyDescription { "Lege sie einfach als eigene Übung an." }
                    }
                }
                button {
                    r#type: "button",
                    class: "mt-2 flex min-h-14 w-full items-center gap-3 rounded-xl px-2 text-left text-[15px] font-medium hover:bg-accent active:bg-accent",
                    onclick: move |_| creating.set(true),
                    div { class: "flex size-10 items-center justify-center rounded-xl border border-dashed", CirclePlus { class: "size-5" } }
                    if ql.is_empty() { "Eigene Übung anlegen" } else { "„{q().trim()}“ als eigene Übung anlegen" }
                }
            }
        }
        if creating() {
            ExerciseForm {
                initial: None,
                preset_name: q().trim().to_string(),
                preset_category: if c == "all" { "strength".to_string() } else { c.clone() },
                onclose: move |_| creating.set(false),
                onsaved: move |ex: Exercise| {
                    creating.set(false);
                    t.library.restart();
                    onpick.call(ex);
                },
            }
        }
    }
}

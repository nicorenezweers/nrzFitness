use chrono::NaiveDate;
use dioxus::prelude::*;

use crate::api::diary::add_quick_entry;
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::field::{Field, INPUT, NumberInput};
use crate::components::ui::sheet::Sheet;
use crate::model::{MEALS, Nutrients, meal_label};
use crate::state::use_app;
use crate::util::parse_num;

/// Schnelleintrag: nur Kalorien (und optional Makros), ohne Lebensmittel.
#[component]
pub fn QuickEntrySheet(date: NaiveDate, meal: String, onclose: EventHandler<()>, ondone: EventHandler<()>) -> Element {
    let mut app = use_app();
    let mut name = use_signal(String::new);
    let mut kcal = use_signal(String::new);
    let mut protein = use_signal(String::new);
    let mut carbs = use_signal(String::new);
    let mut fat = use_signal(String::new);
    let mut m = use_signal(|| meal.clone());
    let mut busy = use_signal(|| false);
    let p = |s: Signal<String>| parse_num(&s()).unwrap_or(0.0);
    let est = p(protein) * 4.0 + p(carbs) * 4.0 + p(fat) * 9.0;
    let total = parse_num(&kcal()).unwrap_or(est);

    let save = move |_| {
        let p = |s: Signal<String>| parse_num(&s()).unwrap_or(0.0);
        let n = Nutrients { kcal: total, protein: p(protein), carbs: p(carbs), fat: p(fat), ..Default::default() };
        if n.kcal <= 0.0 {
            app.error("Bitte Kalorien eingeben");
            return;
        }
        busy.set(true);
        let pid = app.pid();
        spawn(async move {
            match add_quick_entry(pid, date, m(), name(), n).await {
                Ok(e) => {
                    app.bump();
                    app.toast(format!("{} kcal → {}", e.kcal.round(), meal_label(&e.meal)));
                    ondone.call(());
                }
                Err(e) => {
                    app.error(e);
                    busy.set(false);
                }
            }
        });
    };

    rsx! {
        Sheet { title: "Schnelleintrag", onclose,
            footer: rsx! {
                Button { size: ButtonSize::Lg, class: "w-full", disabled: busy() || total <= 0.0, onclick: save,
                    "Eintragen · {total.round()} kcal"
                }
            },
            div { class: "flex flex-col gap-4 px-5 pb-4 pt-1",
                Field { label: "Kalorien",
                    NumberInput { value: kcal(), unit: "kcal", autofocus: true, integer: true, placeholder: if est > 0.0 { format!("≈ {}", est.round()) } else { "z.B. 450".to_string() }, oninput: move |v| kcal.set(v) }
                }
                div { class: "grid grid-cols-3 gap-2",
                    Field { label: "Protein", NumberInput { value: protein(), unit: "g", oninput: move |v| protein.set(v) } }
                    Field { label: "Kohlenh.", NumberInput { value: carbs(), unit: "g", oninput: move |v| carbs.set(v) } }
                    Field { label: "Fett", NumberInput { value: fat(), unit: "g", oninput: move |v| fat.set(v) } }
                }
                Field { label: "Bezeichnung",
                    input { class: INPUT, placeholder: "z.B. Pizza im Restaurant", value: name(), oninput: move |e| name.set(e.value()) }
                }
                div { class: "grid grid-cols-4 gap-1.5",
                    for (k, l) in MEALS {
                        button {
                            r#type: "button",
                            key: "{k}",
                            class: if m() == k { "h-10 truncate rounded-xl bg-primary px-1 text-xs font-medium text-primary-foreground" } else { "h-10 truncate rounded-xl bg-muted px-1 text-xs text-muted-foreground" },
                            onclick: move |_| m.set(k.to_string()),
                            "{l}"
                        }
                    }
                }
            }
        }
    }
}

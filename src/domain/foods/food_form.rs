use dioxus::prelude::*;
use icons::Trash2;

use crate::api::foods::{create_food, delete_food, update_food};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, INPUT, NumberInput, Switch};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::sheet::Sheet;
use crate::model::{Food, FoodInput};
use crate::state::use_app;
use crate::util::{num, parse_num};

/// Lebensmittel anlegen oder bearbeiten. `existing` = bearbeiten; `prefill` = Vorgaben (z.B. Barcode).
#[component]
pub fn FoodFormSheet(
    #[props(default)] existing: Option<Food>,
    #[props(default)] prefill: Option<FoodInput>,
    onclose: EventHandler<()>,
    onsaved: EventHandler<Food>,
) -> Element {
    let mut app = use_app();
    let init: FoodInput = existing.as_ref().map(FoodInput::from).or(prefill.clone()).unwrap_or(FoodInput {
        unit: "g".into(),
        serving_size: 100.0,
        ..Default::default()
    });
    let edit_id = existing.as_ref().map(|f| f.id.clone());
    let n0 = |v: f64| if v == 0.0 { String::new() } else { num(v, 2) };

    let mut name = use_signal(|| init.name.clone());
    let mut brand = use_signal(|| init.brand.clone().unwrap_or_default());
    let mut barcode = use_signal(|| init.barcode.clone().unwrap_or_default());
    let mut unit = use_signal(|| if init.unit == "ml" { "ml".to_string() } else { "g".to_string() });
    let mut serving = use_signal(|| num(init.serving_size, 1));
    let mut serving_name = use_signal(|| init.serving_name.clone().unwrap_or_default());
    // Eingabe pro 100 oder pro Portion
    let mut basis = use_signal(|| "100".to_string());
    let mut kcal = use_signal(|| n0(init.kcal));
    let mut protein = use_signal(|| n0(init.protein));
    let mut carbs = use_signal(|| n0(init.carbs));
    let mut fat = use_signal(|| n0(init.fat));
    let mut fiber = use_signal(|| n0(init.fiber));
    let mut sugar = use_signal(|| n0(init.sugar));
    let mut sat_fat = use_signal(|| n0(init.sat_fat));
    let mut sodium = use_signal(|| n0(init.sodium_mg));
    let mut shared = use_signal(|| if existing.is_some() { init.shared } else { true });
    let mut busy = use_signal(|| false);

    let source = init.source.clone();
    let image = init.image_url.clone();
    let id_for_save = edit_id.clone();
    let save = move |_| {
        let serving_v = parse_num(&serving()).filter(|v| *v > 0.0).unwrap_or(100.0);
        let f = if basis() == "portion" { 100.0 / serving_v } else { 1.0 };
        let v = |s: Signal<String>| parse_num(&s()).unwrap_or(0.0) * f;
        if name().trim().is_empty() {
            app.error("Bitte einen Namen eingeben");
            return;
        }
        let input = FoodInput {
            name: name(),
            brand: Some(brand()),
            barcode: Some(barcode()),
            unit: unit(),
            serving_size: serving_v,
            serving_name: Some(serving_name()),
            kcal: parse_num(&kcal()).map(|k| k * f).unwrap_or_else(|| v(protein) * 4.0 + v(carbs) * 4.0 + v(fat) * 9.0),
            protein: v(protein),
            carbs: v(carbs),
            fat: v(fat),
            fiber: v(fiber),
            sugar: v(sugar),
            sat_fat: v(sat_fat),
            sodium_mg: v(sodium),
            source: if source.is_empty() { "custom".into() } else { source.clone() },
            image_url: image.clone(),
            shared: shared(),
        };
        busy.set(true);
        let pid = app.pid();
        let id = id_for_save.clone();
        spawn(async move {
            let r = match id {
                Some(id) => update_food(pid, id, input).await,
                None => create_food(pid, input).await,
            };
            match r {
                Ok(f) => {
                    app.bump();
                    onsaved.call(f);
                }
                Err(e) => {
                    app.error(e);
                    busy.set(false);
                }
            }
        });
    };

    // Energie aus Makros schätzen, wenn kcal leer
    let est = parse_num(&protein()).unwrap_or(0.0) * 4.0 + parse_num(&carbs()).unwrap_or(0.0) * 4.0 + parse_num(&fat()).unwrap_or(0.0) * 9.0;
    let u = unit();
    let per = if basis() == "portion" { "Portion".to_string() } else { format!("100 {u}") };
    let del_id = edit_id.clone();

    rsx! {
        Sheet {
            title: if edit_id.is_some() { "Lebensmittel bearbeiten" } else { "Neues Lebensmittel" },
            tall: true,
            onclose,
            footer: rsx! {
                div { class: "flex gap-2",
                    if let Some(id) = del_id.clone() {
                        Button { variant: ButtonVariant::Secondary, size: ButtonSize::Lg, class: "px-4 text-destructive",
                            onclick: move |_| {
                                let (pid, id) = (app.pid(), id.clone());
                                spawn(async move {
                                    match delete_food(pid, id).await {
                                        Ok(()) => { app.bump(); app.toast("Gelöscht"); onclose.call(()); }
                                        Err(e) => app.error(e),
                                    }
                                });
                            },
                            Trash2 { class: "size-5" }
                        }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", disabled: busy(), onclick: save, "Speichern" }
                }
            },
            div { class: "flex flex-col gap-4 px-5 pb-6 pt-1",
                Field { label: "Name",
                    input { class: INPUT, placeholder: "z.B. Haferflocken", value: name(), oninput: move |e| name.set(e.value()) }
                }
                div { class: "grid grid-cols-2 gap-3",
                    Field { label: "Marke",
                        input { class: INPUT, placeholder: "optional", value: brand(), oninput: move |e| brand.set(e.value()) }
                    }
                    Field { label: "Barcode",
                        input { class: INPUT, inputmode: "numeric", placeholder: "optional", value: barcode(), oninput: move |e| barcode.set(e.value()) }
                    }
                }
                div { class: "grid grid-cols-[1fr_1fr] gap-3",
                    Field { label: "Einheit",
                        Segmented { options: vec![("g".into(), "Gramm".into()), ("ml".into(), "Milliliter".into())], value: unit(), onchange: move |v| unit.set(v) }
                    }
                    Field { label: "Portion",
                        NumberInput { value: serving(), unit: u.clone(), oninput: move |v| serving.set(v) }
                    }
                }
                Field { label: "Portionsname", hint: "z.B. „1 Becher“, „1 Scheibe“ – optional",
                    input { class: INPUT, placeholder: "1 Portion", value: serving_name(), oninput: move |e| serving_name.set(e.value()) }
                }
                div { class: "flex flex-col gap-3 rounded-2xl border p-4",
                    div { class: "flex items-center justify-between gap-3",
                        span { class: "text-sm font-semibold", "Nährwerte pro {per}" }
                        Segmented { class: "w-44", options: vec![("100".into(), format!("100 {u}")), ("portion".into(), "Portion".into())], value: basis(), onchange: move |v| basis.set(v) }
                    }
                    Field { label: "Energie",
                        NumberInput { value: kcal(), unit: "kcal", placeholder: if est > 0.0 { format!("≈ {}", est.round()) } else { "0".to_string() }, oninput: move |v| kcal.set(v) }
                    }
                    div { class: "grid grid-cols-3 gap-2",
                        Field { label: "Protein", NumberInput { value: protein(), unit: "g", oninput: move |v| protein.set(v) } }
                        Field { label: "Kohlenh.", NumberInput { value: carbs(), unit: "g", oninput: move |v| carbs.set(v) } }
                        Field { label: "Fett", NumberInput { value: fat(), unit: "g", oninput: move |v| fat.set(v) } }
                    }
                    div { class: "grid grid-cols-2 gap-2",
                        Field { label: "davon Zucker", NumberInput { value: sugar(), unit: "g", oninput: move |v| sugar.set(v) } }
                        Field { label: "ges. Fettsäuren", NumberInput { value: sat_fat(), unit: "g", oninput: move |v| sat_fat.set(v) } }
                        Field { label: "Ballaststoffe", NumberInput { value: fiber(), unit: "g", oninput: move |v| fiber.set(v) } }
                        Field { label: "Natrium", NumberInput { value: sodium(), unit: "mg", oninput: move |v| sodium.set(v) } }
                    }
                }
                div { class: "flex items-center justify-between gap-3 rounded-2xl border p-4",
                    div { class: "flex flex-col",
                        span { class: "text-sm font-medium", "Für alle Profile" }
                        span { class: "text-xs text-muted-foreground", "Andere Profile können es auch verwenden" }
                    }
                    Switch { checked: shared(), onchange: move |v| shared.set(v) }
                }
            }
        }
    }
}

use dioxus::prelude::*;
use icons::{Plus, Search, Trash2, X};

use super::food_row::FoodRow;
use crate::api::foods::{delete_meal, save_meal, search_foods};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{Field, INPUT};
use crate::components::ui::sheet::Sheet;
use crate::model::{Food, MealTemplate, Nutrients};
use crate::state::use_app;
use crate::util::{num, parse_num};

/// Mahlzeiten-Vorlage anlegen/bearbeiten: Name + Lebensmittel mit Mengen.
#[component]
pub fn MealEditorSheet(#[props(default)] existing: Option<MealTemplate>, onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let id = existing.as_ref().map(|m| m.id.clone());
    let mut name = use_signal(|| existing.as_ref().map(|m| m.name.clone()).unwrap_or_default());
    let mut items = use_signal(|| {
        existing.as_ref().map(|m| m.items.iter().map(|i| (i.food.clone(), num(i.amount, 1))).collect::<Vec<_>>()).unwrap_or_default()
    });
    let mut picking = use_signal(|| false);
    let mut busy = use_signal(|| false);

    let mut total = Nutrients::default();
    for (f, a) in items.read().iter() {
        total += f.scaled(parse_num(a).unwrap_or(0.0));
    }

    let id_save = id.clone();
    let save = move |_| {
        let list: Vec<(String, f64)> = items.read().iter().map(|(f, a)| (f.id.clone(), parse_num(a).unwrap_or(0.0))).collect();
        if name().trim().is_empty() || list.is_empty() {
            app.error("Name und mindestens ein Lebensmittel nötig");
            return;
        }
        busy.set(true);
        let (pid, id) = (app.pid(), id_save.clone());
        spawn(async move {
            match save_meal(pid, id, name(), None, list).await {
                Ok(_) => {
                    app.bump();
                    app.toast("Mahlzeit gespeichert");
                    onclose.call(());
                }
                Err(e) => {
                    app.error(e);
                    busy.set(false);
                }
            }
        });
    };

    rsx! {
        Sheet {
            title: if id.is_some() { "Mahlzeit bearbeiten" } else { "Neue Mahlzeit" },
            tall: true,
            onclose,
            footer: rsx! {
                div { class: "flex gap-2",
                    if let Some(mid) = id.clone() {
                        Button { variant: ButtonVariant::Secondary, size: ButtonSize::Lg, class: "px-4 text-destructive",
                            onclick: move |_| {
                                let (pid, mid) = (app.pid(), mid.clone());
                                spawn(async move {
                                    match delete_meal(pid, mid).await {
                                        Ok(()) => { app.bump(); onclose.call(()); }
                                        Err(e) => app.error(e),
                                    }
                                });
                            },
                            Trash2 { class: "size-5" }
                        }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", disabled: busy(), onclick: save, "Speichern · {total.kcal.round()} kcal" }
                }
            },
            div { class: "flex flex-col gap-4 px-5 pb-6 pt-1",
                Field { label: "Name",
                    input { class: INPUT, placeholder: "z.B. Müesli mit Joghurt", value: name(), oninput: move |e| name.set(e.value()) }
                }
                div { class: "rounded-2xl bg-muted/60 px-4 py-3 text-sm tabular-nums",
                    span { class: "font-semibold", "{total.kcal.round()} kcal" }
                    span { class: "text-muted-foreground", " · P {num(total.protein, 0)} g · K {num(total.carbs, 0)} g · F {num(total.fat, 0)} g" }
                }
                div { class: "flex flex-col gap-1",
                    for (i, (f, a)) in items.read().clone().into_iter().enumerate() {
                        div { key: "{i}-{f.id}", class: "flex items-center gap-2 rounded-xl border px-3 py-2",
                            span { class: "flex min-w-0 flex-1 flex-col",
                                span { class: "truncate text-sm font-medium", "{f.name}" }
                                span { class: "text-xs text-muted-foreground tabular-nums", "{f.scaled(parse_num(&a).unwrap_or(0.0)).kcal.round()} kcal" }
                            }
                            div { class: "relative w-24",
                                input {
                                    class: "{INPUT} h-10 pr-8 text-right tabular-nums",
                                    inputmode: "decimal",
                                    value: "{a}",
                                    oninput: move |e| { if let Some(it) = items.write().get_mut(i) { it.1 = e.value(); } },
                                }
                                span { class: "pointer-events-none absolute inset-y-0 right-2.5 flex items-center text-xs text-muted-foreground", "{f.unit}" }
                            }
                            button {
                                class: "flex size-9 items-center justify-center rounded-full text-muted-foreground hover:bg-accent",
                                onclick: move |_| { items.write().remove(i); },
                                X { class: "size-4" }
                            }
                        }
                    }
                    button {
                        class: "mt-1 flex items-center justify-center gap-2 rounded-xl border border-dashed py-3 text-sm text-muted-foreground",
                        onclick: move |_| picking.set(true),
                        Plus { class: "size-4" } "Lebensmittel hinzufügen"
                    }
                }
            }
        }
        if picking() {
            FoodPicker {
                onclose: move |_| picking.set(false),
                onpick: move |f: Food| {
                    let a = num(f.serving_size, 1);
                    items.write().push((f, a));
                    picking.set(false);
                },
            }
        }
    }
}

/// Einfache Lebensmittelauswahl (nur eigene Datenbank).
#[component]
pub fn FoodPicker(onclose: EventHandler<()>, onpick: EventHandler<Food>) -> Element {
    let app = use_app();
    let mut q = use_signal(String::new);
    let res = use_resource(move || async move {
        let query = q();
        let pid = app.pid_tracked();
        search_foods(pid, if query.trim().is_empty() { "%".into() } else { query }, false).await.map(|r| r.local).unwrap_or_default()
    });
    rsx! {
        Sheet { title: "Lebensmittel wählen", tall: true, onclose,
            div { class: "sticky top-0 bg-popover px-4 pb-2",
                div { class: "relative",
                    Search { class: "pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" }
                    input {
                        class: "h-12 w-full rounded-2xl bg-muted pl-10 pr-4 text-base outline-none placeholder:text-muted-foreground",
                        r#type: "search",
                        placeholder: "In meinen Lebensmitteln suchen",
                        value: q(),
                        oninput: move |e| q.set(e.value()),
                    }
                }
            }
            div { class: "flex flex-col px-2 pb-4",
                for f in res.read().clone().unwrap_or_default() {
                    {
                        let fc = f.clone();
                        rsx! { FoodRow { key: "{f.id}", food: f, onclick: move |_| onpick.call(fc.clone()) } }
                    }
                }
            }
        }
    }
}

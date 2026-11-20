use dioxus::prelude::*;
use icons::{Plus, ScanBarcode, Search, X};

use super::food_form::FoodFormSheet;
use super::food_row::FoodRow;
use super::meal_editor::MealEditorSheet;
use super::scanner::Scanner;
use crate::api::foods::{all_foods, favorite_foods, list_meals, lookup_barcode};
use crate::components::layout::page::{PageBody, PageHeader};
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::card::Card;
use crate::components::ui::empty::{Empty, EmptyDescription, EmptyTitle};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{Food, FoodInput, MealTemplate};
use crate::state::use_app;
use crate::util::num;

#[derive(Clone, PartialEq)]
enum Open {
    None,
    Food(Option<Food>, Option<FoodInput>),
    Meal(Option<MealTemplate>),
    Scan,
}

/// Lebensmittel-Datenbank und Mahlzeiten-Vorlagen verwalten.
#[component]
pub fn FoodsPage() -> Element {
    let mut app = use_app();
    let mut tab = use_signal(|| "all".to_string());
    let mut q = use_signal(String::new);
    let mut open = use_signal(|| Open::None);

    let foods = use_resource(move || async move {
        let pid = app.pid_tracked();
        let _ = (app.refresh)();
        match tab().as_str() {
            "fav" => favorite_foods(pid).await,
            _ => all_foods(pid).await,
        }
    });
    let meals = use_resource(move || async move {
        let pid = app.pid_tracked();
        let _ = (app.refresh)();
        list_meals(pid).await
    });

    let ql = q().trim().to_lowercase();
    let filtered: Option<Vec<Food>> = match &*foods.read() {
        Some(Ok(list)) => Some(
            list.iter()
                .filter(|f| {
                    ql.is_empty()
                        || f.name.to_lowercase().contains(&ql)
                        || f.brand.as_deref().unwrap_or("").to_lowercase().contains(&ql)
                        || f.barcode.as_deref() == Some(ql.as_str())
                })
                .cloned()
                .collect(),
        ),
        _ => None,
    };

    let on_code = move |code: String| {
        open.set(Open::None);
        let pid = app.pid();
        spawn(async move {
            match lookup_barcode(pid, code.clone()).await {
                Ok(Some(f)) => {
                    app.bump();
                    open.set(Open::Food(Some(f), None));
                }
                Ok(None) => open.set(Open::Food(
                    None,
                    Some(FoodInput { barcode: Some(code), unit: "g".into(), serving_size: 100.0, shared: true, ..Default::default() }),
                )),
                Err(e) => app.error(e),
            }
        });
    };

    let is_meals = tab() == "meals";
    rsx! {
        PageHeader {
            title: "Lebensmittel",
            actions: rsx! {
                button {
                    "aria-label": "Barcode scannen",
                    class: "flex size-10 items-center justify-center rounded-full bg-muted transition-transform active:scale-90",
                    onclick: move |_| open.set(Open::Scan),
                    ScanBarcode { class: "size-5" }
                }
                Button { size: ButtonSize::Icon, class: "lg:hidden",
                    onclick: move |_| open.set(if is_meals { Open::Meal(None) } else { Open::Food(None, None) }),
                    Plus { class: "size-5" }
                }
                Button { class: "hidden lg:inline-flex",
                    onclick: move |_| open.set(if is_meals { Open::Meal(None) } else { Open::Food(None, None) }),
                    Plus { class: "size-4" }
                    if is_meals { "Neue Mahlzeit" } else { "Neues Lebensmittel" }
                }
            },
        }
        PageBody {
            Segmented {
                class: "max-w-md",
                options: vec![("all".into(), "Alle".into()), ("fav".into(), "Favoriten".into()), ("meals".into(), "Mahlzeiten".into())],
                value: tab(),
                onchange: move |v| tab.set(v),
            }
            if is_meals {
                match &*meals.read() {
                    Some(Ok(list)) if list.is_empty() => rsx! {
                        Empty {
                            EmptyTitle { "Noch keine Mahlzeiten" }
                            EmptyDescription { "Fasse Lebensmittel zu einer Mahlzeit zusammen (z.B. dein Frühstück) und trage sie mit einem Tipp ein." }
                            Button { onclick: move |_| open.set(Open::Meal(None)), Plus { class: "size-4" } "Mahlzeit anlegen" }
                        }
                    },
                    Some(Ok(list)) => rsx! {
                        div { class: "grid gap-3 sm:grid-cols-2",
                            for m in list.clone() {
                                {
                                    let t = m.totals();
                                    let mc = m.clone();
                                    let names = m.items.iter().map(|i| i.food.name.clone()).collect::<Vec<_>>().join(", ");
                                    rsx! {
                                        Card { key: "{m.id}", class: "flex flex-col gap-2 p-4", onclick: move |_| open.set(Open::Meal(Some(mc.clone()))),
                                            div { class: "flex items-start gap-3",
                                                span { class: "flex-1 font-semibold", "{m.name}" }
                                                span { class: "text-sm font-semibold tabular-nums", "{t.kcal.round()} kcal" }
                                            }
                                            p { class: "line-clamp-2 text-xs text-muted-foreground", "{names}" }
                                            p { class: "text-xs tabular-nums text-muted-foreground", "P {num(t.protein, 0)} g · K {num(t.carbs, 0)} g · F {num(t.fat, 0)} g" }
                                        }
                                    }
                                }
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { p { class: "text-sm text-destructive", "{e}" } },
                    None => rsx! { Skeleton { class: "h-24 rounded-2xl" } },
                }
            } else {
                div { class: "relative",
                    Search { class: "pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" }
                    input {
                        class: "h-11 w-full rounded-xl bg-muted pl-10 pr-10 text-sm outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring/60",
                        r#type: "search",
                        placeholder: "Filtern nach Name, Marke oder Barcode",
                        value: q(),
                        oninput: move |e| q.set(e.value()),
                    }
                    if !q().is_empty() {
                        button { class: "absolute right-2 top-1/2 flex size-8 -translate-y-1/2 items-center justify-center text-muted-foreground", onclick: move |_| q.set(String::new()), X { class: "size-4" } }
                    }
                }
                match filtered {
                    None => rsx! { div { class: "flex flex-col gap-2", Skeleton { class: "h-14" } Skeleton { class: "h-14" } Skeleton { class: "h-14" } } },
                    Some(list) if list.is_empty() => rsx! {
                        Empty {
                            EmptyTitle { if tab() == "fav" { "Keine Favoriten" } else { "Keine Lebensmittel" } }
                            EmptyDescription { "Lebensmittel landen hier, sobald du sie suchst, scannst oder selbst anlegst." }
                        }
                    },
                    Some(list) => rsx! {
                        p { class: "px-1 text-xs text-muted-foreground", "{list.len()} Lebensmittel" }
                        Card { class: "flex flex-col p-1.5",
                            for f in list {
                                {
                                    let fc = f.clone();
                                    let trailing = format!("{} kcal", f.kcal.round());
                                    rsx! { FoodRow { key: "{f.id}", food: f, trailing, onclick: move |_| open.set(Open::Food(Some(fc.clone()), None)) } }
                                }
                            }
                        }
                        p { class: "px-1 text-center text-[11px] text-muted-foreground", "Werte rechts pro 100 g bzw. 100 ml" }
                    },
                }
            }
        }
        match open() {
            Open::Food(existing, prefill) => rsx! {
                FoodFormSheet { existing, prefill, onclose: move |_| open.set(Open::None), onsaved: move |_| { app.toast("Gespeichert"); open.set(Open::None); } }
            },
            Open::Meal(existing) => rsx! { MealEditorSheet { existing, onclose: move |_| open.set(Open::None) } },
            Open::Scan => rsx! { Scanner { onresult: on_code, onclose: move |_| open.set(Open::None) } },
            Open::None => rsx! {},
        }
    }
}

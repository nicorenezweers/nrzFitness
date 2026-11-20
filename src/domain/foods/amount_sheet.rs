use chrono::NaiveDate;
use dioxus::prelude::*;
use icons::{Minus, Plus, Star, Trash2};

use crate::api::diary::{add_food_entry, delete_food_entry, update_food_entry};
use crate::api::foods::{last_amount, set_favorite};
use crate::api::profile::get_goals;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::ring::Bar;
use crate::components::ui::segmented::Chips;
use crate::components::ui::sheet::Sheet;
use crate::model::{Food, FoodEntry, MEALS, Nutrients, meal_label};
use crate::state::use_app;
use crate::util::{num, parse_num};

#[derive(Clone, PartialEq)]
struct Base {
    title: String,
    subtitle: Option<String>,
    image: Option<String>,
    per100: Nutrients,
    unit: String,
    serving_size: f64,
    serving_name: Option<String>,
}

fn scale(n: &Nutrients, amount: f64) -> Nutrients {
    let f = amount / 100.0;
    Nutrients {
        kcal: n.kcal * f,
        protein: n.protein * f,
        carbs: n.carbs * f,
        fat: n.fat * f,
        fiber: n.fiber * f,
        sugar: n.sugar * f,
        sodium_mg: n.sodium_mg * f,
    }
}

/// Lebensmittel mit Menge eintragen.
#[component]
pub fn AddFoodSheet(food: Food, date: NaiveDate, meal: String, onclose: EventHandler<()>, ondone: EventHandler<()>) -> Element {
    let mut app = use_app();
    let pid = app.pid();
    let fid = food.id.clone();
    let last = use_resource(move || {
        let (pid, fid) = (pid.clone(), fid.clone());
        async move { last_amount(pid, fid).await.ok().flatten() }
    });
    let base = Base {
        title: food.name.clone(),
        subtitle: food.brand.clone(),
        image: food.image_url.clone(),
        per100: food.scaled(100.0),
        unit: food.unit.clone(),
        serving_size: food.serving_size,
        serving_name: food.serving_name.clone(),
    };
    // Erst rendern, wenn die letzte Menge bekannt ist (Vorbelegung).
    let Some(initial) = last.read().clone() else {
        return rsx! { Sheet { title: food.name.clone(), onclose, div { class: "h-64" } } };
    };
    let initial = initial.unwrap_or(food.serving_size);
    let fav = food.favorite;
    let food_id = food.id.clone();
    rsx! {
        AmountSheet {
            base,
            initial_amount: initial,
            initial_meal: meal,
            favorite: Some(fav),
            confirm: "Hinzufügen",
            onclose,
            ontogglefav: {
                let food_id = food_id.clone();
                move |v: bool| {
                    let (pid, id) = (app.pid(), food_id.clone());
                    spawn(async move {
                        if let Err(e) = set_favorite(pid, id, v).await { app.error(e) } else {
                            app.toast(if v { "Zu Favoriten hinzugefügt" } else { "Aus Favoriten entfernt" });
                        }
                    });
                }
            },
            onsave: move |(m, amount): (String, f64)| {
                let (pid, id) = (app.pid(), food_id.clone());
                spawn(async move {
                    match add_food_entry(pid, date, m.clone(), id, amount).await {
                        Ok(e) => {
                            app.bump();
                            app.toast(format!("{} · {} kcal → {}", e.name, e.kcal.round(), meal_label(&m)));
                            ondone.call(());
                        }
                        Err(e) => app.error(e),
                    }
                });
            },
        }
    }
}

/// Bestehenden Tagebuch-Eintrag ändern oder löschen.
#[component]
pub fn EditEntrySheet(entry: FoodEntry, onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let quick = entry.amount <= 0.0;
    let per100 = if quick { entry.nutrients() } else { scale(&entry.nutrients(), 100.0 / entry.amount) };
    let base = Base {
        title: entry.name.clone(),
        subtitle: entry.brand.clone(),
        image: None,
        per100,
        unit: entry.unit.clone(),
        serving_size: if quick { 100.0 } else { entry.amount },
        serving_name: None,
    };
    let id = entry.id.clone();
    let id2 = entry.id.clone();
    rsx! {
        AmountSheet {
            base,
            initial_amount: if quick { 100.0 } else { entry.amount },
            initial_meal: entry.meal.clone(),
            fixed_amount: quick,
            confirm: "Speichern",
            onclose,
            ondelete: move |_| {
                let (pid, id) = (app.pid(), id2.clone());
                spawn(async move {
                    match delete_food_entry(pid, id).await {
                        Ok(()) => { app.bump(); app.toast("Eintrag gelöscht"); onclose.call(()); }
                        Err(e) => app.error(e),
                    }
                });
            },
            onsave: move |(m, amount): (String, f64)| {
                let (pid, id) = (app.pid(), id.clone());
                let amount = if quick { 0.0 } else { amount };
                spawn(async move {
                    match update_food_entry(pid, id, m, amount).await {
                        Ok(_) => { app.bump(); onclose.call(()); }
                        Err(e) => app.error(e),
                    }
                });
            },
        }
    }
}

#[component]
fn AmountSheet(
    base: Base,
    initial_amount: f64,
    initial_meal: String,
    #[props(into)] confirm: String,
    #[props(default = false)] fixed_amount: bool,
    #[props(default)] favorite: Option<bool>,
    onclose: EventHandler<()>,
    onsave: EventHandler<(String, f64)>,
    #[props(optional)] ondelete: Option<EventHandler<()>>,
    #[props(optional)] ontogglefav: Option<EventHandler<bool>>,
) -> Element {
    let app = use_app();
    let mut amount = use_signal(|| num(initial_amount, 1));
    let mut meal = use_signal(|| initial_meal.clone());
    let mut fav = use_signal(|| favorite.unwrap_or(false));
    let mut busy = use_signal(|| false);
    let pid = app.pid();
    let goals = use_resource(move || {
        let pid = pid.clone();
        async move { get_goals(pid).await.ok() }
    });

    let a = parse_num(&amount()).unwrap_or(0.0).max(0.0);
    let n = if fixed_amount { base.per100 } else { scale(&base.per100, a) };
    let unit = base.unit.clone();
    let serving = base.serving_size;
    let serving_label = match base.serving_name.clone() {
        Some(n) => format!("{n} ({} {unit})", num(serving, 0)),
        None => format!("1 Portion ({} {unit})", num(serving, 0)),
    };
    let mut presets: Vec<(String, String)> = vec![(num(serving, 1), serving_label)];
    if serving > 0.0 && (serving - 100.0).abs() > 0.1 {
        presets.push((num(serving * 2.0, 1), format!("2 × {} {unit}", num(serving, 0))));
        presets.push(("100".into(), format!("100 {unit}")));
    }
    for v in [50.0, 150.0, 200.0, 250.0] {
        if (v - serving).abs() > 0.1 {
            presets.push((num(v, 0), format!("{} {unit}", num(v, 0))));
        }
    }
    let step = if serving >= 20.0 { (serving / 2.0).round().max(5.0) } else { 10.0 };
    let goal = goals.read().clone().flatten();
    let pct = goal.as_ref().map(|g| if g.calories > 0.0 { n.kcal / g.calories } else { 0.0 }).unwrap_or(0.0);
    let total_macro_kcal = n.protein * 4.0 + n.carbs * 4.0 + n.fat * 9.0;
    let share = |k: f64| if total_macro_kcal > 0.0 { (k / total_macro_kcal * 100.0).round() } else { 0.0 };

    rsx! {
        Sheet {
            title: base.title.clone(),
            onclose,
            header_action: rsx! {
                if favorite.is_some() {
                    button {
                        r#type: "button",
                        class: "flex size-9 items-center justify-center rounded-full bg-muted transition-transform active:scale-90 sm:size-8",
                        onclick: move |_| {
                            let v = !fav();
                            fav.set(v);
                            if let Some(h) = &ontogglefav { h.call(v) }
                        },
                        Star { class: if fav() { "size-4 fill-current text-carbs" } else { "size-4 text-muted-foreground" } }
                    }
                }
            },
            footer: rsx! {
                div { class: "flex gap-2",
                    if let Some(d) = ondelete {
                        Button { variant: ButtonVariant::Secondary, size: ButtonSize::Lg, class: "px-4 text-destructive", onclick: move |_| d.call(()),
                            Trash2 { class: "size-5" }
                        }
                    }
                    Button {
                        size: ButtonSize::Lg,
                        class: "flex-1",
                        disabled: busy() || (!fixed_amount && a <= 0.0),
                        onclick: move |_| {
                            busy.set(true);
                            onsave.call((meal(), a));
                        },
                        "{confirm} · {n.kcal.round()} kcal"
                    }
                }
            },
            div { class: "flex flex-col gap-5 px-5 pb-4 pt-1",
                if let Some(s) = base.subtitle.as_ref().filter(|s| !s.is_empty()) {
                    p { class: "-mt-2 text-sm text-muted-foreground", "{s}" }
                }
                // Nährwerte
                div { class: "flex items-center gap-4 rounded-2xl bg-muted/60 p-4",
                    div { class: "flex flex-col",
                        span { class: "text-4xl font-bold tabular-nums tracking-tight", "{n.kcal.round()}" }
                        span { class: "text-xs text-muted-foreground", "kcal" }
                    }
                    div { class: "grid flex-1 grid-cols-3 gap-2 text-center",
                        Macro { label: "Protein", value: n.protein, color: "text-protein", share: share(n.protein * 4.0) }
                        Macro { label: "Kohlenh.", value: n.carbs, color: "text-carbs", share: share(n.carbs * 4.0) }
                        Macro { label: "Fett", value: n.fat, color: "text-fat", share: share(n.fat * 9.0) }
                    }
                }
                if goal.is_some() {
                    div { class: "-mt-2 flex flex-col gap-1.5",
                        Bar { value: pct, color: "bg-kcal" }
                        p { class: "text-xs text-muted-foreground", "{(pct * 100.0).round()} % deines Tagesziels" }
                    }
                }
                if !fixed_amount {
                    // Menge
                    div { class: "flex flex-col gap-2.5",
                        span { class: "text-xs font-medium text-muted-foreground", "Menge" }
                        div { class: "flex items-center gap-2",
                            button {
                                r#type: "button",
                                class: "flex size-12 shrink-0 items-center justify-center rounded-2xl bg-muted transition-transform active:scale-90",
                                onclick: move |_| {
                                    let v = (parse_num(&amount()).unwrap_or(0.0) - step).max(0.0);
                                    amount.set(num(v, 1));
                                },
                                Minus { class: "size-5" }
                            }
                            div { class: "relative flex-1",
                                input {
                                    class: "h-12 w-full rounded-2xl border border-input bg-transparent px-4 pr-12 text-center text-2xl font-semibold tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
                                    r#type: "text",
                                    inputmode: "decimal",
                                    value: "{amount}",
                                    onfocus: move |_| { let _ = document::eval("document.activeElement && document.activeElement.select && document.activeElement.select()"); },
                                    oninput: move |e| amount.set(e.value()),
                                }
                                span { class: "pointer-events-none absolute inset-y-0 right-4 flex items-center text-sm text-muted-foreground", "{unit}" }
                            }
                            button {
                                r#type: "button",
                                class: "flex size-12 shrink-0 items-center justify-center rounded-2xl bg-muted transition-transform active:scale-90",
                                onclick: move |_| {
                                    let v = parse_num(&amount()).unwrap_or(0.0) + step;
                                    amount.set(num(v, 1));
                                },
                                Plus { class: "size-5" }
                            }
                        }
                        Chips { options: presets, value: amount(), onchange: move |v| amount.set(v) }
                    }
                }
                // Mahlzeit
                div { class: "flex flex-col gap-2.5",
                    span { class: "text-xs font-medium text-muted-foreground", "Mahlzeit" }
                    div { class: "grid grid-cols-4 gap-1.5",
                        for (k, l) in MEALS {
                            button {
                                r#type: "button",
                                key: "{k}",
                                class: if meal() == k { "h-10 truncate rounded-xl bg-primary px-1 text-xs font-medium text-primary-foreground" } else { "h-10 truncate rounded-xl bg-muted px-1 text-xs text-muted-foreground" },
                                onclick: move |_| meal.set(k.to_string()),
                                "{l}"
                            }
                        }
                    }
                }
                // Details
                div { class: "grid grid-cols-3 gap-2 text-xs",
                    Detail { label: "Ballaststoffe", value: format!("{} g", num(n.fiber, 1)) }
                    Detail { label: "Zucker", value: format!("{} g", num(n.sugar, 1)) }
                    Detail { label: "Natrium", value: format!("{} mg", num(n.sodium_mg, 0)) }
                }
                if !fixed_amount {
                    p { class: "text-center text-[11px] text-muted-foreground",
                        "Pro 100 {unit}: {base.per100.kcal.round()} kcal · P {num(base.per100.protein, 1)} · K {num(base.per100.carbs, 1)} · F {num(base.per100.fat, 1)}"
                    }
                }
            }
        }
    }
}

#[component]
fn Macro(label: &'static str, value: f64, color: &'static str, share: f64) -> Element {
    rsx! {
        div { class: "flex flex-col items-center",
            span { class: "text-lg font-semibold tabular-nums {color}", "{num(value, 1)}" }
            span { class: "text-[11px] text-muted-foreground", "{label} g" }
            span { class: "text-[10px] text-muted-foreground/70", "{share} %" }
        }
    }
}

#[component]
fn Detail(label: &'static str, value: String) -> Element {
    rsx! {
        div { class: "flex flex-col rounded-xl border px-3 py-2",
            span { class: "text-muted-foreground", "{label}" }
            span { class: "font-medium tabular-nums", "{value}" }
        }
    }
}

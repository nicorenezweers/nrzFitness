use dioxus::prelude::*;
use icons::{Globe, Plus, ScanBarcode, Search, Sparkles, X, Zap};

use super::ai_sheet::AiSheet;
use super::amount_sheet::AddFoodSheet;
use super::food_form::FoodFormSheet;
use super::food_row::FoodRow;
use super::quick_entry::QuickEntrySheet;
use super::scanner::Scanner;
use crate::api::diary::{add_food_entry, add_meal_entries};
use crate::api::foods::{all_foods, create_food, favorite_foods, list_meals, lookup_barcode, recent_foods, search_foods};
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::sheet::Sheet;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{Food, FoodInput, MEALS, meal_label};
use crate::state::use_app;
use crate::util::{fmt_relative, num};

#[derive(Clone, PartialEq)]
enum Overlay {
    None,
    Amount(Food),
    Form(Option<FoodInput>),
    Quick,
    Scan,
    Ai,
}

/// Warten im Browser (für Entprellen der Suche).
async fn sleep_ms(ms: u32) {
    let _ = document::eval(&format!("await new Promise(r => setTimeout(r, {ms}));")).await;
}

/// Globales Sheet „Essen eintragen“ (geöffnet über `app.open_food`).
#[component]
pub fn FoodSheet() -> Element {
    let app = use_app();
    let Some(req) = app.food_sheet.read().clone() else { return rsx! {} };
    rsx! { FoodSheetInner { key: "{req.date}-{req.meal}-{req.ai}", date: req.date, meal: req.meal, ai: req.ai } }
}

#[component]
fn FoodSheetInner(date: chrono::NaiveDate, meal: String, ai: bool) -> Element {
    let mut app = use_app();
    let mut meal = use_signal(|| meal);
    let mut query = use_signal(String::new);
    let mut tab = use_signal(|| "recent".to_string());
    let mut overlay = use_signal(|| if ai { Overlay::Ai } else { Overlay::None });
    let mut added = use_signal(|| 0u32);
    let mut online_on = use_signal(|| true);
    let mut busy_code = use_signal(|| false);

    let close = move || {
        let mut app = app;
        app.food_sheet.set(None);
    };

    // Lokale Suche (schnell) und Online-Suche (Open Food Facts, entprellt).
    let local = use_resource(move || async move {
        let q = query();
        let pid = app.pid_tracked();
        let _ = (app.refresh)();
        if q.trim().is_empty() {
            return None;
        }
        sleep_ms(150).await;
        search_foods(pid, q, false).await.ok().map(|r| r.local)
    });
    let online = use_resource(move || async move {
        let q = query();
        let pid = app.pid_tracked();
        if q.trim().chars().count() < 3 || !online_on() {
            return None;
        }
        sleep_ms(650).await;
        Some(search_foods(pid, q, true).await.map(|r| r.online).map_err(crate::state::clean_err))
    });
    let lists = use_resource(move || async move {
        let pid = app.pid_tracked();
        let _ = (app.refresh)();
        match tab().as_str() {
            "fav" => favorite_foods(pid).await.unwrap_or_default(),
            "mine" => all_foods(pid).await.unwrap_or_default().into_iter().filter(|f| f.source == "custom").collect(),
            "recent" => recent_foods(pid).await.unwrap_or_default(),
            _ => Vec::new(),
        }
    });
    let meals = use_resource(move || async move {
        let pid = app.pid_tracked();
        let _ = (app.refresh)();
        if tab() != "meals" {
            return Vec::new();
        }
        list_meals(pid).await.unwrap_or_default()
    });

    let quick_add = move |f: Food| {
        let pid = app.pid();
        let m = meal();
        spawn(async move {
            match add_food_entry(pid, date, m.clone(), f.id.clone(), f.serving_size).await {
                Ok(e) => {
                    app.bump();
                    added += 1;
                    app.toast(format!("{} · {} kcal → {}", e.name, e.kcal.round(), meal_label(&m)));
                }
                Err(e) => app.error(e),
            }
        });
    };
    let pick_online = move |input: FoodInput| {
        let pid = app.pid();
        spawn(async move {
            match create_food(pid, input).await {
                Ok(f) => overlay.set(Overlay::Amount(f)),
                Err(e) => app.error(e),
            }
        });
    };
    let on_code = move |code: String| {
        overlay.set(Overlay::None);
        busy_code.set(true);
        let pid = app.pid();
        spawn(async move {
            let r = lookup_barcode(pid, code.clone()).await;
            busy_code.set(false);
            match r {
                Ok(Some(f)) => overlay.set(Overlay::Amount(f)),
                Ok(None) => {
                    app.toast("Nicht gefunden – bitte Nährwerte erfassen");
                    overlay.set(Overlay::Form(Some(FoodInput {
                        barcode: Some(code),
                        unit: "g".into(),
                        serving_size: 100.0,
                        shared: true,
                        ..Default::default()
                    })));
                }
                Err(e) => app.error(e),
            }
        });
    };

    let q = query();
    let searching = !q.trim().is_empty();
    let date_label = fmt_relative(date);

    rsx! {
        Sheet {
            tall: true,
            onclose: move |_| close(),
            footer: rsx! {
                if added() > 0 {
                    Button { size: ButtonSize::Lg, class: "w-full", onclick: move |_| close(),
                        "Fertig · {added()} hinzugefügt"
                    }
                } else {
                    div { class: "grid grid-cols-2 gap-2",
                        button {
                            class: "flex h-11 items-center justify-center gap-2 rounded-xl bg-muted text-sm font-medium active:scale-[0.98]",
                            onclick: move |_| overlay.set(Overlay::Quick),
                            Zap { class: "size-4" } "Schnelleintrag"
                        }
                        button {
                            class: "flex h-11 items-center justify-center gap-2 rounded-xl bg-muted text-sm font-medium active:scale-[0.98]",
                            onclick: move |_| overlay.set(Overlay::Form(None)),
                            Plus { class: "size-4" } "Neues Lebensmittel"
                        }
                    }
                }
            },
            // Kopf: Mahlzeit + Tag
            div { class: "sticky top-0 z-10 flex flex-col gap-3 bg-popover px-4 pb-3 pt-1 sm:pt-4",
                div { class: "flex items-center gap-2",
                    div { class: "no-scrollbar flex flex-1 gap-1.5 overflow-x-auto",
                        for (k, l) in MEALS {
                            button {
                                key: "{k}",
                                class: if meal() == k { "shrink-0 rounded-full bg-primary px-3.5 py-1.5 text-sm font-semibold text-primary-foreground" } else { "shrink-0 rounded-full bg-muted px-3.5 py-1.5 text-sm text-muted-foreground" },
                                onclick: move |_| meal.set(k.to_string()),
                                "{l}"
                            }
                        }
                    }
                    span { class: "hidden shrink-0 text-xs text-muted-foreground sm:inline", "{date_label}" }
                    button {
                        class: "flex size-9 shrink-0 items-center justify-center rounded-full bg-muted text-muted-foreground",
                        onclick: move |_| close(),
                        X { class: "size-4" }
                    }
                }
                div { class: "flex gap-2",
                    div { class: "relative flex-1",
                        Search { class: "pointer-events-none absolute left-3.5 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" }
                        input {
                            class: "h-12 w-full rounded-2xl bg-muted pl-10 pr-10 text-base outline-none placeholder:text-muted-foreground focus-visible:ring-2 focus-visible:ring-ring/60",
                            r#type: "search",
                            enterkeyhint: "search",
                            autocomplete: "off",
                            placeholder: "Lebensmittel suchen …",
                            value: "{q}",
                            oninput: move |e| query.set(e.value()),
                        }
                        if searching {
                            button {
                                class: "absolute right-2 top-1/2 flex size-8 -translate-y-1/2 items-center justify-center rounded-full text-muted-foreground",
                                onclick: move |_| query.set(String::new()),
                                X { class: "size-4" }
                            }
                        }
                    }
                    button {
                        "aria-label": "Mit KI schätzen (Foto oder Text)",
                        class: "flex size-12 shrink-0 items-center justify-center rounded-2xl bg-muted text-kcal transition-transform active:scale-90",
                        onclick: move |_| overlay.set(Overlay::Ai),
                        Sparkles { class: "size-6" }
                    }
                    button {
                        "aria-label": "Barcode scannen",
                        class: "flex size-12 shrink-0 items-center justify-center rounded-2xl bg-primary text-primary-foreground transition-transform active:scale-90",
                        onclick: move |_| overlay.set(Overlay::Scan),
                        ScanBarcode { class: "size-6" }
                    }
                }
                if !searching {
                    Segmented {
                        options: vec![("recent".into(), "Zuletzt".into()), ("fav".into(), "Favoriten".into()), ("meals".into(), "Mahlzeiten".into()), ("mine".into(), "Eigene".into())],
                        value: tab(),
                        onchange: move |v| tab.set(v),
                    }
                }
            }
            div { class: "flex flex-col px-2 pb-4",
                if busy_code() {
                    div { class: "flex flex-col gap-2 p-2", Skeleton { class: "h-14" } p { class: "text-center text-sm text-muted-foreground", "Barcode wird gesucht …" } }
                }
                if searching {
                    // Lokale Treffer
                    match &*local.read() {
                        Some(Some(list)) if !list.is_empty() => rsx! {
                            p { class: "px-3 pb-1 pt-2 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground", "Meine Datenbank" }
                            for f in list.clone() {
                                {
                                    let (f1, f2) = (f.clone(), f.clone());
                                    rsx! { FoodRow { key: "{f.id}", food: f, onclick: move |_| overlay.set(Overlay::Amount(f1.clone())), onquick: move |_| quick_add(f2.clone()) } }
                                }
                            }
                        },
                        Some(Some(_)) => rsx! {},
                        _ => rsx! { div { class: "p-2", Skeleton { class: "h-14" } } },
                    }
                    // Online
                    div { class: "flex items-center gap-2 px-3 pb-1 pt-4",
                        Globe { class: "size-3.5 text-muted-foreground" }
                        p { class: "flex-1 text-[11px] font-semibold uppercase tracking-wide text-muted-foreground", "Open Food Facts" }
                        button { class: "text-xs text-muted-foreground underline-offset-2 hover:underline", onclick: move |_| online_on.toggle(),
                            if online_on() { "aus" } else { "ein" }
                        }
                    }
                    match &*online.read() {
                        None if online_on() && q.trim().chars().count() >= 3 => rsx! {
                            div { class: "flex flex-col gap-2 p-2", Skeleton { class: "h-14" } Skeleton { class: "h-14" } }
                        },
                        Some(Some(Ok(list))) if !list.is_empty() => rsx! {
                            for (i, f) in list.clone().into_iter().enumerate() {
                                {
                                    let fi = f.clone();
                                    let preview = Food {
                                        id: format!("off-{i}"),
                                        name: f.name.clone(),
                                        brand: f.brand.clone(),
                                        unit: f.unit.clone(),
                                        serving_size: f.serving_size,
                                        serving_name: f.serving_name.clone(),
                                        kcal: f.kcal,
                                        image_url: f.image_url.clone(),
                                        ..Default::default()
                                    };
                                    rsx! { FoodRow { key: "off-{i}", food: preview, trailing: format!("{} kcal/100", f.kcal.round()), onclick: move |_| pick_online(fi.clone()) } }
                                }
                            }
                        },
                        Some(Some(Ok(_))) => rsx! { p { class: "px-3 py-3 text-sm text-muted-foreground", "Keine Online-Treffer." } },
                        Some(Some(Err(e))) => rsx! { p { class: "px-3 py-3 text-sm text-muted-foreground", "Online-Suche nicht möglich: {e}" } },
                        _ => rsx! {
                            if q.trim().chars().count() < 3 && online_on() {
                                p { class: "px-3 py-3 text-sm text-muted-foreground", "Mindestens 3 Zeichen für die Online-Suche." }
                            }
                        },
                    }
                    div { class: "px-2 pt-4",
                        button {
                            class: "flex w-full items-center justify-center gap-2 rounded-xl border border-dashed py-3 text-sm text-muted-foreground",
                            onclick: move |_| overlay.set(Overlay::Form(Some(FoodInput { name: query(), unit: "g".into(), serving_size: 100.0, shared: true, ..Default::default() }))),
                            Plus { class: "size-4" } "„{q.trim()}“ selbst anlegen"
                        }
                    }
                } else if tab() == "meals" {
                    match &*meals.read() {
                        Some(list) if list.is_empty() => rsx! {
                            Hint { text: "Noch keine Mahlzeiten. Lege unter Lebensmittel → Mahlzeiten Vorlagen an, z.B. dein Standard-Frühstück." }
                        },
                        Some(list) => rsx! {
                            for t in list.clone() {
                                {
                                    let tid = t.id.clone();
                                    let tot = t.totals();
                                    let name = t.name.clone();
                                    rsx! {
                                        button {
                                            key: "{t.id}",
                                            class: "flex items-center gap-3 rounded-xl px-2 py-2.5 text-left active:bg-accent hover:bg-accent/40",
                                            onclick: move |_| {
                                                let (pid, id, m, name) = (app.pid(), tid.clone(), meal(), name.clone());
                                                spawn(async move {
                                                    match add_meal_entries(pid, date, m.clone(), id, 1.0).await {
                                                        Ok(n) => { app.bump(); added += n as u32; app.toast(format!("{name} → {} ({n} Positionen)", meal_label(&m))); }
                                                        Err(e) => app.error(e),
                                                    }
                                                });
                                            },
                                            span { class: "flex size-11 shrink-0 items-center justify-center rounded-xl bg-muted text-lg", "🍽" }
                                            span { class: "flex min-w-0 flex-1 flex-col",
                                                span { class: "truncate text-[15px] font-medium", "{t.name}" }
                                                span { class: "truncate text-xs text-muted-foreground", "{t.items.len()} Positionen · P {num(tot.protein, 0)} · K {num(tot.carbs, 0)} · F {num(tot.fat, 0)}" }
                                            }
                                            span { class: "text-sm font-semibold tabular-nums", "{tot.kcal.round()}" span { class: "ml-0.5 text-xs font-normal text-muted-foreground", "kcal" } }
                                        }
                                    }
                                }
                            }
                        },
                        None => rsx! { div { class: "flex flex-col gap-2 p-2", Skeleton { class: "h-14" } Skeleton { class: "h-14" } } },
                    }
                } else {
                    match &*lists.read() {
                        Some(list) if list.is_empty() => rsx! {
                            Hint {
                                text: match tab().as_str() {
                                    "fav" => "Noch keine Favoriten. Tippe beim Eintragen auf den Stern.",
                                    "mine" => "Noch keine eigenen Lebensmittel.",
                                    _ => "Suche ein Lebensmittel oder scanne einen Barcode – zuletzt gegessene erscheinen dann hier.",
                                }
                            }
                        },
                        Some(list) => rsx! {
                            for f in list.clone() {
                                {
                                    let (f1, f2) = (f.clone(), f.clone());
                                    rsx! { FoodRow { key: "{f.id}", food: f, onclick: move |_| overlay.set(Overlay::Amount(f1.clone())), onquick: move |_| quick_add(f2.clone()) } }
                                }
                            }
                        },
                        None => rsx! { div { class: "flex flex-col gap-2 p-2", Skeleton { class: "h-14" } Skeleton { class: "h-14" } Skeleton { class: "h-14" } } },
                    }
                }
            }
        }
        match overlay() {
            Overlay::Amount(f) => rsx! {
                AddFoodSheet {
                    key: "{f.id}",
                    food: f,
                    date,
                    meal: meal(),
                    onclose: move |_| overlay.set(Overlay::None),
                    ondone: move |_| { added += 1; query.set(String::new()); overlay.set(Overlay::None); },
                }
            },
            Overlay::Form(prefill) => rsx! {
                FoodFormSheet {
                    prefill,
                    onclose: move |_| overlay.set(Overlay::None),
                    onsaved: move |f: Food| overlay.set(Overlay::Amount(f)),
                }
            },
            Overlay::Quick => rsx! {
                QuickEntrySheet { date, meal: meal(), onclose: move |_| overlay.set(Overlay::None), ondone: move |_| { added += 1; overlay.set(Overlay::None); } }
            },
            Overlay::Scan => rsx! { Scanner { onresult: on_code, onclose: move |_| overlay.set(Overlay::None) } },
            Overlay::Ai => rsx! {
                AiSheet {
                    date,
                    meal: meal(),
                    onclose: move |_| { if ai && added() == 0 { close() } else { overlay.set(Overlay::None) } },
                    ondone: move |n: usize| { added += n as u32; overlay.set(Overlay::None); if ai { close() } },
                }
            },
            Overlay::None => rsx! {},
        }
    }
}

#[component]
fn Hint(text: &'static str) -> Element {
    rsx! { p { class: "px-6 py-10 text-center text-sm text-muted-foreground", "{text}" } }
}

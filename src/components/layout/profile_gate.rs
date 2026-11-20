use dioxus::prelude::*;

use crate::api::profile::{list_profiles, onboard};
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::field::{Field, INPUT, NumberInput, SELECT};
use crate::components::ui::segmented::Segmented;
use crate::model::ProfileInput;
use crate::state::{remember_profile, use_app};
use crate::util::{ACTIVITY_LEVELS, parse_date, parse_num};

/// Lädt die Profile, wählt das gemerkte (localStorage) und zeigt sonst die Einrichtung.
/// Kinder werden erst gerendert, wenn ein Profil aktiv ist.
#[component]
pub fn ProfileGate(children: Element) -> Element {
    let mut app = use_app();
    let mut loaded = use_signal(|| false);
    let mut error = use_signal(|| None::<String>);

    use_effect(move || {
        spawn(async move {
            let stored = document::eval("return localStorage.getItem('nrzfitness.profile') || '';").await;
            let stored = stored.ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default();
            match list_profiles().await {
                Ok(list) => {
                    let pick = list.iter().find(|p| p.id == stored).or_else(|| list.first()).cloned();
                    app.profiles.set(list);
                    if let Some(p) = pick {
                        remember_profile(&p.id);
                        app.profile.set(Some(p));
                    }
                    loaded.set(true);
                }
                Err(e) => error.set(Some(e.to_string())),
            }
        });
    });

    if let Some(e) = error() {
        return rsx! {
            div { class: "flex h-full flex-col items-center justify-center gap-3 p-8 text-center",
                p { class: "text-lg font-semibold", "Server nicht erreichbar" }
                p { class: "max-w-sm text-sm text-muted-foreground", "{e}" }
            }
        };
    }
    if !loaded() {
        return rsx! { Splash {} };
    }
    if app.profile.read().is_none() {
        return rsx! { Onboarding {} };
    }
    rsx! { {children} }
}

#[component]
fn Splash() -> Element {
    rsx! {
        div { class: "flex h-full items-center justify-center",
            div { class: "flex size-14 animate-pulse items-center justify-center rounded-2xl bg-primary text-2xl font-bold text-primary-foreground", "F" }
        }
    }
}

/// Erstes Profil anlegen – auf dem Handy als Vollbild-Formular.
#[component]
pub fn Onboarding() -> Element {
    let mut app = use_app();
    let mut name = use_signal(String::new);
    let mut sex = use_signal(|| "m".to_string());
    let mut birth = use_signal(String::new);
    let mut height = use_signal(String::new);
    let mut weight = use_signal(String::new);
    let mut target = use_signal(String::new);
    let mut goal = use_signal(|| "0".to_string());
    let mut activity = use_signal(|| "light".to_string());
    let mut busy = use_signal(|| false);
    let mut err = use_signal(|| None::<String>);

    let submit = move |_| {
        if name().trim().is_empty() {
            err.set(Some("Bitte gib einen Namen ein.".into()));
            return;
        }
        busy.set(true);
        let input = ProfileInput {
            name: name(),
            color: "emerald".into(),
            sex: sex(),
            birth_date: parse_date(&birth()),
            height_cm: parse_num(&height()),
            activity_level: activity(),
            cycle_tracking: sex() == "f",
        };
        let weekly: f64 = goal().parse().unwrap_or(0.0);
        spawn(async move {
            match onboard(input, parse_num(&weight()), weekly, parse_num(&target())).await {
                Ok(p) => {
                    remember_profile(&p.id);
                    app.profiles.write().push(p.clone());
                    app.profile.set(Some(p));
                }
                Err(e) => {
                    err.set(Some(e.to_string()));
                    busy.set(false);
                }
            }
        });
    };

    rsx! {
        div { class: "h-full overflow-y-auto",
            div { class: "mx-auto flex min-h-full w-full max-w-md flex-col gap-6 px-5 pb-[calc(env(safe-area-inset-bottom)+1.5rem)] pt-[calc(env(safe-area-inset-top)+2.5rem)]",
                div { class: "flex flex-col gap-3",
                    div { class: "flex size-14 items-center justify-center rounded-2xl bg-primary text-2xl font-bold text-primary-foreground", "F" }
                    h1 { class: "text-3xl font-bold tracking-tight", "Willkommen bei nrzFitness" }
                    p { class: "text-muted-foreground",
                        "Ernährung, Training, Körper und Gesundheit – privat auf deinem Server. Ein paar Angaben, dann berechnen wir deine Ziele."
                    }
                }
                div { class: "flex flex-col gap-4",
                    Field { label: "Name",
                        input { class: INPUT, placeholder: "Dein Name", value: name(), autocomplete: "given-name", oninput: move |e| name.set(e.value()) }
                    }
                    Field { label: "Geschlecht (für den Grundumsatz)",
                        Segmented {
                            options: vec![("m".into(), "Männlich".into()), ("f".into(), "Weiblich".into())],
                            value: sex(),
                            onchange: move |v| sex.set(v),
                        }
                    }
                    div { class: "grid grid-cols-2 gap-3",
                        Field { label: "Geburtsdatum",
                            input { class: INPUT, r#type: "date", value: birth(), oninput: move |e| birth.set(e.value()) }
                        }
                        Field { label: "Grösse",
                            NumberInput { value: height(), unit: "cm", placeholder: "175", oninput: move |v| height.set(v) }
                        }
                        Field { label: "Gewicht",
                            NumberInput { value: weight(), unit: "kg", placeholder: "75", oninput: move |v| weight.set(v) }
                        }
                        Field { label: "Zielgewicht",
                            NumberInput { value: target(), unit: "kg", placeholder: "optional", oninput: move |v| target.set(v) }
                        }
                    }
                    Field { label: "Aktivität",
                        select { class: SELECT, value: activity(), onchange: move |e| activity.set(e.value()),
                            for (k, l) in ACTIVITY_LEVELS {
                                option { value: k, selected: activity() == k, "{l}" }
                            }
                        }
                    }
                    Field { label: "Ziel",
                        Segmented {
                            options: vec![("-0.5".into(), "Abnehmen".into()), ("0".into(), "Halten".into()), ("0.25".into(), "Zunehmen".into())],
                            value: goal(),
                            onchange: move |v| goal.set(v),
                        }
                    }
                }
                if let Some(e) = err() {
                    p { class: "rounded-xl bg-destructive/15 px-3 py-2 text-sm text-destructive", "{e}" }
                }
                div { class: "flex-1" }
                Button { size: ButtonSize::Lg, class: "w-full", disabled: busy(), onclick: submit,
                    if busy() { "Wird eingerichtet …" } else { "Los geht’s" }
                }
            }
        }
    }
}

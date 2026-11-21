use dioxus::prelude::*;
use icons::{ChevronRight, Moon, Plus, Sun};

use crate::api::profile::{create_profile, delete_profile, list_profiles, update_profile};
use crate::components::hooks::use_theme_mode::use_theme_mode;
use crate::components::layout::page::{PageBody, PageHeader, Section};
use crate::components::ui::avatar::Avatar;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::Card;
use crate::components::ui::field::{Field, INPUT, NumberInput, SELECT, Switch};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::sheet::Sheet;
use crate::model::{Profile, ProfileInput};
use crate::state::{remember_profile, use_app};
use crate::util::{ACTIVITY_LEVELS, PROFILE_COLORS, num, parse_date, parse_num, profile_bg};

#[component]
pub fn SettingsPage() -> Element {
    let mut app = use_app();
    let mut is_dark = use_theme_mode();
    let mut editing = use_signal(|| None::<Option<Profile>>);
    let current = app.profile();
    let profiles = app.profiles.read().clone();

    rsx! {
        PageHeader { title: "Einstellungen" }
        PageBody { class: "max-w-2xl",
            if let Some(p) = current.clone() {
                Section { title: "Aktives Profil",
                    Card { class: "flex items-center gap-4 p-4", onclick: { let pc = p.clone(); move |_| editing.set(Some(Some(pc.clone()))) },
                        Avatar { name: p.name.clone(), color: p.color.clone(), class: "size-14 text-lg" }
                        div { class: "flex min-w-0 flex-1 flex-col",
                            span { class: "truncate text-lg font-semibold", "{p.name}" }
                            span { class: "truncate text-sm text-muted-foreground",
                                {
                                    let mut parts = vec![if p.sex == "f" { "weiblich".to_string() } else { "männlich".to_string() }];
                                    if let Some(a) = p.age() { parts.push(format!("{a} Jahre")); }
                                    if let Some(h) = p.height_cm { parts.push(format!("{} cm", num(h, 0))); }
                                    parts.join(" · ")
                                }
                            }
                        }
                        ChevronRight { class: "size-5 text-muted-foreground" }
                    }
                }
            }
            Section { title: "Profile",
                Card { class: "flex flex-col p-1.5",
                    for p in profiles {
                        {
                            let active = current.as_ref().is_some_and(|c| c.id == p.id);
                            let pc = p.clone();
                            rsx! {
                                button {
                                    key: "{p.id}",
                                    class: "flex items-center gap-3 rounded-xl px-3 py-2.5 text-left hover:bg-accent/50 active:bg-accent",
                                    onclick: move |_| {
                                        remember_profile(&pc.id);
                                        app.profile.set(Some(pc.clone()));
                                        app.bump();
                                    },
                                    Avatar { name: p.name.clone(), color: p.color.clone() }
                                    span { class: "flex-1 text-sm font-medium", "{p.name}" }
                                    if active { span { class: "rounded-full bg-muted px-2 py-0.5 text-[11px] text-muted-foreground", "aktiv" } }
                                }
                            }
                        }
                    }
                    button {
                        class: "flex items-center gap-3 rounded-xl px-3 py-2.5 text-left text-sm text-muted-foreground hover:bg-accent/50",
                        onclick: move |_| editing.set(Some(None)),
                        span { class: "flex size-8 items-center justify-center rounded-full border border-dashed", Plus { class: "size-4" } }
                        "Profil hinzufügen (Familie, Partner …)"
                    }
                }
                p { class: "px-1 text-xs text-muted-foreground", "Jedes Profil hat eigene Einträge und Ziele. Lebensmittel können geteilt werden." }
            }
            Section { title: "Gesundheits-Apps",
                crate::domain::settings::health_card::HealthCard {}
            }
            Section { title: "KI-Assistent",
                crate::domain::settings::ai_card::AiCard {}
            }
            Section { title: "Darstellung",
                Card { class: "flex items-center gap-3 p-4",
                    if is_dark() { Moon { class: "size-5" } } else { Sun { class: "size-5" } }
                    span { class: "flex-1 text-sm font-medium", "Dunkles Design" }
                    Switch { checked: is_dark(), onchange: move |v| is_dark.set(v) }
                }
            }
            crate::domain::reports::data_card::DataCard {}
            Section { title: "Über",
                Card { class: "flex flex-col gap-1 p-4 text-sm",
                    p { span { class: "font-semibold", "nrzFitness " } span { class: "text-muted-foreground", "v{env!(\"CARGO_PKG_VERSION\")}" } }
                    p { class: "text-muted-foreground", "Ernährung, Training, Körper und Gesundheit – komplett in Rust, selbst gehostet. Inspiriert von SparkyFitness." }
                    p { class: "text-muted-foreground", "Lebensmitteldaten online: Open Food Facts (ODbL)." }
                    p { class: "pt-2 text-xs text-muted-foreground", "Tipp: Im Browser „Zum Home-Bildschirm“ wählen – dann läuft nrzFitness wie eine App." }
                }
            }
        }
        if let Some(p) = editing() {
            ProfileSheet { existing: p, onclose: move |_| editing.set(None) }
        }
    }
}

#[component]
fn ProfileSheet(existing: Option<Profile>, onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let e = existing.clone();
    let mut name = use_signal(|| e.as_ref().map(|p| p.name.clone()).unwrap_or_default());
    let mut color = use_signal(|| e.as_ref().map(|p| p.color.clone()).unwrap_or_else(|| "sky".into()));
    let mut sex = use_signal(|| e.as_ref().map(|p| p.sex.clone()).unwrap_or_else(|| "m".into()));
    let mut birth = use_signal(|| e.as_ref().and_then(|p| p.birth_date).map(|d| d.format("%Y-%m-%d").to_string()).unwrap_or_default());
    let mut height = use_signal(|| e.as_ref().and_then(|p| p.height_cm).map(|h| num(h, 1)).unwrap_or_default());
    let mut activity = use_signal(|| e.as_ref().map(|p| p.activity_level.clone()).unwrap_or_else(|| "light".into()));
    let mut cycle = use_signal(|| e.as_ref().is_some_and(|p| p.cycle_tracking));
    let mut confirm_delete = use_signal(|| false);
    let id = existing.as_ref().map(|p| p.id.clone());
    let can_delete = app.profiles.read().len() > 1;

    let id_save = id.clone();
    let save = move |_| {
        let input = ProfileInput {
            name: name(),
            color: color(),
            sex: sex(),
            birth_date: parse_date(&birth()),
            height_cm: parse_num(&height()),
            activity_level: activity(),
            cycle_tracking: cycle(),
        };
        let id = id_save.clone();
        spawn(async move {
            let r = match id {
                Some(id) => update_profile(id, input).await,
                None => create_profile(input).await,
            };
            match r {
                Ok(p) => {
                    if let Ok(list) = list_profiles().await {
                        app.profiles.set(list);
                    }
                    if app.profile.peek().as_ref().is_some_and(|c| c.id == p.id) {
                        app.profile.set(Some(p));
                    }
                    app.bump();
                    app.toast("Profil gespeichert");
                    onclose.call(());
                }
                Err(e) => app.error(e),
            }
        });
    };

    rsx! {
        Sheet {
            title: if id.is_some() { "Profil bearbeiten" } else { "Neues Profil" },
            onclose,
            footer: rsx! {
                div { class: "flex gap-2",
                    if let Some(pid) = id.clone().filter(|_| can_delete) {
                        Button { variant: ButtonVariant::Secondary, size: ButtonSize::Lg, class: "text-destructive",
                            onclick: move |_| {
                                if !confirm_delete() { confirm_delete.set(true); return; }
                                let pid = pid.clone();
                                spawn(async move {
                                    match delete_profile(pid.clone()).await {
                                        Ok(()) => {
                                            let list = list_profiles().await.unwrap_or_default();
                                            if app.profile.peek().as_ref().is_some_and(|c| c.id == pid) {
                                                let next = list.first().cloned();
                                                if let Some(n) = &next { remember_profile(&n.id); }
                                                app.profile.set(next);
                                            }
                                            app.profiles.set(list);
                                            app.bump();
                                            onclose.call(());
                                        }
                                        Err(e) => app.error(e),
                                    }
                                });
                            },
                            if confirm_delete() { "Wirklich löschen?" } else { "Löschen" }
                        }
                    }
                    Button { size: ButtonSize::Lg, class: "flex-1", onclick: save, "Speichern" }
                }
            },
            div { class: "flex flex-col gap-4 px-5 pb-4 pt-1",
                Field { label: "Name",
                    input { class: INPUT, value: name(), oninput: move |e| name.set(e.value()) }
                }
                Field { label: "Farbe",
                    div { class: "flex flex-wrap gap-2",
                        for c in PROFILE_COLORS {
                            button {
                                key: "{c}",
                                class: "size-9 rounded-full transition-transform active:scale-90 {profile_bg(c)}",
                                class: if color() == c { "ring-2 ring-foreground ring-offset-2 ring-offset-popover" },
                                onclick: move |_| color.set(c.to_string()),
                            }
                        }
                    }
                }
                Field { label: "Geschlecht",
                    Segmented { options: vec![("m".into(), "Männlich".into()), ("f".into(), "Weiblich".into())], value: sex(), onchange: move |v| sex.set(v) }
                }
                div { class: "grid grid-cols-2 gap-3",
                    Field { label: "Geburtsdatum", input { class: INPUT, r#type: "date", value: birth(), oninput: move |e| birth.set(e.value()) } }
                    Field { label: "Grösse", NumberInput { value: height(), unit: "cm", oninput: move |v| height.set(v) } }
                }
                Field { label: "Aktivität",
                    select { class: SELECT, onchange: move |e| activity.set(e.value()),
                        for (k, l) in ACTIVITY_LEVELS {
                            option { value: k, selected: activity() == k, "{l}" }
                        }
                    }
                }
                div { class: "flex items-center justify-between gap-3 rounded-2xl border p-4",
                    div { class: "flex flex-col",
                        span { class: "text-sm font-medium", "Zyklus-Tracking" }
                        span { class: "text-xs text-muted-foreground", "Periode und Vorhersagen im Bereich Zyklus" }
                    }
                    Switch { checked: cycle(), onchange: move |v| cycle.set(v) }
                }
            }
        }
    }
}

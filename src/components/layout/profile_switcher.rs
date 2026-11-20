use dioxus::prelude::*;
use icons::{Check, ChevronDown, Plus};

use crate::app::Route;
use crate::components::ui::avatar::Avatar;
use crate::components::ui::sheet::Sheet;
use crate::state::{remember_profile, use_app};

/// Profilwahl in der Seitenleiste.
#[component]
pub fn ProfileButton() -> Element {
    let app = use_app();
    let mut open = use_signal(|| false);
    let Some(p) = app.profile() else { return rsx! {} };
    rsx! {
        button {
            class: "flex w-full items-center gap-2.5 rounded-xl px-2 py-2 text-left hover:bg-accent/60",
            onclick: move |_| open.set(true),
            Avatar { name: p.name.clone(), color: p.color.clone() }
            span { class: "min-w-0 flex-1 truncate text-sm font-medium", "{p.name}" }
            ChevronDown { class: "size-4 text-muted-foreground" }
        }
        if open() {
            ProfileSheet { onclose: move |_| open.set(false) }
        }
    }
}

/// Runder Avatar oben rechts (mobil) → Profilwahl.
#[component]
pub fn ProfileAvatarButton() -> Element {
    let app = use_app();
    let mut open = use_signal(|| false);
    let Some(p) = app.profile() else { return rsx! {} };
    rsx! {
        button { class: "rounded-full transition-transform active:scale-90", onclick: move |_| open.set(true),
            Avatar { name: p.name.clone(), color: p.color.clone(), class: "size-9 text-sm" }
        }
        if open() {
            ProfileSheet { onclose: move |_| open.set(false) }
        }
    }
}

#[component]
pub fn ProfileSheet(onclose: EventHandler<()>) -> Element {
    let mut app = use_app();
    let navigator = use_navigator();
    let current = app.pid_tracked();
    let profiles = app.profiles.read().clone();
    rsx! {
        Sheet { title: "Profil wählen", onclose,
            div { class: "flex flex-col gap-1 p-3",
                for p in profiles {
                    {
                        let active = p.id == current;
                        let pc = p.clone();
                        rsx! {
                            button {
                                key: "{p.id}",
                                class: "flex items-center gap-3 rounded-xl px-3 py-3 text-left hover:bg-accent/60 active:bg-accent",
                                onclick: move |_| {
                                    remember_profile(&pc.id);
                                    app.profile.set(Some(pc.clone()));
                                    app.bump();
                                    onclose.call(());
                                },
                                Avatar { name: p.name.clone(), color: p.color.clone(), class: "size-10 text-sm" }
                                span { class: "flex-1 font-medium", "{p.name}" }
                                if active { Check { class: "size-5" } }
                            }
                        }
                    }
                }
                button {
                    class: "flex items-center gap-3 rounded-xl px-3 py-3 text-left text-muted-foreground hover:bg-accent/60",
                    onclick: move |_| {
                        onclose.call(());
                        navigator.push(Route::Settings {});
                    },
                    span { class: "flex size-10 items-center justify-center rounded-full border border-dashed", Plus { class: "size-4" } }
                    span { "Profile verwalten" }
                }
            }
        }
    }
}

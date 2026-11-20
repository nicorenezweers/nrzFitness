//! Formular für den täglichen Check-in (in Karten gruppiert).

use chrono::NaiveDate;
use dioxus::prelude::*;
use icons::{Activity, ChevronDown, Moon, Ruler, Scale, Smile, StickyNote};

use crate::api::body::save_checkin;
use crate::components::ui::button::{Button, ButtonSize};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::field::Scale5;
use crate::components::ui::field::{Field, NumberInput, TEXTAREA};
use crate::components::ui::ring::Bar;
use crate::model::{Checkin, ENERGY_EMOJI, MOOD_EMOJI, SLEEP_EMOJI, STRESS_EMOJI};
use crate::state::use_app;
use crate::util::{int, num, parse_num};

fn s(v: Option<f64>) -> String {
    v.map(|x| num(x, 2)).unwrap_or_default()
}

fn si(v: Option<i64>) -> String {
    v.map(|x| x.to_string()).unwrap_or_default()
}

fn pi(v: &str) -> Option<i64> {
    parse_num(v).map(|x| x.round() as i64)
}

fn emojis(e: [&str; 5]) -> Vec<String> {
    e.iter().map(|x| x.to_string()).collect()
}

#[component]
pub fn CheckinForm(date: NaiveDate, initial: Checkin, steps_goal: i64, sleep_goal: f64) -> Element {
    let mut app = use_app();
    let c = initial.clone();
    let mut weight = use_signal(|| s(c.weight));
    let mut body_fat = use_signal(|| s(c.body_fat));
    let mut waist = use_signal(|| s(c.waist));
    let mut hips = use_signal(|| s(c.hips));
    let mut chest = use_signal(|| s(c.chest));
    let mut neck = use_signal(|| s(c.neck));
    let mut arm = use_signal(|| s(c.arm));
    let mut thigh = use_signal(|| s(c.thigh));
    let mut steps = use_signal(|| si(c.steps));
    let mut resting_hr = use_signal(|| si(c.resting_hr));
    let mut mood = use_signal(|| c.mood);
    let mut energy = use_signal(|| c.energy);
    let mut stress = use_signal(|| c.stress);
    let mut sleep_hours = use_signal(|| s(c.sleep_hours));
    let mut sleep_quality = use_signal(|| c.sleep_quality);
    let mut notes = use_signal(|| c.notes.clone().unwrap_or_default());
    let measured = c.measurements().iter().filter(|m| m.2.is_some()).count();
    let mut show_measures = use_signal(move || measured > 0);
    let mut busy = use_signal(|| false);

    let build = move || Checkin {
        date,
        weight: parse_num(&weight()),
        body_fat: parse_num(&body_fat()),
        waist: parse_num(&waist()),
        hips: parse_num(&hips()),
        chest: parse_num(&chest()),
        neck: parse_num(&neck()),
        arm: parse_num(&arm()),
        thigh: parse_num(&thigh()),
        steps: pi(&steps()),
        mood: mood(),
        energy: energy(),
        stress: stress(),
        sleep_hours: parse_num(&sleep_hours()),
        sleep_quality: sleep_quality(),
        resting_hr: pi(&resting_hr()),
        notes: Some(notes()).filter(|n| !n.trim().is_empty()),
    };
    let dirty = build() != Checkin { notes: initial.notes.clone().filter(|n| !n.trim().is_empty()), ..initial.clone() };

    let save = move |_| {
        if busy() {
            return;
        }
        busy.set(true);
        let pid = app.pid();
        let c = build();
        spawn(async move {
            match save_checkin(pid, c).await {
                Ok(_) => {
                    app.toast("Check-in gespeichert");
                    app.bump();
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };

    let steps_v = pi(&steps()).unwrap_or(0) as f64;
    let sleep_v = parse_num(&sleep_hours()).unwrap_or(0.0);
    let measures_set =
        [waist(), hips(), chest(), neck(), arm(), thigh()].iter().filter(|v| !v.trim().is_empty()).count();

    rsx! {
        div { class: "flex flex-col gap-3",
            Card {
                CardHeader { title: "Gewicht", icon: rsx! { Scale { class: "size-4" } } }
                div { class: "grid grid-cols-2 gap-3 px-4 pb-4",
                    Field { label: "Gewicht",
                        NumberInput { value: weight(), oninput: move |v| weight.set(v), unit: "kg", placeholder: "–" }
                    }
                    Field { label: "Körperfett",
                        NumberInput { value: body_fat(), oninput: move |v| body_fat.set(v), unit: "%", placeholder: "–" }
                    }
                }
            }
            Card {
                button {
                    r#type: "button",
                    class: "flex w-full items-center gap-3 px-4 py-4 text-left",
                    onclick: move |_| show_measures.toggle(),
                    div { class: "flex size-8 shrink-0 items-center justify-center rounded-xl bg-muted", Ruler { class: "size-4" } }
                    div { class: "min-w-0 flex-1",
                        h3 { class: "text-[15px] font-semibold", "Umfänge" }
                        p { class: "text-xs text-muted-foreground",
                            if measures_set > 0 { "{measures_set} von 6 erfasst" } else { "Taille, Hüfte, Brust, Hals, Arm, Oberschenkel" }
                        }
                    }
                    ChevronDown { class: if show_measures() { "size-5 rotate-180 text-muted-foreground transition-transform" } else { "size-5 text-muted-foreground transition-transform" } }
                }
                if show_measures() {
                    div { class: "grid grid-cols-2 gap-3 px-4 pb-4 sm:grid-cols-3",
                        Field { label: "Taille", NumberInput { value: waist(), oninput: move |v| waist.set(v), unit: "cm" } }
                        Field { label: "Hüfte", NumberInput { value: hips(), oninput: move |v| hips.set(v), unit: "cm" } }
                        Field { label: "Brust", NumberInput { value: chest(), oninput: move |v| chest.set(v), unit: "cm" } }
                        Field { label: "Hals", NumberInput { value: neck(), oninput: move |v| neck.set(v), unit: "cm" } }
                        Field { label: "Arm", NumberInput { value: arm(), oninput: move |v| arm.set(v), unit: "cm" } }
                        Field { label: "Oberschenkel", NumberInput { value: thigh(), oninput: move |v| thigh.set(v), unit: "cm" } }
                    }
                }
            }
            Card {
                CardHeader { title: "Aktivität", icon: rsx! { Activity { class: "size-4" } } }
                div { class: "grid grid-cols-2 gap-3 px-4 pb-4",
                    Field { label: "Schritte",
                        NumberInput { value: steps(), oninput: move |v| steps.set(v), integer: true, placeholder: "–" }
                    }
                    Field { label: "Ruhepuls",
                        NumberInput { value: resting_hr(), oninput: move |v| resting_hr.set(v), integer: true, unit: "bpm", placeholder: "–" }
                    }
                    if steps_goal > 0 {
                        div { class: "col-span-2 flex flex-col gap-1.5",
                            Bar { value: steps_v / steps_goal as f64, color: "bg-good" }
                            span { class: "text-[11px] text-muted-foreground", "{int(steps_v)} von {int(steps_goal as f64)} Schritten" }
                        }
                    }
                }
            }
            Card {
                CardHeader { title: "Befinden", icon: rsx! { Smile { class: "size-4" } } }
                div { class: "flex flex-col gap-3 px-4 pb-4",
                    Field { label: "Stimmung", Scale5 { value: mood(), onchange: move |v| mood.set(v), labels: emojis(MOOD_EMOJI) } }
                    Field { label: "Energie", Scale5 { value: energy(), onchange: move |v| energy.set(v), labels: emojis(ENERGY_EMOJI) } }
                    Field { label: "Stress", Scale5 { value: stress(), onchange: move |v| stress.set(v), labels: emojis(STRESS_EMOJI) } }
                }
            }
            Card {
                CardHeader { title: "Schlaf", icon: rsx! { Moon { class: "size-4" } } }
                div { class: "flex flex-col gap-3 px-4 pb-4",
                    Field { label: "Dauer",
                        NumberInput { value: sleep_hours(), oninput: move |v| sleep_hours.set(v), unit: "h", placeholder: "–" }
                    }
                    if sleep_goal > 0.0 && sleep_v > 0.0 {
                        div { class: "flex flex-col gap-1.5",
                            Bar { value: sleep_v / sleep_goal, color: "bg-protein" }
                            span { class: "text-[11px] text-muted-foreground", "{num(sleep_v, 1)} von {num(sleep_goal, 1)} h Ziel" }
                        }
                    }
                    Field { label: "Qualität", Scale5 { value: sleep_quality(), onchange: move |v| sleep_quality.set(v), labels: emojis(SLEEP_EMOJI) } }
                }
            }
            Card {
                CardHeader { title: "Notizen", icon: rsx! { StickyNote { class: "size-4" } } }
                div { class: "px-4 pb-4",
                    textarea {
                        class: "{TEXTAREA}",
                        placeholder: "Wie geht es dir heute?",
                        value: "{notes}",
                        oninput: move |e| notes.set(e.value()),
                    }
                }
            }
            div { class: "sticky bottom-[calc(env(safe-area-inset-bottom)+4.75rem)] z-10 sm:bottom-4",
                Button {
                    size: ButtonSize::Lg,
                    class: "w-full shadow-lg",
                    disabled: busy() || !dirty,
                    onclick: save,
                    if busy() { "Speichern …" } else if dirty { "Check-in speichern" } else { "Gespeichert" }
                }
            }
        }
    }
}

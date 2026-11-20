use dioxus::prelude::*;
use icons::{Flame, Sparkles};

use crate::api::profile::{get_goals, latest_weight, save_goals};
use crate::components::layout::page::{PageBody, PageHeader, Section};
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::field::{Field, NumberInput};
use crate::components::ui::segmented::Segmented;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{Goals, suggested_goals};
use crate::state::use_app;
use crate::util::{activity_factor, bmr, int, num, parse_num};

#[component]
pub fn GoalsPage() -> Element {
    let app = use_app();
    let data = use_resource(move || async move {
        let pid = app.pid_tracked();
        let _ = (app.refresh)();
        let g = get_goals(pid.clone()).await?;
        let w = latest_weight(pid).await?;
        Ok::<_, ServerFnError>((g, w))
    });
    let pid = app.pid_tracked();
    rsx! {
        PageHeader { title: "Ziele", subtitle: "Tagesziele für Ernährung, Wasser und Aktivität" }
        match &*data.read() {
            Some(Ok((g, w))) => rsx! { GoalsForm { key: "{pid}", goals: g.clone(), weight: *w } },
            Some(Err(e)) => rsx! { PageBody { p { class: "text-sm text-destructive", "{e}" } } },
            None => rsx! { PageBody { Skeleton { class: "h-64 rounded-2xl" } } },
        }
    }
}

fn s(v: f64, d: usize) -> String {
    num(v, d)
}

#[component]
fn GoalsForm(goals: Goals, weight: Option<f64>) -> Element {
    let mut app = use_app();
    let profile = app.profile();
    let mut kcal = use_signal(|| s(goals.calories, 0));
    let mut protein = use_signal(|| s(goals.protein_g, 0));
    let mut carbs = use_signal(|| s(goals.carbs_g, 0));
    let mut fat = use_signal(|| s(goals.fat_g, 0));
    let mut fiber = use_signal(|| s(goals.fiber_g, 0));
    let mut sugar = use_signal(|| s(goals.sugar_g, 0));
    let mut water = use_signal(|| s(goals.water_ml, 0));
    let mut steps = use_signal(|| s(goals.steps as f64, 0));
    let mut exercise = use_signal(|| s(goals.exercise_min as f64, 0));
    let mut sleep = use_signal(|| s(goals.sleep_hours, 1));
    let mut target = use_signal(|| goals.target_weight.map(|t| s(t, 1)).unwrap_or_default());
    let mut weekly = use_signal(|| s(goals.weekly_change, 2));
    let mut fasting = use_signal(|| s(goals.fasting_hours, 0));
    let mut busy = use_signal(|| false);

    let p = |sig: Signal<String>| parse_num(&sig()).unwrap_or(0.0);
    let macro_kcal = p(protein) * 4.0 + p(carbs) * 4.0 + p(fat) * 9.0;
    let pct = |v: f64| if macro_kcal > 0.0 { (v / macro_kcal * 100.0).round() } else { 0.0 };
    let diff = macro_kcal - p(kcal);

    // TDEE-Rechner
    let w = weight.unwrap_or(0.0);
    let tdee_info = profile.as_ref().and_then(|pr| {
        let h = pr.height_cm.filter(|h| *h > 0.0)?;
        (w > 0.0).then(|| {
            let b = bmr(&pr.sex, w, h, pr.age().unwrap_or(30));
            (b, b * activity_factor(&pr.activity_level))
        })
    });

    let base = goals.clone();
    let apply = move |_| {
        let Some(pr) = app.profile() else { return };
        if w <= 0.0 {
            return;
        }
        let g = suggested_goals(&pr, w, parse_num(&weekly()).unwrap_or(0.0), &base);
        kcal.set(s(g.calories, 0));
        protein.set(s(g.protein_g, 0));
        carbs.set(s(g.carbs_g, 0));
        fat.set(s(g.fat_g, 0));
        fiber.set(s(g.fiber_g, 0));
        water.set(s(g.water_ml, 0));
        app.toast("Vorschlag übernommen – noch speichern");
    };
    // Makros aus kcal mit Verteilung neu berechnen
    let mut split = move |(pp, cc, ff): (f64, f64, f64)| {
        let k = parse_num(&kcal()).unwrap_or(0.0);
        protein.set(s(k * pp / 4.0, 0));
        carbs.set(s(k * cc / 4.0, 0));
        fat.set(s(k * ff / 9.0, 0));
    };

    let save = move |_| {
        let p = |sig: Signal<String>| parse_num(&sig()).unwrap_or(0.0);
        let g = Goals {
            calories: p(kcal),
            protein_g: p(protein),
            carbs_g: p(carbs),
            fat_g: p(fat),
            fiber_g: p(fiber),
            sugar_g: p(sugar),
            water_ml: p(water),
            steps: p(steps).round() as i64,
            exercise_min: p(exercise).round() as i64,
            sleep_hours: p(sleep),
            target_weight: parse_num(&target()),
            weekly_change: p(weekly),
            fasting_hours: p(fasting),
        };
        busy.set(true);
        let pid = app.pid();
        spawn(async move {
            match save_goals(pid, g).await {
                Ok(_) => {
                    app.bump();
                    app.toast("Ziele gespeichert");
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };

    let weekly_v = parse_num(&weekly()).unwrap_or(0.0);
    let weekly_key = if weekly_v < -0.6 { "-0.75" } else if weekly_v < -0.3 { "-0.5" } else if weekly_v < -0.1 { "-0.25" } else if weekly_v > 0.1 { "0.25" } else { "0" };

    rsx! {
        PageBody { class: "lg:grid lg:grid-cols-2 lg:items-start",
            div { class: "flex flex-col gap-4",
                // Rechner
                Card {
                    CardHeader { title: "Bedarf berechnen", subtitle: "Mifflin-St Jeor · Profil & letztes Gewicht", icon: rsx! { Flame { class: "size-4" } } }
                    div { class: "flex flex-col gap-4 px-4 pb-4",
                        if let Some((b, t)) = tdee_info {
                            div { class: "grid grid-cols-3 gap-2 text-center",
                                Info { label: "Grundumsatz", value: format!("{}", int(b)), unit: "kcal" }
                                Info { label: "Gesamtumsatz", value: format!("{}", int(t)), unit: "kcal" }
                                Info { label: "Gewicht", value: num(w, 1), unit: "kg" }
                            }
                            Field { label: "Ziel pro Woche",
                                Segmented {
                                    options: vec![
                                        ("-0.75".into(), "−0,75".into()), ("-0.5".into(), "−0,5".into()), ("-0.25".into(), "−0,25".into()),
                                        ("0".into(), "Halten".into()), ("0.25".into(), "+0,25".into()),
                                    ],
                                    value: weekly_key.to_string(),
                                    onchange: move |v| weekly.set(v),
                                }
                            }
                            p { class: "text-xs text-muted-foreground",
                                "kg pro Woche · ≈ {int(t + weekly_v * 1100.0)} kcal pro Tag"
                            }
                            Button { variant: ButtonVariant::Secondary, onclick: apply, Sparkles { class: "size-4" } "Vorschlag übernehmen" }
                        } else {
                            p { class: "text-sm text-muted-foreground",
                                "Für den Rechner brauchst du Grösse (Einstellungen → Profil) und ein erfasstes Gewicht (Körper)."
                            }
                        }
                    }
                }
                Section { title: "Ernährung",
                    Card { class: "flex flex-col gap-4 p-4",
                        Field { label: "Kalorien pro Tag",
                            NumberInput { value: kcal(), unit: "kcal", integer: true, oninput: move |v| kcal.set(v) }
                        }
                        div { class: "grid grid-cols-3 gap-2",
                            Field { label: "Protein · {pct(p(protein) * 4.0)} %", NumberInput { value: protein(), unit: "g", oninput: move |v| protein.set(v) } }
                            Field { label: "Kohlenh. · {pct(p(carbs) * 4.0)} %", NumberInput { value: carbs(), unit: "g", oninput: move |v| carbs.set(v) } }
                            Field { label: "Fett · {pct(p(fat) * 9.0)} %", NumberInput { value: fat(), unit: "g", oninput: move |v| fat.set(v) } }
                        }
                        div { class: "flex flex-col gap-2",
                            span { class: "text-xs text-muted-foreground", "Verteilung aus Kalorien berechnen" }
                            div { class: "grid grid-cols-2 gap-2 sm:grid-cols-4",
                                for (label, sp) in [("Ausgewogen", (0.25, 0.50, 0.25)), ("Proteinreich", (0.35, 0.40, 0.25)), ("Low Carb", (0.30, 0.20, 0.50)), ("Keto", (0.20, 0.05, 0.75))] {
                                    button {
                                        key: "{label}",
                                        class: "rounded-xl bg-muted px-2 py-2 text-xs font-medium active:scale-95",
                                        onclick: move |_| split(sp),
                                        "{label}"
                                    }
                                }
                            }
                        }
                        if diff.abs() > 50.0 && macro_kcal > 0.0 {
                            p { class: "rounded-xl bg-warn/10 px-3 py-2 text-xs text-warn",
                                {format!("Die Makros ergeben {} kcal ({}{} zum Kalorienziel).", int(macro_kcal), if diff > 0.0 { "+" } else { "" }, int(diff))}
                            }
                        }
                        div { class: "grid grid-cols-2 gap-2",
                            Field { label: "Ballaststoffe (min.)", NumberInput { value: fiber(), unit: "g", oninput: move |v| fiber.set(v) } }
                            Field { label: "Zucker (max.)", NumberInput { value: sugar(), unit: "g", oninput: move |v| sugar.set(v) } }
                        }
                    }
                }
            }
            div { class: "flex flex-col gap-4",
                Section { title: "Körper",
                    Card { class: "grid grid-cols-2 gap-3 p-4",
                        Field { label: "Zielgewicht", NumberInput { value: target(), unit: "kg", placeholder: "optional", oninput: move |v| target.set(v) } }
                        Field { label: "Änderung/Woche", NumberInput { value: weekly(), unit: "kg", oninput: move |v| weekly.set(v) } }
                    }
                }
                Section { title: "Wasser & Aktivität",
                    Card { class: "grid grid-cols-2 gap-3 p-4",
                        Field { label: "Wasser", NumberInput { value: water(), unit: "ml", integer: true, oninput: move |v| water.set(v) } }
                        Field { label: "Schritte", NumberInput { value: steps(), integer: true, oninput: move |v| steps.set(v) } }
                        Field { label: "Training", NumberInput { value: exercise(), unit: "min", integer: true, oninput: move |v| exercise.set(v) } }
                        Field { label: "Schlaf", NumberInput { value: sleep(), unit: "h", oninput: move |v| sleep.set(v) } }
                        Field { label: "Fasten", NumberInput { value: fasting(), unit: "h", oninput: move |v| fasting.set(v) } }
                    }
                }
                Button { size: ButtonSize::Lg, class: "w-full", disabled: busy(), onclick: save, "Ziele speichern" }
            }
        }
    }
}

#[component]
fn Info(label: &'static str, value: String, unit: &'static str) -> Element {
    rsx! {
        div { class: "flex flex-col rounded-xl bg-muted/60 px-2 py-2.5",
            span { class: "text-[11px] text-muted-foreground", "{label}" }
            span { class: "text-lg font-semibold tabular-nums", "{value}" }
            span { class: "text-[10px] text-muted-foreground", "{unit}" }
        }
    }
}

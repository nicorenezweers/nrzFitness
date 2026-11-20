//! KI-Schätzung: Essen beschreiben und/oder fotografieren → Positionen mit Nährwerten → eintragen.

use chrono::NaiveDate;
use dioxus::prelude::*;
use icons::{Camera, Check, ImagePlus, LoaderCircle, RefreshCw, Sparkles, X};

use crate::api::ai::{ai_add_items, ai_estimate, get_ai_config};
use crate::app::Route;
use crate::components::ui::button::{Button, ButtonSize, ButtonVariant};
use crate::components::ui::field::{GHOST_INPUT, TEXTAREA};
use crate::components::ui::sheet::Sheet;
use crate::components::ui::skeleton::Skeleton;
use crate::model::{AiEstimate, AiItem, MEALS, meal_label, provider_label};
use crate::state::use_app;
use crate::util::{num, parse_num};

/// Liest das gewählte Bild aus einem Datei-Feld, dreht es richtig, verkleinert es (max. 1280 px)
/// und gibt eine JPEG-Data-URL zurück (oder "ERR:…").
const READ_IMAGE: &str = r#"
const id = await dioxus.recv();
const input = document.getElementById(id);
const f = input && input.files && input.files[0];
if (!f) { dioxus.send(""); return; }
const draw = (src, w, h) => {
  const s = Math.min(1, 1280 / Math.max(w, h));
  const c = document.createElement('canvas');
  c.width = Math.round(w * s); c.height = Math.round(h * s);
  c.getContext('2d').drawImage(src, 0, 0, c.width, c.height);
  return c.toDataURL('image/jpeg', 0.82);
};
try {
  let out;
  try {
    const bmp = await createImageBitmap(f, { imageOrientation: 'from-image' });
    out = draw(bmp, bmp.width, bmp.height);
  } catch (_) {
    const url = URL.createObjectURL(f);
    const img = await new Promise((ok, bad) => { const i = new Image(); i.onload = () => ok(i); i.onerror = bad; i.src = url; });
    out = draw(img, img.naturalWidth, img.naturalHeight);
    URL.revokeObjectURL(url);
  }
  dioxus.send(out);
} catch (e) {
  dioxus.send("ERR:" + (e && e.message ? e.message : "Bild konnte nicht gelesen werden"));
}
input.value = '';
"#;

async fn read_image(id: &str) -> Result<Option<String>, String> {
    let mut ev = document::eval(READ_IMAGE);
    let _ = ev.send(id.to_string());
    match ev.recv::<String>().await {
        Ok(s) if s.is_empty() => Ok(None),
        Ok(s) if s.starts_with("ERR:") => Err(s.trim_start_matches("ERR:").to_string()),
        Ok(s) => Ok(Some(s)),
        Err(_) => Err("Bild konnte nicht gelesen werden".into()),
    }
}

#[component]
pub fn AiSheet(date: NaiveDate, meal: String, onclose: EventHandler<()>, ondone: EventHandler<usize>) -> Element {
    let mut app = use_app();
    let navigator = use_navigator();
    let cfg = use_resource(get_ai_config);
    let mut meal = use_signal(|| meal);
    let mut text = use_signal(String::new);
    let mut image = use_signal(|| None::<String>);
    let mut busy = use_signal(|| false);
    let mut saving = use_signal(|| false);
    let mut result = use_signal(|| None::<AiEstimate>);
    // Pro Position: (Menge als Text, ausgewählt)
    let mut edits = use_signal(Vec::<(String, bool)>::new);
    let mut refine = use_signal(String::new);

    let on_pick = move |id: &'static str| {
        spawn(async move {
            match read_image(id).await {
                Ok(Some(d)) => image.set(Some(d)),
                Ok(None) => {}
                Err(m) => app.error(format!("Foto: {m}")),
            }
        });
    };

    let mut run = move |previous: Option<AiEstimate>, extra: String| {
        busy.set(true);
        let t = if extra.trim().is_empty() { text() } else { format!("{}\n{}", text(), extra.trim()) };
        let img = image();
        spawn(async move {
            match ai_estimate(t, img, previous).await {
                Ok(est) => {
                    if est.items.is_empty() {
                        app.error(if est.note.is_empty() { "Nichts Essbares erkannt".to_string() } else { est.note.clone() });
                    } else {
                        edits.set(est.items.iter().map(|i| (num(i.amount, 0), true)).collect());
                        refine.set(String::new());
                        result.set(Some(est));
                    }
                }
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };

    // Aktuelle Positionen mit bearbeiteten Mengen.
    let items: Vec<(AiItem, bool)> = match &*result.read() {
        Some(est) => est
            .items
            .iter()
            .enumerate()
            .map(|(i, it)| {
                let (a, on) = edits.read().get(i).cloned().unwrap_or((num(it.amount, 0), true));
                (it.scaled_to(parse_num(&a).unwrap_or(0.0)), on)
            })
            .collect(),
        None => Vec::new(),
    };
    let chosen: Vec<AiItem> = items.iter().filter(|(it, on)| *on && it.amount > 0.0).map(|(it, _)| it.clone()).collect();
    let (tk, tp, tc, tf) = chosen.iter().fold((0.0, 0.0, 0.0, 0.0), |a, i| (a.0 + i.kcal, a.1 + i.protein, a.2 + i.carbs, a.3 + i.fat));

    let save = move |_| {
        let list = chosen.clone();
        if list.is_empty() {
            return;
        }
        saving.set(true);
        let (pid, m) = (app.pid(), meal());
        spawn(async move {
            match ai_add_items(pid, date, m.clone(), list).await {
                Ok(n) => {
                    app.bump();
                    app.toast(format!("{n} Positionen · {} kcal → {}", tk.round(), meal_label(&m)));
                    ondone.call(n);
                }
                Err(e) => {
                    app.error(e);
                    saving.set(false);
                }
            }
        });
    };

    let config = cfg.read().clone();
    let ready = matches!(&config, Some(Ok(c)) if c.ready);
    let who = match &config {
        Some(Ok(c)) => provider_label(&c.provider).to_string(),
        _ => "KI".into(),
    };
    let has_result = result.read().is_some();
    let can_estimate = !busy() && (image().is_some() || !text().trim().is_empty());

    rsx! {
        Sheet {
            title: "Mit KI schätzen",
            tall: true,
            onclose,
            footer: rsx! {
                if has_result {
                    div { class: "flex flex-col gap-2",
                        div { class: "grid grid-cols-4 gap-1.5",
                            for (k, l) in MEALS {
                                button {
                                    key: "{k}",
                                    class: if meal() == k { "h-9 truncate rounded-xl bg-primary px-1 text-xs font-medium text-primary-foreground" } else { "h-9 truncate rounded-xl bg-muted px-1 text-xs text-muted-foreground" },
                                    onclick: move |_| meal.set(k.to_string()),
                                    "{l}"
                                }
                            }
                        }
                        Button { size: ButtonSize::Lg, class: "w-full", disabled: saving() || busy() || tk <= 0.0, onclick: save,
                            Check { class: "size-5" }
                            "Eintragen · {tk.round()} kcal"
                        }
                    }
                } else if ready {
                    Button { size: ButtonSize::Lg, class: "w-full", disabled: !can_estimate, onclick: move |_| run(None, String::new()),
                        if busy() { LoaderCircle { class: "size-5 animate-spin" } "{who} schätzt …" } else { Sparkles { class: "size-5" } "Kalorien schätzen" }
                    }
                }
            },
            div { class: "flex flex-col gap-4 px-4 pb-6 pt-1",
                match &config {
                    None => rsx! { Skeleton { class: "h-40 rounded-2xl" } },
                    Some(Ok(c)) if !c.ready => rsx! {
                        div { class: "flex flex-col items-center gap-3 rounded-2xl border border-dashed px-6 py-10 text-center",
                            span { class: "flex size-12 items-center justify-center rounded-2xl bg-muted", Sparkles { class: "size-6 text-kcal" } }
                            p { class: "font-semibold", "KI-Assistent einrichten" }
                            p { class: "text-sm text-muted-foreground",
                                "Wähle in den Einstellungen Claude, Grok, ChatGPT, Gemini oder einen eigenen Server und hinterlege den API-Schlüssel. Danach schätzt die KI Kalorien aus Text und Fotos."
                            }
                            Button {
                                onclick: move |_| {
                                    onclose.call(());
                                    app.food_sheet.set(None);
                                    navigator.push(Route::Settings {});
                                },
                                "Zu den Einstellungen"
                            }
                        }
                    },
                    Some(Err(e)) => rsx! { p { class: "text-sm text-destructive", "{e}" } },
                    Some(Ok(_)) => rsx! {
                        // Foto
                        if let Some(src) = image() {
                            div { class: "relative overflow-hidden rounded-2xl bg-muted",
                                img { src: "{src}", class: if has_result { "max-h-36 w-full object-cover" } else { "max-h-72 w-full object-cover" }, alt: "Foto der Mahlzeit" }
                                if busy() {
                                    div { class: "absolute inset-0 flex items-center justify-center bg-black/40",
                                        div { class: "flex items-center gap-2 rounded-full bg-black/60 px-4 py-2 text-sm text-white",
                                            LoaderCircle { class: "size-4 animate-spin" } "{who} analysiert das Foto …"
                                        }
                                    }
                                }
                                if !busy() {
                                    button {
                                        "aria-label": "Foto entfernen",
                                        class: "absolute right-2 top-2 flex size-9 items-center justify-center rounded-full bg-black/60 text-white",
                                        onclick: move |_| { image.set(None); result.set(None); },
                                        X { class: "size-4" }
                                    }
                                }
                            }
                        } else if !has_result {
                            div { class: "grid grid-cols-2 gap-2",
                                label {
                                    class: "flex cursor-pointer flex-col items-center justify-center gap-2 rounded-2xl bg-primary px-3 py-6 text-primary-foreground transition-transform active:scale-95",
                                    Camera { class: "size-7" }
                                    span { class: "text-sm font-semibold", "Foto aufnehmen" }
                                    input {
                                        id: "nrz-ai-camera",
                                        r#type: "file",
                                        accept: "image/*",
                                        "capture": "environment",
                                        class: "sr-only",
                                        onchange: move |_| on_pick("nrz-ai-camera"),
                                    }
                                }
                                label {
                                    class: "flex cursor-pointer flex-col items-center justify-center gap-2 rounded-2xl bg-muted px-3 py-6 transition-transform active:scale-95",
                                    ImagePlus { class: "size-7" }
                                    span { class: "text-sm font-semibold", "Aus Galerie" }
                                    input {
                                        id: "nrz-ai-gallery",
                                        r#type: "file",
                                        accept: "image/*",
                                        class: "sr-only",
                                        onchange: move |_| on_pick("nrz-ai-gallery"),
                                    }
                                }
                            }
                        }
                        if !has_result {
                            div { class: "flex flex-col gap-1.5",
                                span { class: "text-sm font-medium text-muted-foreground",
                                    if image().is_some() { "Ergänzungen (optional)" } else { "Oder beschreibe, was du gegessen hast" }
                                }
                                textarea {
                                    class: "{TEXTAREA} text-base",
                                    rows: 3,
                                    placeholder: if image().is_some() { "z.B. ohne Sauce, halbe Portion, mit Vollrahm …" } else { "z.B. Teller Spaghetti Bolognese mit Parmesan und ein Glas Rotwein" },
                                    value: "{text}",
                                    oninput: move |e| text.set(e.value()),
                                }
                                p { class: "px-1 text-xs text-muted-foreground",
                                    "Tipp: Fotografiere schräg von oben mit Besteck oder Hand im Bild – Mengenangaben und Marken machen die Schätzung genauer."
                                }
                            }
                            if busy() && image().is_none() {
                                div { class: "flex flex-col gap-2", Skeleton { class: "h-16" } Skeleton { class: "h-16" } }
                            }
                        }
                        // Ergebnis
                        if let Some(est) = result() {
                            div { class: "flex flex-col gap-3",
                                div { class: "rounded-2xl bg-muted/60 p-4",
                                    div { class: "flex items-start gap-3",
                                        div { class: "flex min-w-0 flex-1 flex-col",
                                            span { class: "truncate font-semibold", if est.title.is_empty() { "Mahlzeit" } else { "{est.title}" } }
                                            span { class: "text-xs text-muted-foreground", "{est.by}" }
                                        }
                                        span {
                                            class: format!("shrink-0 rounded-full px-2.5 py-1 text-[11px] font-medium {}", match est.confidence.as_str() { "hoch" => "bg-good/15 text-good", "niedrig" => "bg-bad/15 text-bad", _ => "bg-warn/15 text-warn" }),
                                            "Sicherheit {est.confidence}"
                                        }
                                    }
                                    div { class: "mt-3 flex items-end gap-4",
                                        div { class: "flex flex-col",
                                            span { class: "text-4xl font-bold tabular-nums leading-none", "{tk.round()}" }
                                            span { class: "text-xs text-muted-foreground", "kcal" }
                                        }
                                        div { class: "grid flex-1 grid-cols-3 gap-2 text-center text-sm tabular-nums",
                                            div { span { class: "block font-semibold text-protein", "{num(tp, 0)} g" } span { class: "text-[11px] text-muted-foreground", "Protein" } }
                                            div { span { class: "block font-semibold text-carbs", "{num(tc, 0)} g" } span { class: "text-[11px] text-muted-foreground", "Kohlenh." } }
                                            div { span { class: "block font-semibold text-fat", "{num(tf, 0)} g" } span { class: "text-[11px] text-muted-foreground", "Fett" } }
                                        }
                                    }
                                    if !est.note.is_empty() {
                                        p { class: "mt-3 text-xs text-muted-foreground", "{est.note}" }
                                    }
                                }
                                div { class: "flex flex-col gap-1.5",
                                    for (i, (it, on)) in items.clone().into_iter().enumerate() {
                                        div {
                                            key: "{i}",
                                            class: "flex items-center gap-2 rounded-xl border px-2 py-2",
                                            class: if !on { "opacity-50" },
                                            button {
                                                "aria-label": "Auswählen",
                                                class: "flex size-9 shrink-0 items-center justify-center rounded-lg",
                                                class: if on { "bg-primary text-primary-foreground" } else { "bg-muted" },
                                                onclick: move |_| { if let Some(e) = edits.write().get_mut(i) { e.1 = !e.1; } },
                                                if on { Check { class: "size-4" } }
                                            }
                                            div { class: "flex min-w-0 flex-1 flex-col",
                                                span { class: "truncate text-sm font-medium", "{it.name}" }
                                                span { class: "truncate text-xs tabular-nums text-muted-foreground",
                                                    "{it.kcal.round()} kcal · P {num(it.protein, 0)} · K {num(it.carbs, 0)} · F {num(it.fat, 0)}"
                                                }
                                            }
                                            div { class: "relative w-24 shrink-0",
                                                input {
                                                    class: "h-10 w-full rounded-lg border border-input bg-transparent pl-2 pr-8 text-right text-base tabular-nums outline-none focus-visible:ring-2 focus-visible:ring-ring/60",
                                                    inputmode: "decimal",
                                                    value: "{edits.read().get(i).map(|e| e.0.clone()).unwrap_or_default()}",
                                                    oninput: move |e| { if let Some(x) = edits.write().get_mut(i) { x.0 = e.value(); } },
                                                }
                                                span { class: "pointer-events-none absolute inset-y-0 right-2 flex items-center text-xs text-muted-foreground", "{it.unit}" }
                                            }
                                        }
                                    }
                                }
                                // Korrigieren
                                div { class: "flex items-center gap-2 rounded-2xl bg-muted px-3",
                                    input {
                                        class: "{GHOST_INPUT} h-12 text-base",
                                        placeholder: "Korrigieren, z.B. „Portion war kleiner“",
                                        value: "{refine}",
                                        disabled: busy(),
                                        onkeydown: move |e: KeyboardEvent| {
                                            if e.key() == Key::Enter && !refine().trim().is_empty() {
                                                let prev = result();
                                                run(prev, refine());
                                            }
                                        },
                                        oninput: move |e| refine.set(e.value()),
                                    }
                                    button {
                                        "aria-label": "Neu schätzen",
                                        class: "flex size-10 shrink-0 items-center justify-center rounded-xl bg-background disabled:opacity-40",
                                        disabled: busy() || refine().trim().is_empty(),
                                        onclick: move |_| { let prev = result(); run(prev, refine()); },
                                        if busy() { LoaderCircle { class: "size-4 animate-spin" } } else { RefreshCw { class: "size-4" } }
                                    }
                                }
                                Button { variant: ButtonVariant::Ghost, class: "self-center text-muted-foreground",
                                    onclick: move |_| { result.set(None); image.set(None); text.set(String::new()); },
                                    "Neue Schätzung"
                                }
                            }
                        }
                    },
                }
            }
        }
    }
}

//! Einstellungen → KI-Assistent: Anbieter, Schlüssel, Modell, Test.

use dioxus::prelude::*;
use icons::{Check, KeyRound, LoaderCircle, RefreshCw, Sparkles, Terminal};

use crate::api::ai::{ai_models, ai_test, get_ai_config, save_ai_config};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::field::{Field, INPUT};
use crate::components::ui::skeleton::Skeleton;
use crate::model::{AI_PROVIDERS, AiConfig, AiConfigInput, CLI_PRESETS, provider_default_model};
use crate::state::use_app;

#[component]
pub fn AiCard() -> Element {
    let cfg = use_resource(get_ai_config);
    match &*cfg.read() {
        Some(Ok(c)) => rsx! { AiForm { cfg: c.clone() } },
        Some(Err(e)) => rsx! { p { class: "text-sm text-destructive", "{e}" } },
        None => rsx! { Skeleton { class: "h-40 rounded-2xl" } },
    }
}

#[component]
fn AiForm(cfg: AiConfig) -> Element {
    let mut app = use_app();
    let mut current = use_signal(|| cfg.clone());
    let mut provider = use_signal(|| if cfg.provider.is_empty() { "anthropic".to_string() } else { cfg.provider.clone() });
    let mut model = use_signal(|| cfg.model.clone());
    let mut base_url = use_signal(|| cfg.base_url.clone());
    let mut command = use_signal(|| if cfg.command.is_empty() { CLI_PRESETS[0].1.to_string() } else { cfg.command.clone() });
    let mut key = use_signal(String::new);
    let mut models = use_signal(Vec::<String>::new);
    let mut busy = use_signal(|| None::<&'static str>);
    let mut test_result = use_signal(|| None::<Result<String, String>>);

    let p = provider();
    let info = AI_PROVIDERS.iter().find(|x| x.0 == p).copied().unwrap_or(AI_PROVIDERS[0]);
    let same = current().provider == p;
    let needs_key = !matches!(p.as_str(), "cli");

    // Speichern; gibt die neuen Einstellungen zurück.
    let persist = move || async move {
        let k = key();
        let input = AiConfigInput {
            provider: provider(),
            model: model(),
            base_url: base_url(),
            command: if provider() == "cli" { command() } else { String::new() },
            api_key: if k.trim().is_empty() { None } else { Some(k) },
        };
        let r = save_ai_config(input).await;
        if let Ok(c) = &r {
            current.set(c.clone());
            key.set(String::new());
        }
        r
    };

    let save = move |_| {
        busy.set(Some("save"));
        spawn(async move {
            match persist().await {
                Ok(c) if c.ready => app.toast("KI-Assistent gespeichert"),
                Ok(_) => app.toast("Gespeichert – es fehlen noch Angaben"),
                Err(e) => app.error(e),
            }
            busy.set(None);
        });
    };
    let load_models = move |_| {
        busy.set(Some("models"));
        spawn(async move {
            if let Err(e) = persist().await {
                app.error(e);
            } else {
                match ai_models().await {
                    Ok(list) if list.is_empty() => app.toast("Keine Modelle gefunden"),
                    Ok(list) => {
                        app.toast(format!("{} Modelle geladen", list.len()));
                        models.set(list);
                    }
                    Err(e) => app.error(e),
                }
            }
            busy.set(None);
        });
    };
    let test = move |_| {
        busy.set(Some("test"));
        test_result.set(None);
        spawn(async move {
            let r = match persist().await {
                Err(e) => Err(crate::state::clean_err(e)),
                Ok(c) if !c.ready => Err("Es fehlen noch Angaben (Schlüssel, Modell oder Befehl).".into()),
                Ok(_) => match ai_test().await {
                    Ok(est) => {
                        let k: f64 = est.items.iter().map(|i| i.kcal).sum();
                        Ok(format!("Funktioniert: 1 Apfel ≈ {} kcal ({})", k.round(), est.by))
                    }
                    Err(e) => Err(crate::state::clean_err(e)),
                },
            };
            test_result.set(Some(r));
            busy.set(None);
        });
    };

    let status = if current().ready { "eingerichtet" } else { "nicht eingerichtet" };
    let placeholder_model = match provider_default_model(&p) {
        "" => "Modellname".to_string(),
        m => format!("Standard: {m}"),
    };

    rsx! {
        Card { class: "flex flex-col",
            CardHeader {
                title: "KI-Assistent",
                subtitle: "Kalorien aus Fotos und Beschreibungen schätzen · {status}",
                icon: rsx! { Sparkles { class: "size-4 text-kcal" } },
            }
            div { class: "flex flex-col gap-4 px-4 pb-4",
                div { class: "grid grid-cols-3 gap-1.5",
                    for (k, label, _, _) in AI_PROVIDERS {
                        button {
                            key: "{k}",
                            class: "flex h-11 items-center justify-center gap-1.5 rounded-xl px-1 text-sm transition-transform active:scale-95",
                            class: if p == k { "bg-primary font-semibold text-primary-foreground" } else { "bg-muted text-muted-foreground" },
                            onclick: move |_| {
                                if provider() != k {
                                    provider.set(k.to_string());
                                    model.set(String::new());
                                    models.set(Vec::new());
                                    test_result.set(None);
                                }
                            },
                            if k == "cli" { Terminal { class: "size-3.5" } }
                            "{label}"
                        }
                    }
                }
                p { class: "-mt-2 px-1 text-xs text-muted-foreground", "{info.3}" }

                if p == "custom" {
                    Field { label: "Adresse (OpenAI-kompatibel)",
                        input { class: "{INPUT} text-base sm:text-sm", inputmode: "url", autocomplete: "off", placeholder: "z.B. http://192.168.1.10:11434/v1", value: "{base_url}", oninput: move |e| base_url.set(e.value()) }
                    }
                }
                if needs_key {
                    Field {
                        label: if p == "custom" { "API-Schlüssel (optional)" } else { "API-Schlüssel" },
                        hint: if same && current().has_key {
                            if current().key_hint.is_empty() { "Gespeichert – leer lassen, um ihn zu behalten.".to_string() } else { format!("Gespeichert (…{}) – leer lassen, um ihn zu behalten.", current().key_hint) }
                        } else { "Bleibt auf deinem Server und wird nie an den Browser zurückgegeben.".to_string() },
                        div { class: "relative",
                            KeyRound { class: "pointer-events-none absolute left-3 top-1/2 size-4 -translate-y-1/2 text-muted-foreground" }
                            input {
                                class: "{INPUT} pl-9 text-base sm:text-sm",
                                r#type: "password",
                                autocomplete: "off",
                                placeholder: if same && current().has_key { "••••••••" } else { "Schlüssel einfügen" },
                                value: "{key}",
                                oninput: move |e| key.set(e.value()),
                            }
                        }
                    }
                }
                if p == "cli" {
                    Field { label: "Befehl", hint: "Die Anweisung kommt über stdin, das Foto als Datei (Pfad in der Anweisung oder per {{image}}). Das Programm muss dort laufen, wo nrzFitness läuft, und angemeldet sein.",
                        input { class: "{INPUT} font-mono text-base sm:text-sm", autocomplete: "off", spellcheck: "false", value: "{command}", oninput: move |e| command.set(e.value()) }
                    }
                    div { class: "-mt-2 flex flex-wrap gap-1.5",
                        for (label, cmd) in CLI_PRESETS {
                            button { key: "{label}", class: "rounded-full bg-muted px-3 py-1.5 text-xs", onclick: move |_| command.set(cmd.to_string()), "{label}" }
                        }
                    }
                } else {
                    Field { label: "Modell",
                        div { class: "flex gap-2",
                            input {
                                class: "{INPUT} flex-1 font-mono text-base sm:text-sm",
                                list: "nrz-ai-models",
                                autocomplete: "off",
                                spellcheck: "false",
                                placeholder: "{placeholder_model}",
                                value: "{model}",
                                oninput: move |e| model.set(e.value()),
                            }
                            datalist { id: "nrz-ai-models",
                                for m in models() { option { key: "{m}", value: "{m}" } }
                            }
                            button {
                                "aria-label": "Modelle laden",
                                title: "Modelle vom Anbieter laden",
                                class: "flex size-11 shrink-0 items-center justify-center rounded-xl bg-muted disabled:opacity-50 sm:size-10",
                                disabled: busy().is_some(),
                                onclick: load_models,
                                if busy() == Some("models") { LoaderCircle { class: "size-4 animate-spin" } } else { RefreshCw { class: "size-4" } }
                            }
                        }
                    }
                    if !models().is_empty() {
                        div { class: "-mt-2 flex max-h-32 flex-wrap gap-1.5 overflow-y-auto",
                            for m in models() {
                                {
                                    let mm = m.clone();
                                    rsx! {
                                        button {
                                            key: "{m}",
                                            class: "rounded-full px-2.5 py-1 font-mono text-[11px]",
                                            class: if model() == m { "bg-primary text-primary-foreground" } else { "bg-muted text-muted-foreground" },
                                            onclick: move |_| model.set(mm.clone()),
                                            "{m}"
                                        }
                                    }
                                }
                            }
                        }
                    }
                }

                if let Some(r) = test_result() {
                    match r {
                        Ok(m) => rsx! { p { class: "flex items-center gap-2 rounded-xl bg-good/10 px-3 py-2 text-sm text-good", Check { class: "size-4 shrink-0" } "{m}" } },
                        Err(m) => rsx! { p { class: "rounded-xl bg-bad/10 px-3 py-2 text-sm text-bad", "{m}" } },
                    }
                }
                div { class: "grid grid-cols-2 gap-2",
                    Button { variant: ButtonVariant::Secondary, disabled: busy().is_some(), onclick: test,
                        if busy() == Some("test") { LoaderCircle { class: "size-4 animate-spin" } "Teste …" } else { "Testen" }
                    }
                    Button { disabled: busy().is_some(), onclick: save, "Speichern" }
                }
            }
        }
    }
}

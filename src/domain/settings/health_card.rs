//! Einstellungen → Apple Health & Google Health Connect.

use dioxus::prelude::*;
use icons::{Apple, Check, Copy, HeartPulse, LoaderCircle, RefreshCw, Smartphone, Upload};
use serde_json::Value;

use crate::api::health::{health_set_token, health_status};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::segmented::Segmented;
use crate::state::use_app;
use crate::util::today;

/// Datei per XHR hochladen und Fortschritt melden (`{p}` … `{done, ok, summary|error}`).
const UPLOAD: &str = r#"
const [id, url] = await dioxus.recv();
const input = document.getElementById(id);
const f = input && input.files && input.files[0];
if (!f) { dioxus.send({ done: true, cancel: true }); return; }
dioxus.send({ p: 0, size: f.size });
const xhr = new XMLHttpRequest();
xhr.open('POST', url);
xhr.upload.onprogress = (e) => { if (e.lengthComputable) dioxus.send({ p: e.loaded / e.total }); };
xhr.upload.onload = () => dioxus.send({ p: 1, processing: true });
xhr.onload = () => {
  let j;
  try { j = JSON.parse(xhr.responseText); } catch (_) { j = { ok: false, error: 'HTTP ' + xhr.status }; }
  dioxus.send(Object.assign({ done: true }, j));
};
xhr.onerror = () => dioxus.send({ done: true, ok: false, error: 'Verbindung unterbrochen' });
xhr.send(f);
input.value = '';
"#;

const COPY: &str = r#"
const t = await dioxus.recv();
try { await navigator.clipboard.writeText(t); }
catch (_) {
  const a = document.createElement('textarea');
  a.value = t; a.style.position = 'fixed'; a.style.opacity = '0';
  document.body.appendChild(a); a.select(); document.execCommand('copy'); a.remove();
}
"#;

fn copy(text: String) {
    let ev = document::eval(COPY);
    let _ = ev.send(text);
}

#[derive(Clone, PartialEq)]
enum Up {
    Idle,
    Uploading(f64, f64),
    Processing(f64),
}

#[component]
pub fn HealthCard() -> Element {
    let mut app = use_app();
    let mut status = use_resource(move || async move { health_status(app.pid_tracked()).await });
    let origin = use_resource(|| async {
        document::eval("return window.location.origin;").await.ok().and_then(|v| v.as_str().map(String::from)).unwrap_or_default()
    });
    let mut range = use_signal(|| "365".to_string());
    let mut up = use_signal(|| Up::Idle);
    let mut guide = use_signal(|| "iphone".to_string());
    let mut busy = use_signal(|| false);

    let on_file = move |_| {
        let since = match range().as_str() {
            "all" => String::new(),
            d => {
                let days: i64 = d.parse().unwrap_or(365);
                format!("&since={}", today() - chrono::Duration::days(days))
            }
        };
        let url = format!("/api/health/import/apple?pid={}{since}", app.pid());
        spawn(async move {
            let mut ev = document::eval(UPLOAD);
            let _ = ev.send(serde_json::json!(["nrz-health-file", url]));
            let mut size = 0.0;
            loop {
                let Ok(msg) = ev.recv::<Value>().await else {
                    app.error("Upload fehlgeschlagen");
                    break;
                };
                if let Some(s) = msg.get("size").and_then(Value::as_f64) {
                    size = s;
                }
                if msg.get("done").and_then(Value::as_bool) == Some(true) {
                    if msg.get("cancel").is_some() {
                    } else if msg.get("ok").and_then(Value::as_bool) == Some(true) {
                        app.bump();
                        app.toast(format!("Apple Health importiert: {}", msg.get("summary").and_then(Value::as_str).unwrap_or("")));
                        status.restart();
                    } else {
                        app.error(msg.get("error").and_then(Value::as_str).unwrap_or("Import fehlgeschlagen").to_string());
                    }
                    break;
                }
                let p = msg.get("p").and_then(Value::as_f64).unwrap_or(0.0);
                if msg.get("processing").is_some() {
                    up.set(Up::Processing(size));
                } else {
                    up.set(Up::Uploading(p, size));
                }
            }
            up.set(Up::Idle);
        });
    };

    let mut set_token = move |enable: bool| {
        busy.set(true);
        let pid = app.pid();
        spawn(async move {
            match health_set_token(pid, enable).await {
                Ok(_) => status.restart(),
                Err(e) => app.error(e),
            }
            busy.set(false);
        });
    };

    let st = status.read().clone();
    let token = match &st {
        Some(Ok(s)) => s.token.clone(),
        _ => None,
    };
    let base = origin.read().clone().unwrap_or_default();
    let mb = |b: f64| format!("{:.0} MB", b / 1_048_576.0);

    rsx! {
        Card { class: "flex flex-col",
            CardHeader {
                title: "Apple Health & Google Health Connect",
                subtitle: "Schritte, Gewicht, Körperfett, Schlaf, Ruhepuls, Aktivkalorien und Trainings übernehmen",
                icon: rsx! { HeartPulse { class: "size-4 text-bad" } },
            }
            div { class: "flex flex-col gap-5 px-4 pb-4",
                // --- Apple-Export
                div { class: "flex flex-col gap-3",
                    div { class: "flex items-center gap-2",
                        Apple { class: "size-4" }
                        span { class: "text-sm font-semibold", "Apple-Health-Export importieren" }
                    }
                    ol { class: "list-decimal space-y-1 pl-5 text-xs text-muted-foreground",
                        li { "Health-App öffnen → oben rechts dein Profilbild → „Alle Gesundheitsdaten exportieren“." }
                        li { "„export.zip“ in „Dateien“ sichern (dauert je nach Datenmenge eine Weile)." }
                        li { "Hier Zeitraum wählen und die ZIP-Datei auswählen. Mehrfaches Importieren erzeugt keine Doppelten." }
                    }
                    Segmented {
                        options: vec![("90".into(), "90 Tage".into()), ("365".into(), "1 Jahr".into()), ("all".into(), "Alles".into())],
                        value: range(),
                        onchange: move |v| range.set(v),
                    }
                    match up() {
                        Up::Idle => rsx! {
                            label { class: "flex h-12 cursor-pointer items-center justify-center gap-2 rounded-xl bg-primary text-sm font-semibold text-primary-foreground transition-transform active:scale-[0.98]",
                                Upload { class: "size-4" }
                                "export.zip auswählen"
                                input { id: "nrz-health-file", r#type: "file", accept: ".zip,.xml,application/zip,text/xml", class: "sr-only", onchange: on_file }
                            }
                        },
                        Up::Uploading(p, size) => rsx! {
                            div { class: "flex flex-col gap-1.5",
                                div { class: "h-2 overflow-hidden rounded-full bg-muted",
                                    div { class: "h-full rounded-full bg-primary transition-all", style: "width: {(p * 100.0).round()}%" }
                                }
                                p { class: "text-xs text-muted-foreground tabular-nums", "Hochladen … {(p * 100.0).round()} % von {mb(size)}" }
                            }
                        },
                        Up::Processing(size) => rsx! {
                            p { class: "flex items-center gap-2 rounded-xl bg-muted px-3 py-3 text-sm",
                                LoaderCircle { class: "size-4 animate-spin" }
                                "Werte {mb(size)} aus – das kann bei grossen Exporten eine Minute dauern …"
                            }
                        },
                    }
                }

                div { class: "h-px bg-border" }

                // --- Automatische Synchronisierung
                div { class: "flex flex-col gap-3",
                    div { class: "flex items-center gap-2",
                        RefreshCw { class: "size-4" }
                        span { class: "text-sm font-semibold", "Automatisch synchronisieren" }
                    }
                    p { class: "text-xs text-muted-foreground",
                        "iPhone (Kurzbefehle oder „Health Auto Export“) und Android (Health Connect über eine Webhook-/Automations-App) senden Daten an eine persönliche Adresse dieses Profils."
                    }
                    if let Some(t) = token.clone() {
                        {
                            let ingest = format!("{base}/api/health/hook/ingest?token={t}");
                            let day = format!("{base}/api/health/hook/day?token={t}");
                            rsx! {
                                UrlRow { label: "Daten senden (POST, JSON)", url: ingest.clone() }
                                UrlRow { label: "Nährwerte des Tages abholen (GET)", url: day.clone() }
                                Segmented {
                                    options: vec![("iphone".into(), "iPhone".into()), ("android".into(), "Android".into()), ("format".into(), "Format".into())],
                                    value: guide(),
                                    onchange: move |v| guide.set(v),
                                }
                                match guide().as_str() {
                                    "iphone" => rsx! {
                                        div { class: "flex flex-col gap-2 text-xs text-muted-foreground",
                                            p { span { class: "font-semibold text-foreground", "Am einfachsten: " } "App „Health Auto Export“ → Automationen → REST API → obige Senden-Adresse einfügen, Format JSON, Metriken wählen (Schritte, Gewicht, Schlaf, Aktivenergie, Ruhepuls) und Workouts aktivieren." }
                                            p { span { class: "font-semibold text-foreground", "Mit Kurzbefehlen (gratis): " } }
                                            ol { class: "list-decimal space-y-1 pl-5",
                                                li { "Neuer Kurzbefehl: „Health-Stichproben suchen“ → Schritte, Startdatum heute, gruppiert nach Tag." }
                                                li { "Optional weitere: Gewicht (neueste, Limit 1), Schlafanalyse." }
                                                li { "„Text“: {{\"source\":\"Kurzbefehl\",\"entries\":[{{\"type\":\"steps\",\"date\":\"[Aktuelles Datum, JJJJ-MM-TT]\",\"value\":[Schritte]}}]}}" }
                                                li { "„Inhalte von URL abrufen“: Senden-Adresse, Methode POST, Header Content-Type = application/json, Anfragetext = Datei → Text." }
                                                li { "Automation „Tageszeit“ (z.B. 22:00, täglich, sofort ausführen) startet den Kurzbefehl." }
                                            }
                                            p { span { class: "font-semibold text-foreground", "Kalorien zurück in Apple Health: " } "Kurzbefehl mit „Inhalte von URL abrufen“ (Abholen-Adresse) → „Wörterbuchwert für kcal“ → „Health-Stichprobe protokollieren“ (Nahrungsenergie). Gleich für Protein, Kohlenhydrate, Fett." }
                                        }
                                    },
                                    "android" => rsx! {
                                        div { class: "flex flex-col gap-2 text-xs text-muted-foreground",
                                            p { "Google Fit und Samsung Health speichern in Health Connect. Health Connect selbst sendet nichts ins Netz – dafür braucht es eine App, die Health-Connect-Daten per Webhook bzw. HTTP sendet (z.B. Automations-Apps wie Tasker oder spezielle „Health Connect → Webhook“-Apps)." }
                                            ol { class: "list-decimal space-y-1 pl-5",
                                                li { "In der App Lesezugriff auf Schritte, Gewicht, Schlaf, Aktivkalorien, Ruhepuls und Trainings erlauben." }
                                                li { "Als Ziel die Senden-Adresse eintragen (POST, JSON)." }
                                                li { "Daten im Format unter „Format“ senden; Trainingstypen wie EXERCISE_TYPE_RUNNING werden erkannt." }
                                                li { "Täglich oder stündlich auslösen – erneutes Senden überschreibt nur die Tageswerte." }
                                            }
                                        }
                                    },
                                    _ => rsx! {
                                        div { class: "flex flex-col gap-2 text-xs text-muted-foreground",
                                            p { "Tageswerte (Typen: steps, weight, body_fat, active_kcal, sleep_hours, resting_hr) und Trainings. Das Format von „Health Auto Export“ wird ebenfalls verstanden." }
                                            pre { class: "overflow-x-auto rounded-xl bg-muted p-3 font-mono text-[11px] leading-relaxed text-foreground",
"{{
  \"source\": \"Pixel\",
  \"entries\": [
    {{ \"type\": \"steps\", \"date\": \"2026-11-21\", \"value\": 8123 }},
    {{ \"type\": \"weight\", \"date\": \"2026-11-21\", \"value\": 84.2 }},
    {{ \"type\": \"sleep_hours\", \"date\": \"2026-11-21\", \"value\": 7.4 }}
  ],
  \"workouts\": [
    {{ \"id\": \"abc\", \"name\": \"EXERCISE_TYPE_RUNNING\", \"date\": \"2026-11-21\",
      \"minutes\": 32, \"kcal\": 310, \"distance_km\": 5.1 }}
  ]
}}"
                                            }
                                        }
                                    },
                                }
                                div { class: "grid grid-cols-2 gap-2",
                                    Button { variant: ButtonVariant::Secondary, disabled: busy(), onclick: move |_| set_token(true), "Neue Adresse" }
                                    Button { variant: ButtonVariant::Ghost, class: "text-destructive", disabled: busy(), onclick: move |_| set_token(false), "Trennen" }
                                }
                            }
                        }
                    } else {
                        Button { disabled: busy(), onclick: move |_| set_token(true), Smartphone { class: "size-4" } "Verbindung erstellen" }
                    }
                }

                if let Some(Ok(s)) = &st && !s.syncs.is_empty() {
                    div { class: "flex flex-col gap-1",
                        span { class: "text-xs font-semibold uppercase tracking-wide text-muted-foreground", "Letzte Übernahmen" }
                        for (i, sy) in s.syncs.iter().enumerate() {
                            div { key: "{i}", class: "flex items-start gap-2 rounded-lg px-1 py-1 text-xs",
                                Check { class: "mt-0.5 size-3.5 shrink-0 text-good" }
                                span { class: "flex-1",
                                    span { class: "font-medium", "{sy.source}" }
                                    span { class: "text-muted-foreground", " · {sy.summary}" }
                                }
                                span { class: "shrink-0 tabular-nums text-muted-foreground", "{sy.created_at.get(5..16).unwrap_or(\"\").replace('T', \" \")}" }
                            }
                        }
                    }
                }
            }
        }
    }
}

#[component]
fn UrlRow(label: &'static str, url: String) -> Element {
    let mut app = use_app();
    let u = url.clone();
    rsx! {
        div { class: "flex flex-col gap-1",
            span { class: "text-xs text-muted-foreground", "{label}" }
            div { class: "flex items-center gap-2",
                code { class: "min-w-0 flex-1 truncate rounded-lg bg-muted px-3 py-2.5 font-mono text-[11px]", "{url}" }
                button {
                    "aria-label": "Kopieren",
                    class: "flex size-10 shrink-0 items-center justify-center rounded-lg bg-muted active:scale-90",
                    onclick: move |_| { copy(u.clone()); app.toast("Adresse kopiert"); },
                    Copy { class: "size-4" }
                }
            }
        }
    }
}

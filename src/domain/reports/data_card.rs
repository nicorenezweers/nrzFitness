//! Einstellungen → Daten: JSON-Sicherung exportieren/importieren, CSV-Export.
// DataCard wird in den Einstellungen eingebunden; bis dahin sonst „unbenutzt“.
#![allow(dead_code)]

use dioxus::prelude::*;
use icons::{ChevronRight, Database, FileJson, FileSpreadsheet, LoaderCircle, Upload};

use crate::api::reports::{IMPORT_CHUNK, export_backup, export_checkins_csv, export_diary_csv, import_backup_chunk};
use crate::components::ui::button::{Button, ButtonVariant};
use crate::components::ui::card::{Card, CardHeader};
use crate::components::ui::sheet::Sheet;
use crate::state::use_app;
use crate::util::{date_input, int, today};

/// Lädt Text im Browser als Datei herunter (Blob + a[download], funktioniert auch auf iOS Safari).
pub fn download(name: &str, mime: &str, text: String) {
    let ev = document::eval(
        r#"
        const [name, mime, text] = await dioxus.recv();
        const blob = new Blob([text], { type: mime });
        const url = URL.createObjectURL(blob);
        const a = document.createElement('a');
        a.href = url;
        a.download = name;
        a.rel = 'noopener';
        a.style.display = 'none';
        document.body.appendChild(a);
        a.click();
        setTimeout(() => { URL.revokeObjectURL(url); a.remove(); }, 4000);
        "#,
    );
    let _ = ev.send(serde_json::json!([name, mime, text]));
}

/// Dateiname-tauglicher Profilname.
fn slug(s: &str) -> String {
    let t: String = s
        .to_lowercase()
        .chars()
        .map(|c| match c {
            'ä' => 'a',
            'ö' => 'o',
            'ü' => 'u',
            c if c.is_ascii_alphanumeric() => c,
            _ => '-',
        })
        .collect();
    let t = t.split('-').filter(|p| !p.is_empty()).collect::<Vec<_>>().join("-");
    if t.is_empty() { "profil".into() } else { t }
}

/// Teile eines Textes an Zeichengrenzen (höchstens `max` Bytes).
fn chunks(s: &str, max: usize) -> Vec<&str> {
    let mut out = Vec::new();
    let mut rest = s;
    while !rest.is_empty() {
        let mut end = rest.len().min(max);
        while !rest.is_char_boundary(end) {
            end -= 1;
        }
        out.push(&rest[..end]);
        rest = &rest[end..];
    }
    if out.is_empty() {
        out.push("");
    }
    out
}

/// Vorschau einer gewählten Sicherung vor dem Import.
#[derive(Clone, PartialEq)]
struct Pending {
    json: String,
    profile: String,
    exported: String,
    rows: usize,
    file: String,
}

fn inspect(file: String, json: String) -> Result<Pending, String> {
    let v: serde_json::Value =
        serde_json::from_str(&json).map_err(|_| "Die Datei ist kein gültiges JSON.".to_string())?;
    if v.get("app").and_then(|a| a.as_str()) != Some("nrzFitness") {
        return Err("Das ist keine nrzFitness-Sicherung.".into());
    }
    let rows = v
        .get("tables")
        .and_then(|t| t.as_object())
        .map(|t| {
            t.iter()
                .filter(|(k, _)| k.as_str() != "profiles")
                .map(|(_, r)| r.as_array().map(Vec::len).unwrap_or(0))
                .sum()
        })
        .unwrap_or(0);
    let profile = v.get("profile_name").and_then(|a| a.as_str()).unwrap_or("?").to_string();
    let exported = v
        .get("exported_at")
        .and_then(|a| a.as_str())
        .and_then(crate::util::parse_datetime)
        .map(|d| d.format("%d.%m.%Y, %H:%M").to_string())
        .unwrap_or_else(|| "unbekannt".into());
    Ok(Pending { json, profile, exported, rows, file })
}

#[component]
pub fn DataCard() -> Element {
    let mut app = use_app();
    let mut busy = use_signal(|| None::<&'static str>);
    let mut pending = use_signal(|| None::<Pending>);
    let mut progress = use_signal(|| None::<(usize, usize)>);

    let mut run_export = move |kind: &'static str| {
        if busy().is_some() {
            return;
        }
        busy.set(Some(kind));
        let pid = app.pid();
        let name = slug(&app.profile().map(|p| p.name).unwrap_or_default());
        let day = date_input(today());
        spawn(async move {
            let res = match kind {
                "json" => {
                    export_backup(pid).await.map(|t| (format!("nrzfitness-{name}-{day}.json"), "application/json", t))
                }
                "diary" => export_diary_csv(pid)
                    .await
                    .map(|t| (format!("nrzfitness-{name}-tagebuch-{day}.csv"), "text/csv;charset=utf-8", t)),
                _ => export_checkins_csv(pid)
                    .await
                    .map(|t| (format!("nrzfitness-{name}-checkins-{day}.csv"), "text/csv;charset=utf-8", t)),
            };
            match res {
                Ok((file, mime, text)) => {
                    download(&file, mime, text);
                    app.toast("Export erstellt");
                }
                Err(e) => app.error(e),
            }
            busy.set(None);
        });
    };

    let on_file = move |e: FormEvent| async move {
        let Some(f) = e.files().into_iter().next() else { return };
        busy.set(Some("read"));
        let name = f.name();
        match f.read_string().await {
            Ok(text) => match inspect(name, text) {
                Ok(p) => pending.set(Some(p)),
                Err(m) => app.error(m),
            },
            Err(_) => app.error("Datei konnte nicht gelesen werden"),
        }
        busy.set(None);
        // Auswahl zurücksetzen, damit dieselbe Datei erneut gewählt werden kann.
        document::eval("const i = document.getElementById('nrz-backup-file'); if (i) i.value = '';");
    };

    let do_import = move |_| {
        let Some(p) = pending() else { return };
        busy.set(Some("import"));
        let pid = app.pid();
        spawn(async move {
            let parts = chunks(&p.json, IMPORT_CHUNK);
            let total = parts.len();
            let upload = uuid::Uuid::new_v4().to_string();
            let mut result = None;
            for (i, part) in parts.iter().enumerate() {
                progress.set(Some((i + 1, total)));
                match import_backup_chunk(pid.clone(), upload.clone(), i as u32, total as u32, part.to_string()).await {
                    Ok(r) if r.done => result = Some(r),
                    Ok(_) => {}
                    Err(e) => {
                        app.error(e);
                        break;
                    }
                }
            }
            progress.set(None);
            busy.set(None);
            if let Some(r) = result {
                pending.set(None);
                let msg = if r.skipped > 0 {
                    format!("{} Einträge importiert, {} übersprungen", int(r.imported as f64), int(r.skipped as f64))
                } else {
                    format!("{} Einträge importiert", int(r.imported as f64))
                };
                app.toast(msg);
                // Profil (Name, Einstellungen) kann sich geändert haben → Profile neu laden.
                if let Ok(list) = crate::api::profile::list_profiles().await {
                    let cur = app.pid();
                    if let Some(p) = list.iter().find(|p| p.id == cur).cloned() {
                        app.profile.set(Some(p));
                    }
                    app.profiles.set(list);
                }
                app.bump();
            }
        });
    };

    let b = busy();
    rsx! {
        Card {
            CardHeader {
                title: "Daten",
                subtitle: "Sicherung, Export und Import des aktiven Profils",
                icon: rsx! { Database { class: "size-4" } },
            }
            div { class: "flex flex-col px-2 pb-2",
                DataRow {
                    title: "Sicherung exportieren",
                    desc: "Alle Daten als JSON-Datei",
                    busy: b == Some("json"),
                    onclick: move |_| run_export("json"),
                    FileJson { class: "size-5" }
                }
                DataRow {
                    title: "Tagebuch als CSV",
                    desc: "Alle Mahlzeiten-Einträge, z.B. für Excel",
                    busy: b == Some("diary"),
                    onclick: move |_| run_export("diary"),
                    FileSpreadsheet { class: "size-5" }
                }
                DataRow {
                    title: "Check-ins als CSV",
                    desc: "Gewicht, Masse, Schlaf, Stimmung …",
                    busy: b == Some("checkins"),
                    onclick: move |_| run_export("checkins"),
                    FileSpreadsheet { class: "size-5" }
                }
                // Import: echtes Datei-Feld im Label → öffnet die Dateiauswahl auch auf iOS zuverlässig.
                label {
                    class: "flex min-h-14 cursor-pointer items-center gap-3 rounded-xl px-2 py-2.5 transition-colors hover:bg-accent/50 active:bg-accent",
                    class: if b.is_some() { "pointer-events-none opacity-60" },
                    div { class: "flex size-10 shrink-0 items-center justify-center rounded-xl bg-muted",
                        if b == Some("read") || b == Some("import") {
                            LoaderCircle { class: "size-5 animate-spin" }
                        } else {
                            Upload { class: "size-5" }
                        }
                    }
                    div { class: "min-w-0 flex-1",
                        p { class: "truncate text-sm font-medium", "Sicherung importieren" }
                        p { class: "truncate text-xs text-muted-foreground", "JSON-Datei in dieses Profil einspielen" }
                    }
                    ChevronRight { class: "size-4 shrink-0 text-muted-foreground" }
                    input {
                        id: "nrz-backup-file",
                        r#type: "file",
                        accept: ".json,application/json",
                        class: "sr-only",
                        disabled: b.is_some(),
                        onchange: on_file,
                    }
                }
            }
        }
        if let Some(p) = pending() {
            Sheet {
                title: "Sicherung importieren",
                onclose: move |_| if busy().is_none() { pending.set(None) },
                footer: rsx! {
                    div { class: "flex gap-2",
                        Button {
                            variant: ButtonVariant::Outline,
                            class: "h-12 flex-1",
                            disabled: busy().is_some(),
                            onclick: move |_| pending.set(None),
                            "Abbrechen"
                        }
                        Button { class: "h-12 flex-1", disabled: busy().is_some(), onclick: do_import,
                            if let Some((i, n)) = progress() {
                                LoaderCircle { class: "size-4 animate-spin" }
                                if n > 1 { "Teil {i}/{n} …" } else { "Importiere …" }
                            } else {
                                "Importieren"
                            }
                        }
                    }
                },
                div { class: "flex flex-col gap-3 px-5 pb-4 pt-1 text-sm",
                    div { class: "rounded-xl bg-muted p-3",
                        p { class: "truncate font-medium", "{p.file}" }
                        p { class: "text-xs text-muted-foreground", "Profil „{p.profile}“ · erstellt {p.exported}" }
                        p { class: "text-xs text-muted-foreground", "{int(p.rows as f64)} Datensätze" }
                    }
                    p { class: "text-muted-foreground",
                        "Die Daten werden in das aktive Profil "
                        span { class: "font-medium text-foreground", "„{app.profile().map(|p| p.name).unwrap_or_default()}“" }
                        " übernommen. Einträge mit gleicher ID werden ersetzt, alles andere bleibt erhalten. Profil-Einstellungen und Ziele werden aus der Sicherung übernommen."
                    }
                }
            }
        }
    }
}

#[component]
fn DataRow(
    #[props(into)] title: String,
    #[props(into)] desc: String,
    busy: bool,
    onclick: EventHandler<MouseEvent>,
    children: Element,
) -> Element {
    rsx! {
        button {
            r#type: "button",
            class: "flex min-h-14 w-full items-center gap-3 rounded-xl px-2 py-2.5 text-left transition-colors hover:bg-accent/50 active:bg-accent disabled:opacity-60",
            disabled: busy,
            onclick: move |e| onclick.call(e),
            div { class: "flex size-10 shrink-0 items-center justify-center rounded-xl bg-muted",
                if busy {
                    LoaderCircle { class: "size-5 animate-spin" }
                } else {
                    {children}
                }
            }
            div { class: "min-w-0 flex-1",
                p { class: "truncate text-sm font-medium", "{title}" }
                p { class: "truncate text-xs text-muted-foreground", "{desc}" }
            }
            ChevronRight { class: "size-4 shrink-0 text-muted-foreground" }
        }
    }
}

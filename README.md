# nrzFitness

Selbst gehosteter Fitness- und Gesundheits-Tracker – komplett in Rust (Dioxus 0.7 fullstack, SQLite,
nrzUI-Thema), gebaut fürs Handy und mit voller Desktop-Ansicht. Nachbau von
[SparkyFitness](https://github.com/CodeWithCJ/SparkyFitness).

## Funktionen

- **Heute** – Kalorienring (Ziel − gegessen + verbrannt), Makros, Wasser, Mahlzeiten, Training, Schritte,
  Schlaf, Gewicht, Fasten; Wochenleiste und Wischen zum Tageswechsel.
- **Essen eintragen** – über 120 Basis-Lebensmittel, Open-Food-Facts-Suche, Barcode-Scanner (Kamera,
  BarcodeDetector bzw. ZXing auf iOS), Favoriten, Zuletzt, Mahlzeiten-Vorlagen, Schnelleintrag, „Wie gestern“.
- **KI-Schätzung** – Mahlzeit fotografieren oder beschreiben; Claude, Grok, ChatGPT, Gemini, ein eigener
  OpenAI-kompatibler Server (Ollama, OpenRouter) oder ein CLI-Befehl (z.B. `claude -p`) zerlegt sie in
  Positionen mit Mengen und Nährwerten, korrigierbar per Satz. Einrichtung unter Einstellungen → KI-Assistent.
- **Training** – über 100 Übungen, Sätze (Wdh × kg), Vorlagen, Ruhe-Timer, kcal über MET, Rekorde/1RM, Verlauf.
- **Körper** – Check-in mit Gewicht, Körperfett, Umfängen, Schritten, Stimmung, Schlaf, Ruhepuls; Diagramme, BMI.
- **Fasten** – Live-Timer mit Phasen, Presets, Verlauf und Statistik.
- **Zyklus** – Periodenkalender mit Vorhersage (pro Profil einschaltbar).
- **Medikamente & Symptome** – Einnahme-Checkliste, Einhaltung, Symptom-Tagebuch.
- **Ziele** – Bedarfsrechner (Mifflin-St Jeor), Makro-Verteilungen.
- **Berichte** – 7 Tage bis 1 Jahr, Trends, Heatmap, Serien.
- **Profile & Daten** – mehrere Profile, JSON-Sicherung, CSV-Export, Import.

## Entwickeln

```bash
pnpm install && pnpm css:build
dx serve                      # http://127.0.0.1:8080
cargo test --features server  # DB-Tests gegen In-Memory-SQLite
```

Umgebung: `DATABASE_URL` (Standard `sqlite://data/nrzfitness.db`), `PORT`, `OFF_URL`/`OFF_SEARCH_URL`
(Open-Food-Facts-Endpunkte).

## Umbrel

App `nrz-fitness` im nrzStore (Port 8630). Release: `./scripts/umbrel-release <version> "Notizen"` –
baut amd64/arm64, legt die Dateien auf den Zweig `umbrel` und pusht den Tag `umbrel-<version>`;
nrzGit aktualisiert den Store über `nrzStore.json`.

Lebensmitteldaten online: Open Food Facts (ODbL). Barcode-Erkennung: ZXing (Apache-2.0, `public/vendor`).

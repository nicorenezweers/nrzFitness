# nrzFitness

Dioxus 0.7 fullstack + SQLite (sqlx), UI im nrzUI-Stil (dunkles nrz-Thema, nur Theme-Variablen wie `bg-card`,
`text-muted-foreground`; Datenfarben `text-kcal/protein/carbs/fat/water/good/warn/bad`). Nachbau von SparkyFitness.
Sprache der Oberfläche: Deutsch (Schweiz: „ss“ statt „ß“).

- Server prüfen: `cargo check --features server`; Client: `cargo check --target wasm32-unknown-unknown --features web`.
- Tailwind: `pnpm css:build` nach neuen Klassen (Quelle `style/tailwind.css` → `public/tailwind.css`).
- Aufbau pro Bereich: `src/model/<b>.rs` (Typen), `src/server/db/<b>.rs` (SQL, nur hier), `src/api/<b>.rs`
  (`#[server]`-Funktionen, rufen db auf), `src/domain/<b>/` (Seiten/Komponenten).
- Schema komplett in `migrations/20261120000000_init.sql`; neue Spalten/Tabellen nur über NEUE Migrationsdateien.
- Jede Server-Funktion bekommt `pid: String` (aktives Profil); in Komponenten `let app = use_app(); app.pid()`.
- Daten laden mit `use_resource` und `app.refresh` lesen (`let _ = (app.refresh)();`), damit globale Sheets
  (Essen, Schnellaktionen) die Seite neu laden; nach Änderungen `app.bump()` bzw. `res.restart()`.
- Meldungen: `app.toast("…")`, Fehler: `app.error(e)`.
- Mobil zuerst: Dialoge immer mit `components::ui::sheet::Sheet` (Bottom-Sheet), Tap-Ziele ≥ 44 px, Zahlen mit
  `NumberInput` (Dezimaltastatur), Seiten mit `PageHeader` + `PageBody`, Karten mit `Card`.
- Zeitstempel in der DB über `db::ts()`/`now_ts()` (`%Y-%m-%dT%H:%M:%S`), Datum als `NaiveDate`.
- Buttons in Formularen: `submit: true` nur für den Absenden-Button.
- Umbrel-Release nur mit `scripts/umbrel-release.sh` (App `nrz-fitness`, Port 8630).

//! Datenbank-Schema als eingebettete Migrationen (byte-genau wie die früheren `migrations/*.sql`,
//! damit bestehende Datenbanken die Prüfsummen wiedererkennen). Neue Änderungen: neuen Eintrag
//! ans Ende von `MIGRATIONS` anhängen, bestehende nie verändern.

use std::borrow::Cow;
use std::future::Future;
use std::pin::Pin;

use sqlx::error::BoxDynError;
use sqlx::migrate::{Migration, MigrationSource, MigrationType, Migrator};

/// (Version, Beschreibung, SQL)
pub const MIGRATIONS: &[(i64, &str, &str)] = &[
    (20261120000000, "init", INIT),
    (20261121000000, "training", TRAINING),
    (20261122000000, "ai", AI),
];

const INIT: &str = r#"-- nrzFitness: komplettes Schema. Datumsfelder als TEXT 'YYYY-MM-DD', Zeitstempel 'YYYY-MM-DDTHH:MM:SS'.

CREATE TABLE profiles (
    id             TEXT PRIMARY KEY,
    name           TEXT NOT NULL,
    color          TEXT NOT NULL DEFAULT 'emerald',
    sex            TEXT NOT NULL DEFAULT 'm',          -- 'm' | 'f'
    birth_date     TEXT,
    height_cm      REAL,
    activity_level TEXT NOT NULL DEFAULT 'light',      -- sedentary | light | moderate | active | very_active
    cycle_tracking INTEGER NOT NULL DEFAULT 0,
    created_at     TEXT NOT NULL
);

CREATE TABLE goals (
    profile_id     TEXT PRIMARY KEY REFERENCES profiles(id) ON DELETE CASCADE,
    calories       REAL NOT NULL DEFAULT 2000,
    protein_g      REAL NOT NULL DEFAULT 120,
    carbs_g        REAL NOT NULL DEFAULT 220,
    fat_g          REAL NOT NULL DEFAULT 70,
    fiber_g        REAL NOT NULL DEFAULT 30,
    sugar_g        REAL NOT NULL DEFAULT 50,
    water_ml       REAL NOT NULL DEFAULT 2500,
    steps          INTEGER NOT NULL DEFAULT 8000,
    exercise_min   INTEGER NOT NULL DEFAULT 30,
    sleep_hours    REAL NOT NULL DEFAULT 8,
    target_weight  REAL,
    weekly_change  REAL NOT NULL DEFAULT 0,             -- kg pro Woche (negativ = abnehmen)
    fasting_hours  REAL NOT NULL DEFAULT 16
);

-- Lebensmittel: Nährwerte pro 100 g bzw. 100 ml. profile_id NULL = für alle Profile.
CREATE TABLE foods (
    id           TEXT PRIMARY KEY,
    profile_id   TEXT REFERENCES profiles(id) ON DELETE CASCADE,
    name         TEXT NOT NULL,
    brand        TEXT,
    barcode      TEXT,
    unit         TEXT NOT NULL DEFAULT 'g',             -- 'g' | 'ml'
    serving_size REAL NOT NULL DEFAULT 100,
    serving_name TEXT,
    kcal         REAL NOT NULL DEFAULT 0,
    protein      REAL NOT NULL DEFAULT 0,
    carbs        REAL NOT NULL DEFAULT 0,
    fat          REAL NOT NULL DEFAULT 0,
    fiber        REAL NOT NULL DEFAULT 0,
    sugar        REAL NOT NULL DEFAULT 0,
    sat_fat      REAL NOT NULL DEFAULT 0,
    sodium_mg    REAL NOT NULL DEFAULT 0,
    source       TEXT NOT NULL DEFAULT 'custom',        -- 'custom' | 'off'
    image_url    TEXT,
    created_at   TEXT NOT NULL
);
CREATE INDEX idx_foods_barcode ON foods(barcode);
CREATE INDEX idx_foods_name ON foods(name COLLATE NOCASE);

CREATE TABLE food_favorites (
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    food_id    TEXT NOT NULL REFERENCES foods(id) ON DELETE CASCADE,
    PRIMARY KEY (profile_id, food_id)
);

-- Mahlzeiten-Vorlagen (Rezepte): mehrere Lebensmittel auf einmal eintragen.
CREATE TABLE meals (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    notes      TEXT,
    created_at TEXT NOT NULL
);
CREATE TABLE meal_items (
    id       TEXT PRIMARY KEY,
    meal_id  TEXT NOT NULL REFERENCES meals(id) ON DELETE CASCADE,
    food_id  TEXT NOT NULL REFERENCES foods(id) ON DELETE CASCADE,
    amount   REAL NOT NULL,
    position INTEGER NOT NULL DEFAULT 0
);

-- Tagebuch-Einträge: Nährwerte als Schnappschuss (bleiben beim Ändern des Lebensmittels gleich).
CREATE TABLE food_entries (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date       TEXT NOT NULL,
    meal       TEXT NOT NULL,                           -- breakfast | lunch | dinner | snack
    food_id    TEXT REFERENCES foods(id) ON DELETE SET NULL,
    name       TEXT NOT NULL,
    brand      TEXT,
    unit       TEXT NOT NULL DEFAULT 'g',
    amount     REAL NOT NULL,
    kcal       REAL NOT NULL DEFAULT 0,
    protein    REAL NOT NULL DEFAULT 0,
    carbs      REAL NOT NULL DEFAULT 0,
    fat        REAL NOT NULL DEFAULT 0,
    fiber      REAL NOT NULL DEFAULT 0,
    sugar      REAL NOT NULL DEFAULT 0,
    sodium_mg  REAL NOT NULL DEFAULT 0,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_food_entries_day ON food_entries(profile_id, date);

CREATE TABLE water_entries (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date       TEXT NOT NULL,
    ml         REAL NOT NULL,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_water_day ON water_entries(profile_id, date);

-- Übungsbibliothek. profile_id NULL = mitgeliefert/für alle.
CREATE TABLE exercises (
    id         TEXT PRIMARY KEY,
    profile_id TEXT REFERENCES profiles(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    category   TEXT NOT NULL DEFAULT 'strength',        -- strength | cardio | flexibility | sport
    muscle     TEXT,
    equipment  TEXT,
    met        REAL NOT NULL DEFAULT 5,
    created_at TEXT NOT NULL
);

CREATE TABLE exercise_entries (
    id           TEXT PRIMARY KEY,
    profile_id   TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date         TEXT NOT NULL,
    exercise_id  TEXT REFERENCES exercises(id) ON DELETE SET NULL,
    name         TEXT NOT NULL,
    category     TEXT NOT NULL DEFAULT 'strength',
    duration_min REAL NOT NULL DEFAULT 0,
    kcal         REAL NOT NULL DEFAULT 0,
    distance_km  REAL,
    notes        TEXT,
    created_at   TEXT NOT NULL
);
CREATE INDEX idx_exercise_entries_day ON exercise_entries(profile_id, date);

CREATE TABLE exercise_sets (
    id        TEXT PRIMARY KEY,
    entry_id  TEXT NOT NULL REFERENCES exercise_entries(id) ON DELETE CASCADE,
    position  INTEGER NOT NULL DEFAULT 0,
    reps      INTEGER NOT NULL DEFAULT 0,
    weight_kg REAL NOT NULL DEFAULT 0,
    done      INTEGER NOT NULL DEFAULT 1
);

-- Täglicher Check-in: Körpermasse, Schritte, Stimmung, Schlaf (alles optional).
CREATE TABLE checkins (
    profile_id    TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date          TEXT NOT NULL,
    weight        REAL,
    body_fat      REAL,
    waist         REAL,
    hips          REAL,
    chest         REAL,
    neck          REAL,
    arm           REAL,
    thigh         REAL,
    steps         INTEGER,
    mood          INTEGER,                              -- 1..5
    energy        INTEGER,                              -- 1..5
    stress        INTEGER,                              -- 1..5
    sleep_hours   REAL,
    sleep_quality INTEGER,                              -- 1..5
    resting_hr    INTEGER,
    notes         TEXT,
    PRIMARY KEY (profile_id, date)
);

CREATE TABLE fasts (
    id           TEXT PRIMARY KEY,
    profile_id   TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    start_at     TEXT NOT NULL,
    end_at       TEXT,
    target_hours REAL NOT NULL DEFAULT 16,
    notes        TEXT
);
CREATE INDEX idx_fasts_profile ON fasts(profile_id, start_at);

CREATE TABLE medications (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    dose       TEXT,
    times      TEXT NOT NULL DEFAULT '08:00',           -- kommagetrennt, z.B. '08:00,20:00'
    active     INTEGER NOT NULL DEFAULT 1,
    notes      TEXT,
    created_at TEXT NOT NULL
);

CREATE TABLE medication_logs (
    id            TEXT PRIMARY KEY,
    medication_id TEXT NOT NULL REFERENCES medications(id) ON DELETE CASCADE,
    profile_id    TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date          TEXT NOT NULL,
    slot          TEXT NOT NULL,                        -- geplante Uhrzeit, z.B. '08:00'
    taken_at      TEXT NOT NULL,
    UNIQUE (medication_id, date, slot)
);

CREATE TABLE symptoms (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date       TEXT NOT NULL,
    name       TEXT NOT NULL,
    severity   INTEGER NOT NULL DEFAULT 3,              -- 1..5
    notes      TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_symptoms_day ON symptoms(profile_id, date);

-- Zyklus: ein Eintrag pro Periodentag.
CREATE TABLE cycle_days (
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    date       TEXT NOT NULL,
    flow       INTEGER NOT NULL DEFAULT 2,              -- 1 leicht, 2 mittel, 3 stark
    PRIMARY KEY (profile_id, date)
);
"#;

const TRAINING: &str = r#"-- Training: Vorlagen (Routinen) und Indizes für Verlauf/Rekorde.

CREATE TABLE workout_templates (
    id         TEXT PRIMARY KEY,
    profile_id TEXT NOT NULL REFERENCES profiles(id) ON DELETE CASCADE,
    name       TEXT NOT NULL,
    notes      TEXT,
    created_at TEXT NOT NULL
);
CREATE INDEX idx_workout_templates_profile ON workout_templates(profile_id);

-- Übung in einer Vorlage mit Ziel-Sätzen (Kraft) bzw. Ziel-Dauer (Cardio usw.).
CREATE TABLE workout_template_items (
    id           TEXT PRIMARY KEY,
    template_id  TEXT NOT NULL REFERENCES workout_templates(id) ON DELETE CASCADE,
    exercise_id  TEXT NOT NULL REFERENCES exercises(id) ON DELETE CASCADE,
    position     INTEGER NOT NULL DEFAULT 0,
    sets         INTEGER NOT NULL DEFAULT 3,
    reps         INTEGER NOT NULL DEFAULT 10,
    weight_kg    REAL NOT NULL DEFAULT 0,
    duration_min REAL NOT NULL DEFAULT 0
);
CREATE INDEX idx_workout_template_items ON workout_template_items(template_id, position);

CREATE INDEX idx_exercise_entries_exercise ON exercise_entries(profile_id, exercise_id, date);
CREATE INDEX idx_exercise_sets_entry ON exercise_sets(entry_id, position);
CREATE INDEX idx_exercises_profile ON exercises(profile_id);
"#;

const AI: &str = r#"-- Einstellungen der ganzen Installation (Schlüssel → JSON), z.B. der KI-Assistent.
CREATE TABLE app_settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);
"#;

/// Quelle für `sqlx::migrate::Migrator` aus den eingebetteten Migrationen.
#[derive(Debug)]
struct Embedded;

impl MigrationSource<'static> for Embedded {
    fn resolve(self) -> Pin<Box<dyn Future<Output = Result<Vec<Migration>, BoxDynError>> + Send + 'static>> {
        Box::pin(async {
            Ok(MIGRATIONS
                .iter()
                .map(|(v, d, sql)| Migration::new(*v, Cow::Borrowed(*d), MigrationType::Simple, Cow::Borrowed(*sql), false))
                .collect())
        })
    }
}

/// Alle Migrationen auf die Datenbank anwenden.
pub async fn run(pool: &sqlx::SqlitePool) -> Result<(), sqlx::migrate::MigrateError> {
    Migrator::new(Embedded).await?.run(pool).await
}

#[cfg(test)]
mod tests {
    use sha2::Digest;

    /// Prüfsummen der bisher ausgelieferten Migrationen (SHA-384) dürfen sich nie ändern.
    #[test]
    fn checksums_unchanged() {
        let expected = ["f6735b05a616a910afedaf60afd2a3d6958e7ac68fa97e22a4a4c843e8c47e7dabd655f7d1c613f50fc9df833d6de7f2", "049d114448b0ed4c8ac2a42f854a3f6b5c065cc7b4e6c73f50e582fd5e575dc37ef5362926a71ecb476eeffdc214ead9", "5228d1c83df56c4d5305425918579869902e7d4f160b47d8575965fc695905ddf416fb0e8f24342ef30f1364af67d1d8"];
        for ((_, _, sql), want) in super::MIGRATIONS.iter().zip(expected) {
            let got: String = sha2::Sha384::digest(sql.as_bytes()).iter().map(|b| format!("{b:02x}")).collect();
            assert_eq!(got, want);
        }
    }
}

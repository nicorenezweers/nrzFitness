-- Training: Vorlagen (Routinen) und Indizes für Verlauf/Rekorde.

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

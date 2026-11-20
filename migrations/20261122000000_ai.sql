-- Einstellungen der ganzen Installation (Schlüssel → JSON), z.B. der KI-Assistent.
CREATE TABLE app_settings (
    key        TEXT PRIMARY KEY,
    value      TEXT NOT NULL,
    updated_at TEXT NOT NULL
);

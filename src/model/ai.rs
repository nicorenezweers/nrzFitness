//! KI-Assistent: Kalorien aus Text oder Foto schätzen.

use serde::{Deserialize, Serialize};

/// Anbieter: (Schlüssel, Name, Standardmodell, Hinweis).
pub const AI_PROVIDERS: [(&str, &str, &str, &str); 6] = [
    ("anthropic", "Claude", "claude-opus-5-5", "API-Schlüssel von console.anthropic.com"),
    ("xai", "Grok", "grok-4", "API-Schlüssel von console.x.ai"),
    ("openai", "ChatGPT", "gpt-5", "API-Schlüssel von platform.openai.com"),
    ("gemini", "Gemini", "gemini-2.5-flash", "API-Schlüssel von aistudio.google.com"),
    ("custom", "Eigener Server", "", "OpenAI-kompatibel, z.B. Ollama, OpenRouter, LM Studio"),
    ("cli", "CLI-Befehl", "", "Programm auf dem Server, z.B. Claude Code"),
];

pub fn provider_label(key: &str) -> &'static str {
    AI_PROVIDERS.iter().find(|p| p.0 == key).map(|p| p.1).unwrap_or("KI")
}

pub fn provider_default_model(key: &str) -> &'static str {
    AI_PROVIDERS.iter().find(|p| p.0 == key).map(|p| p.2).unwrap_or("")
}

/// Vorlagen für den CLI-Befehl. `{image}` wird durch den Pfad zum Foto ersetzt (falls vorhanden),
/// die Anweisung kommt über stdin.
pub const CLI_PRESETS: [(&str, &str); 3] = [
    ("Claude Code", "claude -p --output-format text --allowedTools Read"),
    ("Gemini CLI", "gemini -p"),
    ("Grok CLI", "grok -p"),
];

/// Einstellungen für die Oberfläche (ohne Schlüssel im Klartext).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AiConfig {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub command: String,
    /// Ein Schlüssel ist gespeichert.
    pub has_key: bool,
    /// Letzte vier Zeichen des Schlüssels.
    pub key_hint: String,
    /// Eingerichtet und benutzbar.
    pub ready: bool,
}

/// Speichern aus der Oberfläche. `api_key: None` = gespeicherten Schlüssel behalten.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AiConfigInput {
    pub provider: String,
    pub model: String,
    pub base_url: String,
    pub command: String,
    pub api_key: Option<String>,
}

/// Eine erkannte Position mit Nährwerten für die angegebene Menge.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AiItem {
    pub name: String,
    pub amount: f64,
    pub unit: String,
    pub kcal: f64,
    pub protein: f64,
    pub carbs: f64,
    pub fat: f64,
    #[serde(default)]
    pub fiber: f64,
    #[serde(default)]
    pub sugar: f64,
}

impl AiItem {
    /// Nährwerte auf eine neue Menge umrechnen.
    pub fn scaled_to(&self, amount: f64) -> AiItem {
        let f = if self.amount > 0.0 { amount / self.amount } else { 1.0 };
        AiItem {
            name: self.name.clone(),
            amount,
            unit: self.unit.clone(),
            kcal: self.kcal * f,
            protein: self.protein * f,
            carbs: self.carbs * f,
            fat: self.fat * f,
            fiber: self.fiber * f,
            sugar: self.sugar * f,
        }
    }
}

#[derive(Debug, Clone, PartialEq, Serialize, Deserialize, Default)]
pub struct AiEstimate {
    pub title: String,
    pub items: Vec<AiItem>,
    /// "hoch" | "mittel" | "niedrig"
    pub confidence: String,
    pub note: String,
    /// Wer geschätzt hat, z.B. "Claude · claude-opus-5-5".
    #[serde(default)]
    pub by: String,
}

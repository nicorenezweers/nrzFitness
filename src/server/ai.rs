//! KI-Assistent: Nährwerte aus Beschreibung und/oder Foto schätzen.
//!
//! Anbieter: Claude (Anthropic Messages API), Grok/ChatGPT/Gemini/eigener Server (OpenAI-kompatible
//! Chat-Completions) und ein frei wählbarer CLI-Befehl (z.B. `claude -p`). Die Einstellungen liegen in
//! `app_settings` (Schlüssel `ai`); API-Schlüssel verlassen den Server nie.

use std::time::Duration;

use serde::{Deserialize, Serialize};
use serde_json::{Value, json};
use sqlx::SqlitePool;

use super::db::now_ts;
use super::{AppError, AppResult};
use crate::model::{AiConfig, AiConfigInput, AiEstimate, AiItem, provider_default_model, provider_label};

const KEY: &str = "ai";

#[derive(Debug, Clone, Default, Serialize, Deserialize)]
struct Stored {
    provider: String,
    model: String,
    base_url: String,
    command: String,
    api_key: String,
}

impl Stored {
    /// Schlüssel aus der DB, sonst aus der Umgebung (z.B. ANTHROPIC_API_KEY).
    fn key(&self) -> String {
        if !self.api_key.trim().is_empty() {
            return self.api_key.trim().to_string();
        }
        let var = match self.provider.as_str() {
            "anthropic" => "ANTHROPIC_API_KEY",
            "xai" => "XAI_API_KEY",
            "openai" => "OPENAI_API_KEY",
            "gemini" => "GEMINI_API_KEY",
            "custom" => "AI_API_KEY",
            _ => return String::new(),
        };
        std::env::var(var).unwrap_or_default().trim().to_string()
    }

    fn model(&self) -> String {
        let m = self.model.trim();
        if m.is_empty() { provider_default_model(&self.provider).to_string() } else { m.to_string() }
    }

    fn ready(&self) -> bool {
        match self.provider.as_str() {
            "cli" => !self.command.trim().is_empty(),
            "custom" => !self.base_url.trim().is_empty() && !self.model().is_empty(),
            "" => false,
            _ => !self.key().is_empty(),
        }
    }

    fn by(&self) -> String {
        match self.provider.as_str() {
            "cli" => format!("CLI · {}", self.command.split_whitespace().next().unwrap_or("")),
            p => format!("{} · {}", provider_label(p), self.model()),
        }
    }
}

async fn load(pool: &SqlitePool) -> AppResult<Stored> {
    let v: Option<String> = sqlx::query_scalar("SELECT value FROM app_settings WHERE key = ?").bind(KEY).fetch_optional(pool).await?;
    Ok(v.and_then(|s| serde_json::from_str(&s).ok()).unwrap_or_default())
}

fn public(s: &Stored) -> AiConfig {
    let key = s.key();
    AiConfig {
        provider: s.provider.clone(),
        model: s.model.clone(),
        base_url: s.base_url.clone(),
        command: s.command.clone(),
        has_key: !key.is_empty(),
        key_hint: if key.chars().count() > 8 { key.chars().rev().take(4).collect::<Vec<_>>().into_iter().rev().collect() } else { String::new() },
        ready: s.ready(),
    }
}

pub async fn config(pool: &SqlitePool) -> AppResult<AiConfig> {
    Ok(public(&load(pool).await?))
}

pub async fn save_config(pool: &SqlitePool, input: AiConfigInput) -> AppResult<AiConfig> {
    let old = load(pool).await?;
    let provider = input.provider.trim().to_string();
    if !["anthropic", "xai", "openai", "gemini", "custom", "cli", ""].contains(&provider.as_str()) {
        return Err(AppError::BadRequest("Unbekannter Anbieter".into()));
    }
    // Schlüssel beim Anbieterwechsel nicht mitnehmen.
    let keep = if old.provider == provider { old.api_key } else { String::new() };
    let s = Stored {
        provider,
        model: input.model.trim().to_string(),
        base_url: input.base_url.trim().trim_end_matches('/').to_string(),
        command: input.command.trim().to_string(),
        api_key: input.api_key.map(|k| k.trim().to_string()).unwrap_or(keep),
    };
    sqlx::query(
        "INSERT INTO app_settings (key, value, updated_at) VALUES (?, ?, ?)
         ON CONFLICT(key) DO UPDATE SET value = excluded.value, updated_at = excluded.updated_at",
    )
    .bind(KEY)
    .bind(serde_json::to_string(&s).unwrap_or_default())
    .bind(now_ts())
    .execute(pool)
    .await?;
    Ok(public(&s))
}

fn client(secs: u64) -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("nrzFitness/1.0 (self-hosted)")
        .timeout(Duration::from_secs(secs))
        .build()
        .map_err(|e| AppError::Http(e.to_string()))
}

fn openai_base(s: &Stored) -> String {
    match s.provider.as_str() {
        "xai" => "https://api.x.ai/v1".into(),
        "openai" => "https://api.openai.com/v1".into(),
        "gemini" => "https://generativelanguage.googleapis.com/v1beta/openai".into(),
        _ => s.base_url.trim_end_matches('/').to_string(),
    }
}

/// Fehlermeldung des Anbieters lesbar machen.
async fn api_error(res: reqwest::Response, who: &str) -> AppError {
    let status = res.status().as_u16();
    let body = res.text().await.unwrap_or_default();
    let msg = serde_json::from_str::<Value>(&body)
        .ok()
        .and_then(|v| {
            v.pointer("/error/message")
                .or_else(|| v.get("error").filter(|e| e.is_string()))
                .or_else(|| v.get("message"))
                .and_then(Value::as_str)
                .map(String::from)
        })
        .unwrap_or_else(|| body.chars().take(200).collect());
    let hint = match status {
        401 | 403 => " – API-Schlüssel prüfen",
        404 => " – Modellname prüfen",
        429 => " – Limit erreicht, später nochmal",
        _ => "",
    };
    AppError::Http(format!("{who}: HTTP {status} {msg}{hint}"))
}

/// Verfügbare Modelle des Anbieters (für die Auswahl in den Einstellungen).
pub async fn models(pool: &SqlitePool) -> AppResult<Vec<String>> {
    let s = load(pool).await?;
    let key = s.key();
    let c = client(20)?;
    let mut ids: Vec<String> = match s.provider.as_str() {
        "anthropic" => {
            let res = c
                .get("https://api.anthropic.com/v1/models?limit=100")
                .header("x-api-key", &key)
                .header("anthropic-version", "2023-06-01")
                .send()
                .await
                .map_err(|e| AppError::Http(e.to_string()))?;
            if !res.status().is_success() {
                return Err(api_error(res, "Claude").await);
            }
            let v: Value = res.json().await.map_err(|e| AppError::Http(e.to_string()))?;
            ids_of(&v)
        }
        "cli" | "" => return Ok(Vec::new()),
        _ => {
            let mut rb = c.get(format!("{}/models", openai_base(&s)));
            if !key.is_empty() {
                rb = rb.bearer_auth(&key);
            }
            let res = rb.send().await.map_err(|e| AppError::Http(e.to_string()))?;
            if !res.status().is_success() {
                return Err(api_error(res, provider_label(&s.provider)).await);
            }
            let v: Value = res.json().await.map_err(|e| AppError::Http(e.to_string()))?;
            ids_of(&v)
        }
    };
    ids.iter_mut().for_each(|i| *i = i.trim_start_matches("models/").to_string());
    // Nur Text/Bild-Modelle, keine Embeddings/TTS/Bildgeneratoren.
    ids.retain(|i| {
        let l = i.to_lowercase();
        !["embed", "tts", "whisper", "dall-e", "imagen", "image-gen", "moderation", "audio", "realtime", "transcribe"].iter().any(|x| l.contains(x))
    });
    ids.sort();
    ids.dedup();
    Ok(ids)
}

fn ids_of(v: &Value) -> Vec<String> {
    v.get("data")
        .or_else(|| v.get("models"))
        .and_then(Value::as_array)
        .map(|a| a.iter().filter_map(|m| m.get("id").or_else(|| m.get("name")).and_then(Value::as_str).map(String::from)).collect())
        .unwrap_or_default()
}

const SYSTEM: &str = r#"Du bist eine erfahrene Ernährungsberaterin in der Schweiz. Du schätzt aus einer Beschreibung und/oder einem Foto, was gegessen oder getrunken wird, wie viel davon, und die Nährwerte – so genau wie möglich.

Vorgehen:
- Zerlege die Mahlzeit in einzelne Positionen (z.B. Pasta, Sauce, Reibkäse, Salat, Dressing, Getränk). Fasse nicht alles zu einer Position zusammen, ausser es ist ein klar definiertes Produkt (z.B. „Big Mac“, „Snickers“).
- Schätze die Menge jeder Position in Gramm (g) bzw. Milliliter (ml) für Getränke und flüssige Suppen. Nutze Grössenhinweise im Bild (Teller ca. 26 cm, Besteck, Hand, Verpackung, Glas). Angaben des Nutzers (Menge, Marke, Zubereitung) haben Vorrang.
- Denke an versteckte Kalorien: Öl/Butter beim Braten, Saucen, Dressings, Zucker im Getränk, Panade.
- Nutze realistische Nährwerte (Schweizer bzw. europäische Produkte, z.B. Nährwerttabellen oder Packungsangaben). Bei Markenprodukten die bekannten Packungswerte.
- Werte gelten für die geschätzte Menge (nicht pro 100 g). Kalorien müssen zu den Makros passen (ca. 4 kcal/g Protein und Kohlenhydrate, 9 kcal/g Fett, 7 kcal/g Alkohol).
- Ist nichts Essbares zu erkennen, gib eine leere Liste zurück und erkläre es in "note".

Antworte ausschliesslich mit einem JSON-Objekt, ohne Text davor oder danach, in diesem Format:
{"title": "kurzer Name der Mahlzeit", "items": [{"name": "Position auf Deutsch", "amount": 180, "unit": "g", "kcal": 250, "protein": 9.5, "carbs": 45, "fat": 3.2, "fiber": 2.5, "sugar": 1.5}], "confidence": "hoch|mittel|niedrig", "note": "ein kurzer Satz zu Annahmen oder Unsicherheiten"}"#;

fn user_text(text: &str, has_image: bool, previous: Option<&AiEstimate>) -> String {
    let mut t = String::new();
    if has_image {
        t.push_str("Schätze die Mahlzeit auf dem Foto.\n");
    }
    let text = text.trim();
    if !text.is_empty() {
        t.push_str(if has_image { "Angaben des Nutzers: " } else { "Beschreibung des Nutzers: " });
        t.push_str(text);
        t.push('\n');
    }
    if let Some(p) = previous {
        let prev = json!({ "title": p.title, "items": p.items });
        t.push_str(&format!("\nDeine bisherige Schätzung war:\n{prev}\nKorrigiere sie anhand der Angaben des Nutzers und gib die vollständige neue Schätzung zurück.\n"));
    }
    t
}

/// Bild aus einer Data-URL (`data:image/jpeg;base64,...`) → (MIME, Base64).
fn split_image(data_url: &str) -> AppResult<(String, String)> {
    let rest = data_url.strip_prefix("data:").ok_or_else(|| AppError::BadRequest("Bild ungültig".into()))?;
    let (meta, b64) = rest.split_once(',').ok_or_else(|| AppError::BadRequest("Bild ungültig".into()))?;
    let mime = meta.split(';').next().unwrap_or("image/jpeg").to_string();
    if !["image/jpeg", "image/png", "image/webp", "image/gif"].contains(&mime.as_str()) {
        return Err(AppError::BadRequest("Bildformat nicht unterstützt".into()));
    }
    Ok((mime, b64.to_string()))
}

pub async fn estimate(pool: &SqlitePool, text: String, image: Option<String>, previous: Option<AiEstimate>) -> AppResult<AiEstimate> {
    let s = load(pool).await?;
    if !s.ready() {
        return Err(AppError::BadRequest("KI-Assistent ist nicht eingerichtet (Einstellungen → KI-Assistent)".into()));
    }
    if text.trim().is_empty() && image.is_none() {
        return Err(AppError::BadRequest("Beschreibe das Essen oder mache ein Foto".into()));
    }
    let img = image.as_deref().map(split_image).transpose()?;
    let prompt = user_text(&text, img.is_some(), previous.as_ref());
    let raw = match s.provider.as_str() {
        "anthropic" => anthropic(&s, &prompt, img.as_ref()).await?,
        "cli" => cli(&s, &prompt, img.as_ref()).await?,
        _ => openai(&s, &prompt, img.as_ref()).await?,
    };
    let mut est = parse(&raw)?;
    est.by = s.by();
    Ok(est)
}

/// Kurzer Verbindungstest mit den gespeicherten Einstellungen.
pub async fn test(pool: &SqlitePool) -> AppResult<AiEstimate> {
    estimate(pool, "1 Apfel (mittelgross)".into(), None, None).await
}

async fn anthropic(s: &Stored, prompt: &str, img: Option<&(String, String)>) -> AppResult<String> {
    let model = s.model();
    let mut content = Vec::new();
    if let Some((mime, data)) = img {
        content.push(json!({ "type": "image", "source": { "type": "base64", "media_type": mime, "data": data } }));
    }
    content.push(json!({ "type": "text", "text": prompt }));
    let mut body = json!({
        "model": model,
        "max_tokens": 16000,
        "system": SYSTEM,
        "messages": [{ "role": "user", "content": content }],
    });
    // Aktuelle Modelle: Aufwand explizit, bei Ablehnung serverseitig auf ein anderes Modell ausweichen.
    let current = ["claude-opus-5", "claude-sonnet-5-5", "claude-fable-5", "claude-haiku-5-5"].iter().any(|p| model.starts_with(p));
    let fallback = ["claude-opus-5", "claude-sonnet-5-5", "claude-fable-5"].iter().any(|p| model.starts_with(p));
    if current {
        body["output_config"] = json!({ "effort": "medium" });
    }
    let mut rb = client(180)?
        .post("https://api.anthropic.com/v1/messages")
        .header("x-api-key", s.key())
        .header("anthropic-version", "2023-06-01");
    if fallback {
        body["fallbacks"] = json!("default");
        rb = rb.header("anthropic-beta", "server-side-fallback-2026-07-01");
    }
    let res = rb.json(&body).send().await.map_err(|e| AppError::Http(format!("Claude nicht erreichbar ({e})")))?;
    if !res.status().is_success() {
        return Err(api_error(res, "Claude").await);
    }
    let v: Value = res.json().await.map_err(|e| AppError::Http(e.to_string()))?;
    if v.get("stop_reason").and_then(Value::as_str) == Some("refusal") {
        return Err(AppError::Http("Claude hat die Anfrage abgelehnt".into()));
    }
    let text: String = v
        .get("content")
        .and_then(Value::as_array)
        .map(|a| a.iter().filter(|b| b.get("type").and_then(Value::as_str) == Some("text")).filter_map(|b| b.get("text").and_then(Value::as_str)).collect())
        .unwrap_or_default();
    Ok(text)
}

async fn openai(s: &Stored, prompt: &str, img: Option<&(String, String)>) -> AppResult<String> {
    let who = provider_label(&s.provider);
    let mut content = vec![json!({ "type": "text", "text": prompt })];
    if let Some((mime, data)) = img {
        content.push(json!({ "type": "image_url", "image_url": { "url": format!("data:{mime};base64,{data}") } }));
    }
    let body = json!({
        "model": s.model(),
        "messages": [
            { "role": "system", "content": SYSTEM },
            { "role": "user", "content": content },
        ],
    });
    let mut rb = client(180)?.post(format!("{}/chat/completions", openai_base(s)));
    let key = s.key();
    if !key.is_empty() {
        rb = rb.bearer_auth(key);
    }
    let res = rb.json(&body).send().await.map_err(|e| AppError::Http(format!("{who} nicht erreichbar ({e})")))?;
    if !res.status().is_success() {
        return Err(api_error(res, who).await);
    }
    let v: Value = res.json().await.map_err(|e| AppError::Http(e.to_string()))?;
    let msg = v.pointer("/choices/0/message/content");
    Ok(match msg {
        Some(Value::String(t)) => t.clone(),
        Some(Value::Array(a)) => a.iter().filter_map(|p| p.get("text").and_then(Value::as_str)).collect(),
        _ => String::new(),
    })
}

/// CLI-Befehl: Anweisung über stdin, Foto als Datei (`{image}` im Befehl oder Pfad in der Anweisung).
async fn cli(s: &Stored, prompt: &str, img: Option<&(String, String)>) -> AppResult<String> {
    use base64::Engine;
    use tokio::io::AsyncWriteExt;

    let dir = std::env::temp_dir().join(format!("nrzfitness-ai-{}", uuid::Uuid::new_v4()));
    tokio::fs::create_dir_all(&dir).await.map_err(|e| AppError::Http(e.to_string()))?;
    let mut cmd_line = s.command.clone();
    let mut full = format!("{SYSTEM}\n\n{prompt}");
    if let Some((mime, data)) = img {
        let ext = mime.trim_start_matches("image/").replace("jpeg", "jpg");
        let path = dir.join(format!("foto.{ext}"));
        let bytes = base64::engine::general_purpose::STANDARD.decode(data.as_bytes()).map_err(|_| AppError::BadRequest("Bild ungültig".into()))?;
        tokio::fs::write(&path, bytes).await.map_err(|e| AppError::Http(e.to_string()))?;
        let p = path.to_string_lossy().to_string();
        if cmd_line.contains("{image}") {
            cmd_line = cmd_line.replace("{image}", &format!("'{}'", p.replace('\'', "'\\''")));
        }
        full.push_str(&format!("\nDas Foto liegt unter: {p}\nÖffne und betrachte diese Bilddatei, bevor du antwortest.\n"));
    }
    let run = async {
        let mut child = tokio::process::Command::new("sh")
            .arg("-c")
            .arg(&cmd_line)
            .current_dir(&dir)
            .stdin(std::process::Stdio::piped())
            .stdout(std::process::Stdio::piped())
            .stderr(std::process::Stdio::piped())
            .kill_on_drop(true)
            .spawn()
            .map_err(|e| AppError::Http(format!("Befehl konnte nicht gestartet werden: {e}")))?;
        if let Some(mut stdin) = child.stdin.take() {
            let _ = stdin.write_all(full.as_bytes()).await;
        }
        child.wait_with_output().await.map_err(|e| AppError::Http(e.to_string()))
    };
    let out = tokio::time::timeout(Duration::from_secs(240), run).await;
    let _ = tokio::fs::remove_dir_all(&dir).await;
    let out = out.map_err(|_| AppError::Http("CLI-Befehl hat zu lange gebraucht (4 min)".into()))??;
    let stdout = String::from_utf8_lossy(&out.stdout).to_string();
    if !out.status.success() && !stdout.contains('{') {
        let err = String::from_utf8_lossy(&out.stderr);
        let err: String = err.trim().chars().take(300).collect();
        return Err(AppError::Http(format!("CLI-Befehl fehlgeschlagen ({}): {err}", out.status)));
    }
    Ok(stdout)
}

fn num(v: Option<&Value>) -> f64 {
    match v {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.trim().replace(',', ".").split_whitespace().next().and_then(|x| x.parse().ok()).unwrap_or(0.0),
        _ => 0.0,
    }
    .max(0.0)
}

fn text_of(v: &Value, k: &str) -> String {
    v.get(k).and_then(Value::as_str).unwrap_or("").trim().to_string()
}

/// JSON aus der Antwort holen (auch mit ```-Blöcken oder Text drumherum) und bereinigen.
pub fn parse(raw: &str) -> AppResult<AiEstimate> {
    let bad = || AppError::Http("Die KI hat keine lesbare Schätzung geliefert – bitte nochmal versuchen".into());
    let start = raw.find('{').ok_or_else(bad)?;
    let end = raw.rfind('}').ok_or_else(bad)?;
    if end < start {
        return Err(bad());
    }
    let v: Value = serde_json::from_str(&raw[start..=end]).map_err(|_| bad())?;
    let items = v
        .get("items")
        .and_then(Value::as_array)
        .map(|a| {
            a.iter()
                .filter_map(|it| {
                    let name = text_of(it, "name");
                    let amount = num(it.get("amount"));
                    if name.is_empty() || amount <= 0.0 {
                        return None;
                    }
                    let unit = if text_of(it, "unit").to_lowercase().starts_with("ml") { "ml" } else { "g" };
                    let (protein, carbs, fat) = (num(it.get("protein")), num(it.get("carbs")), num(it.get("fat")));
                    let mut kcal = num(it.get("kcal"));
                    if kcal <= 0.0 {
                        kcal = protein * 4.0 + carbs * 4.0 + fat * 9.0;
                    }
                    Some(AiItem {
                        name,
                        amount,
                        unit: unit.into(),
                        kcal,
                        protein,
                        carbs,
                        fat,
                        fiber: num(it.get("fiber")),
                        sugar: num(it.get("sugar")),
                    })
                })
                .collect::<Vec<_>>()
        })
        .unwrap_or_default();
    let confidence = match text_of(&v, "confidence").to_lowercase().as_str() {
        c if c.starts_with('h') => "hoch",
        c if c.starts_with('n') || c.starts_with('l') => "niedrig",
        _ => "mittel",
    };
    Ok(AiEstimate { title: text_of(&v, "title"), items, confidence: confidence.into(), note: text_of(&v, "note"), by: String::new() })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_fenced_json() {
        let raw = "Hier:\n```json\n{\"title\":\"Pasta\",\"items\":[{\"name\":\"Spaghetti\",\"amount\":\"250 g\",\"unit\":\"g\",\"kcal\":395,\"protein\":14,\"carbs\":77,\"fat\":2.3},{\"name\":\"\",\"amount\":10}],\"confidence\":\"High\",\"note\":\"x\"}\n```";
        let e = parse(raw).unwrap();
        assert_eq!(e.items.len(), 1);
        assert_eq!(e.items[0].amount, 250.0);
        assert_eq!(e.confidence, "hoch");
    }

    #[test]
    fn computes_missing_kcal() {
        let e = parse(r#"{"items":[{"name":"Ei","amount":60,"unit":"g","protein":7.5,"carbs":0.5,"fat":5.7}]}"#).unwrap();
        assert!((e.items[0].kcal - (7.5 * 4.0 + 0.5 * 4.0 + 5.7 * 9.0)).abs() < 0.01);
    }

    #[test]
    fn rejects_garbage() {
        assert!(parse("keine Ahnung").is_err());
    }
}

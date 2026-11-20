//! Open Food Facts: Produktsuche und Barcode-Abfrage (öffentliche API, ohne Konto).

use std::time::Duration;

use serde_json::Value;

use super::{AppError, AppResult};
use crate::model::FoodInput;

const FIELDS: &str = "code,product_name,product_name_de,generic_name,brands,nutriments,serving_quantity,serving_size,quantity,image_front_small_url,image_small_url";

fn client() -> AppResult<reqwest::Client> {
    reqwest::Client::builder()
        .user_agent("nrzFitness/1.0 (self-hosted)")
        .timeout(Duration::from_secs(12))
        .build()
        .map_err(|e| AppError::Http(e.to_string()))
}

fn base() -> String {
    std::env::var("OFF_URL").unwrap_or_else(|_| "https://world.openfoodfacts.org".into())
}

fn f(v: &Value, key: &str) -> f64 {
    match v.get(key) {
        Some(Value::Number(n)) => n.as_f64().unwrap_or(0.0),
        Some(Value::String(s)) => s.replace(',', ".").parse().unwrap_or(0.0),
        _ => 0.0,
    }
}

fn s(v: &Value, key: &str) -> Option<String> {
    v.get(key).and_then(Value::as_str).map(str::trim).filter(|x| !x.is_empty()).map(String::from)
}

/// OFF-Produkt → FoodInput (pro 100 g/ml). `None`, wenn Name oder Energie fehlen.
fn to_food(p: &Value) -> Option<FoodInput> {
    let name = s(p, "product_name_de").or_else(|| s(p, "product_name")).or_else(|| s(p, "generic_name"))?;
    let n = p.get("nutriments")?;
    let mut kcal = f(n, "energy-kcal_100g");
    if kcal <= 0.0 {
        let kj = f(n, "energy_100g");
        if kj > 0.0 {
            kcal = kj / 4.184;
        }
    }
    let protein = f(n, "proteins_100g");
    let carbs = f(n, "carbohydrates_100g");
    let fat = f(n, "fat_100g");
    if kcal <= 0.0 && protein + carbs + fat <= 0.0 {
        return None;
    }
    let quantity = s(p, "quantity").unwrap_or_default().to_lowercase();
    let serving_txt = s(p, "serving_size").unwrap_or_default().to_lowercase();
    let liquid = |t: &str| t.contains("ml") || t.contains("cl") || t.ends_with(" l") || t.ends_with("0l");
    let unit = if liquid(&quantity) || liquid(&serving_txt) { "ml" } else { "g" };
    let serving = f(p, "serving_quantity");
    // `brands`: Text "A, B" (klassische API) oder Liste (Suche).
    let brand = match p.get("brands") {
        Some(Value::Array(a)) => a.first().and_then(Value::as_str).map(String::from),
        _ => s(p, "brands").map(|b| b.split(',').next().unwrap_or("").to_string()),
    }
    .map(|b| b.trim().to_string())
    .filter(|b| !b.is_empty());
    Some(FoodInput {
        name,
        brand,
        barcode: s(p, "code"),
        unit: unit.into(),
        serving_size: if serving > 0.0 { serving } else { 100.0 },
        serving_name: if serving > 0.0 { s(p, "serving_size") } else { None },
        kcal: (kcal * 10.0).round() / 10.0,
        protein,
        carbs,
        fat,
        fiber: f(n, "fiber_100g"),
        sugar: f(n, "sugars_100g"),
        sat_fat: f(n, "saturated-fat_100g"),
        sodium_mg: f(n, "sodium_100g") * 1000.0,
        source: "off".into(),
        image_url: s(p, "image_front_small_url").or_else(|| s(p, "image_small_url")),
        shared: true,
    })
}

/// Kurzzeit-Cache für Suchanfragen (OFF begrenzt die Anfragen pro Minute).
fn cache() -> &'static std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Vec<FoodInput>)>> {
    static C: std::sync::OnceLock<std::sync::Mutex<std::collections::HashMap<String, (std::time::Instant, Vec<FoodInput>)>>> =
        std::sync::OnceLock::new();
    C.get_or_init(Default::default)
}

pub async fn search(q: &str) -> AppResult<Vec<FoodInput>> {
    let q = q.trim();
    if q.chars().count() < 2 {
        return Ok(Vec::new());
    }
    let key = q.to_lowercase();
    if let Ok(c) = cache().lock()
        && let Some((t, v)) = c.get(&key)
        && t.elapsed() < Duration::from_secs(900)
    {
        return Ok(v.clone());
    }
    // Neue Such-API zuerst, klassische als Ausweichlösung.
    let mut result = search_new(q).await;
    if result.as_ref().map(|v| v.is_empty()).unwrap_or(true) {
        if let Ok(v) = search_classic(q).await {
            result = Ok(v);
        }
    }
    let mut out = result?;
    out.dedup_by(|a, b| a.name == b.name && a.brand == b.brand);
    if let Ok(mut c) = cache().lock() {
        if c.len() > 500 {
            c.clear();
        }
        c.insert(key, (std::time::Instant::now(), out.clone()));
    }
    Ok(out)
}

fn unreachable(e: impl std::fmt::Display) -> AppError {
    AppError::Http(format!("Open Food Facts nicht erreichbar ({e})"))
}

async fn search_new(q: &str) -> AppResult<Vec<FoodInput>> {
    let url = std::env::var("OFF_SEARCH_URL").unwrap_or_else(|_| "https://search.openfoodfacts.org".into());
    let res = client()?
        .get(format!("{url}/search"))
        .query(&[("q", q), ("page_size", "30"), ("langs", "de,en,fr"), ("fields", FIELDS)])
        .send()
        .await
        .map_err(unreachable)?;
    if !res.status().is_success() {
        return Err(AppError::Http(format!("Open Food Facts ist gerade überlastet (HTTP {})", res.status().as_u16())));
    }
    let v: Value = res.json().await.map_err(unreachable)?;
    Ok(v.get("hits").and_then(Value::as_array).map(|a| a.iter().filter_map(to_food).collect()).unwrap_or_default())
}

async fn search_classic(q: &str) -> AppResult<Vec<FoodInput>> {
    let res = client()?
        .get(format!("{}/cgi/search.pl", base()))
        .query(&[
            ("search_terms", q),
            ("search_simple", "1"),
            ("action", "process"),
            ("json", "1"),
            ("page_size", "25"),
            ("fields", FIELDS),
        ])
        .send()
        .await
        .map_err(unreachable)?;
    if !res.status().is_success() {
        return Err(AppError::Http(format!("Open Food Facts ist gerade überlastet (HTTP {})", res.status().as_u16())));
    }
    let v: Value = res.json().await.map_err(unreachable)?;
    Ok(v.get("products").and_then(Value::as_array).map(|a| a.iter().filter_map(to_food).collect()).unwrap_or_default())
}

pub async fn barcode(code: &str) -> AppResult<Option<FoodInput>> {
    let code: String = code.chars().filter(char::is_ascii_digit).collect();
    if code.len() < 6 {
        return Err(AppError::BadRequest("Ungültiger Barcode".into()));
    }
    let url = format!("{}/api/v2/product/{code}.json?fields={FIELDS}", base());
    let res = client()?
        .get(url)
        .send()
        .await
        .map_err(unreachable)?;
    if res.status() == reqwest::StatusCode::NOT_FOUND {
        return Ok(None);
    }
    let v: Value = res.json().await.map_err(|e| AppError::Http(e.to_string()))?;
    if v.get("status").and_then(Value::as_i64) != Some(1) {
        return Ok(None);
    }
    Ok(v.get("product").and_then(to_food).map(|mut f| {
        f.barcode = Some(code);
        f
    }))
}

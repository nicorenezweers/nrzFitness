use chrono::NaiveDate;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{AiConfig, AiConfigInput, AiEstimate, AiItem};
#[cfg(feature = "server")]
use crate::model::FoodInput;
#[cfg(feature = "server")]
use crate::server::{ai, db, pool};

#[server]
pub async fn get_ai_config() -> Result<AiConfig, ServerFnError> {
    ai::config(pool()).await.map_err(e)
}

#[server]
pub async fn save_ai_config(input: AiConfigInput) -> Result<AiConfig, ServerFnError> {
    ai::save_config(pool(), input).await.map_err(e)
}

#[server]
pub async fn ai_models() -> Result<Vec<String>, ServerFnError> {
    ai::models(pool()).await.map_err(e)
}

#[server]
pub async fn ai_test() -> Result<AiEstimate, ServerFnError> {
    ai::test(pool()).await.map_err(e)
}

/// Nährwerte schätzen. `image`: Data-URL (JPEG, verkleinert im Browser); `previous` zum Korrigieren.
#[server]
pub async fn ai_estimate(text: String, image: Option<String>, previous: Option<AiEstimate>) -> Result<AiEstimate, ServerFnError> {
    ai::estimate(pool(), text, image, previous).await.map_err(e)
}

/// Geschätzte Positionen als Lebensmittel (Quelle „ai“, nur dieses Profil) anlegen und eintragen.
#[server]
pub async fn ai_add_items(pid: String, date: NaiveDate, meal: String, items: Vec<AiItem>) -> Result<usize, ServerFnError> {
    let mut n = 0;
    for it in items.into_iter().filter(|i| i.amount > 0.0 && !i.name.trim().is_empty()) {
        let per = 100.0 / it.amount;
        let input = FoodInput {
            name: it.name.trim().to_string(),
            unit: if it.unit == "ml" { "ml".into() } else { "g".into() },
            serving_size: (it.amount * 10.0).round() / 10.0,
            kcal: it.kcal * per,
            protein: it.protein * per,
            carbs: it.carbs * per,
            fat: it.fat * per,
            fiber: it.fiber * per,
            sugar: it.sugar * per,
            source: "ai".into(),
            shared: false,
            ..Default::default()
        };
        let food = db::foods::create(pool(), &pid, input).await.map_err(e)?;
        db::diary::add_food(pool(), &pid, date, &meal, &food.id, it.amount).await.map_err(e)?;
        n += 1;
    }
    Ok(n)
}

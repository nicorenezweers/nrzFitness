use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{Food, FoodInput, FoodSearch, MealTemplate};
#[cfg(feature = "server")]
use crate::server::{db, off, pool};

/// Eigene Datenbank durchsuchen; mit `online` zusätzlich Open Food Facts.
#[server]
pub async fn search_foods(pid: String, q: String, online: bool) -> Result<FoodSearch, ServerFnError> {
    let local = db::foods::search(pool(), &pid, &q, 40).await.map_err(e)?;
    let online = if online {
        let known: Vec<String> = local.iter().filter_map(|f| f.barcode.clone()).collect();
        off::search(&q).await.map_err(e)?.into_iter().filter(|f| f.barcode.as_ref().is_none_or(|b| !known.contains(b))).collect()
    } else {
        Vec::new()
    };
    Ok(FoodSearch { local, online })
}

/// Barcode: zuerst lokal, sonst Open Food Facts (wird dann gespeichert).
#[server]
pub async fn lookup_barcode(pid: String, code: String) -> Result<Option<Food>, ServerFnError> {
    if let Some(f) = db::foods::by_barcode(pool(), &pid, &code).await.map_err(e)? {
        return Ok(Some(f));
    }
    match off::barcode(&code).await.map_err(e)? {
        Some(input) => Ok(Some(db::foods::create(pool(), &pid, input).await.map_err(e)?)),
        None => Ok(None),
    }
}

#[server]
pub async fn recent_foods(pid: String) -> Result<Vec<Food>, ServerFnError> {
    db::foods::recent(pool(), &pid, 30).await.map_err(e)
}

#[server]
pub async fn favorite_foods(pid: String) -> Result<Vec<Food>, ServerFnError> {
    db::foods::favorites(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn all_foods(pid: String) -> Result<Vec<Food>, ServerFnError> {
    db::foods::list_all(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn get_food(pid: String, id: String) -> Result<Food, ServerFnError> {
    db::foods::get(pool(), &pid, &id).await.map_err(e)
}

#[server]
pub async fn last_amount(pid: String, food_id: String) -> Result<Option<f64>, ServerFnError> {
    db::foods::last_amount(pool(), &pid, &food_id).await.map_err(e)
}

#[server]
pub async fn create_food(pid: String, input: FoodInput) -> Result<Food, ServerFnError> {
    db::foods::create(pool(), &pid, input).await.map_err(e)
}

#[server]
pub async fn update_food(pid: String, id: String, input: FoodInput) -> Result<Food, ServerFnError> {
    db::foods::update(pool(), &pid, &id, input).await.map_err(e)
}

#[server]
pub async fn delete_food(pid: String, id: String) -> Result<(), ServerFnError> {
    db::foods::delete(pool(), &pid, &id).await.map_err(e)
}

#[server]
pub async fn set_favorite(pid: String, id: String, fav: bool) -> Result<(), ServerFnError> {
    db::foods::set_favorite(pool(), &pid, &id, fav).await.map_err(e)
}

#[server]
pub async fn list_meals(pid: String) -> Result<Vec<MealTemplate>, ServerFnError> {
    db::foods::meals(pool(), &pid).await.map_err(e)
}

#[server]
pub async fn save_meal(
    pid: String,
    id: Option<String>,
    name: String,
    notes: Option<String>,
    items: Vec<(String, f64)>,
) -> Result<MealTemplate, ServerFnError> {
    db::foods::save_meal(pool(), &pid, id, name, notes, items).await.map_err(e)
}

#[server]
pub async fn delete_meal(pid: String, id: String) -> Result<(), ServerFnError> {
    db::foods::delete_meal(pool(), &pid, &id).await.map_err(e)
}

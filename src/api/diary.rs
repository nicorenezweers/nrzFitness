use chrono::NaiveDate;
use dioxus::prelude::*;

#[cfg(feature = "server")]
use super::e;
use crate::model::{DayData, FoodEntry, Nutrients};
#[cfg(feature = "server")]
use crate::server::{db, pool};

#[server]
pub async fn get_day(pid: String, date: NaiveDate) -> Result<DayData, ServerFnError> {
    db::diary::day(pool(), &pid, date).await.map_err(e)
}

#[server]
pub async fn add_food_entry(pid: String, date: NaiveDate, meal: String, food_id: String, amount: f64) -> Result<FoodEntry, ServerFnError> {
    db::diary::add_food(pool(), &pid, date, &meal, &food_id, amount).await.map_err(e)
}

#[server]
pub async fn add_quick_entry(pid: String, date: NaiveDate, meal: String, name: String, n: Nutrients) -> Result<FoodEntry, ServerFnError> {
    db::diary::add_quick(pool(), &pid, date, &meal, name, n).await.map_err(e)
}

#[server]
pub async fn add_meal_entries(pid: String, date: NaiveDate, meal: String, meal_id: String, factor: f64) -> Result<usize, ServerFnError> {
    db::diary::add_meal(pool(), &pid, date, &meal, &meal_id, factor).await.map_err(e)
}

#[server]
pub async fn update_food_entry(pid: String, id: String, meal: String, amount: f64) -> Result<FoodEntry, ServerFnError> {
    db::diary::update_entry(pool(), &pid, &id, &meal, amount).await.map_err(e)
}

#[server]
pub async fn delete_food_entry(pid: String, id: String) -> Result<(), ServerFnError> {
    db::diary::delete_entry(pool(), &pid, &id).await.map_err(e)
}

#[server]
pub async fn copy_day(pid: String, from: NaiveDate, to: NaiveDate, meal: Option<String>) -> Result<u64, ServerFnError> {
    db::diary::copy_day(pool(), &pid, from, to, meal).await.map_err(e)
}

#[server]
pub async fn add_water(pid: String, date: NaiveDate, ml: f64) -> Result<(), ServerFnError> {
    db::diary::add_water(pool(), &pid, date, ml).await.map_err(e)
}
